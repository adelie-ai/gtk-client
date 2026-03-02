use std::future::Future;
use std::sync::OnceLock;

use desktop_assistant_client_common::SignalEvent;
use gtk4::glib;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
    })
}

/// Messages sent from async tasks back to the GTK main thread.
#[derive(Debug)]
pub enum UiMessage {
    ConversationsLoaded(Vec<desktop_assistant_client_common::ConversationSummary>),
    ConversationLoaded(desktop_assistant_client_common::ConversationDetail),
    ConversationCreated {
        id: String,
    },
    ConversationDeleted {
        id: String,
    },
    ConversationRenamed {
        id: String,
        title: String,
    },
    StreamChunk {
        request_id: String,
        chunk: String,
    },
    StreamComplete {
        request_id: String,
        full_response: String,
    },
    StreamError {
        request_id: String,
        error: String,
    },
    TitleChanged {
        conversation_id: String,
        title: String,
    },
    PromptSent {
        request_id: String,
    },
    StatusUpdate(String),
    Error(String),
}

/// Bridge between the GTK main loop and tokio async tasks.
///
/// Uses a tokio mpsc channel + `glib::spawn_future_local` to dispatch
/// messages onto the GTK main thread.
pub struct AsyncBridge {
    ui_tx: mpsc::UnboundedSender<UiMessage>,
}

impl AsyncBridge {
    /// Create a new bridge. `handler` is called on the GTK main thread for each UiMessage.
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(UiMessage) + 'static,
    {
        let (ui_tx, mut ui_rx) = mpsc::unbounded_channel::<UiMessage>();

        // Spawn a local future on the GLib main context to receive messages
        glib::spawn_future_local(async move {
            while let Some(msg) = ui_rx.recv().await {
                handler(msg);
            }
        });

        Self { ui_tx }
    }

    /// Spawn an async task on the tokio runtime.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        runtime().spawn(future);
    }

    /// Get a clone of the UI sender for passing into async tasks.
    pub fn ui_sender(&self) -> mpsc::UnboundedSender<UiMessage> {
        self.ui_tx.clone()
    }

    /// Start forwarding SignalEvents from the transport to UiMessages on the GTK thread.
    pub fn forward_signals(&self, mut signal_rx: mpsc::UnboundedReceiver<SignalEvent>) {
        let tx = self.ui_tx.clone();
        runtime().spawn(async move {
            while let Some(signal) = signal_rx.recv().await {
                let msg = match signal {
                    SignalEvent::Chunk { request_id, chunk } => {
                        UiMessage::StreamChunk { request_id, chunk }
                    }
                    SignalEvent::Complete {
                        request_id,
                        full_response,
                    } => UiMessage::StreamComplete {
                        request_id,
                        full_response,
                    },
                    SignalEvent::Error { request_id, error } => {
                        UiMessage::StreamError { request_id, error }
                    }
                    SignalEvent::TitleChanged {
                        conversation_id,
                        title,
                    } => UiMessage::TitleChanged {
                        conversation_id,
                        title,
                    },
                };
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });
    }
}
