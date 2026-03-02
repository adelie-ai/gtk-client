use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use desktop_assistant_client_common::{
    AssistantClient, ChatMessage, ConnectionConfig, ConversationDetail, ConversationSummary,
    TransportClient, connect_transport, transport::transport_label,
};
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, CheckButton, Label, Orientation, Separator, gdk,
    glib,
};
use tokio::sync::mpsc;

use crate::async_bridge::{AsyncBridge, UiMessage};
use crate::widgets::chat_view::ChatView;
use crate::widgets::input_bar::InputBar;
use crate::widgets::sidebar::Sidebar;

/// Shared mutable state for the window.
struct WindowState {
    conversations: Vec<ConversationSummary>,
    current_conversation_id: Option<String>,
    current_conversation: Option<ConversationDetail>,
    pending_request_id: Option<String>,
    streaming_buffer: String,
    debug_enabled: bool,
}

/// Internal message for bootstrapping the transport client on the main thread.
enum InternalMsg {
    TransportReady {
        client: Arc<TransportClient>,
        signal_rx: mpsc::UnboundedReceiver<desktop_assistant_client_common::SignalEvent>,
    },
}

pub struct AdelieWindow {
    pub window: ApplicationWindow,
}

impl AdelieWindow {
    pub fn new(app: &Application, config: ConnectionConfig) -> Self {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("Adelie Desktop Assistant")
            .default_width(1100)
            .default_height(700)
            .build();

        // Apply CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("style.css"));
        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Layout
        let main_box = GtkBox::new(Orientation::Horizontal, 0);

        let sidebar = Sidebar::new();
        main_box.append(&sidebar.container);

        let separator = Separator::new(Orientation::Vertical);
        main_box.append(&separator);

        let right_box = GtkBox::new(Orientation::Vertical, 0);
        right_box.set_hexpand(true);
        right_box.set_vexpand(true);

        let chat_view = ChatView::new();
        right_box.append(&chat_view.container);

        let input_sep = Separator::new(Orientation::Horizontal);
        right_box.append(&input_sep);

        let input_bar = InputBar::new();
        right_box.append(&input_bar.container);

        let status_bar = GtkBox::new(Orientation::Horizontal, 0);
        status_bar.set_margin_top(4);
        status_bar.set_margin_bottom(4);

        let status_label = Label::new(Some("Connecting..."));
        status_label.set_halign(gtk4::Align::Start);
        status_label.set_hexpand(true);
        status_label.set_margin_start(12);
        status_label.add_css_class("status-bar");
        status_bar.append(&status_label);

        let debug_check = CheckButton::with_label("Debug");
        debug_check.set_halign(gtk4::Align::End);
        debug_check.set_margin_end(12);
        debug_check.add_css_class("debug-check");
        status_bar.append(&debug_check);

        right_box.append(&status_bar);

        main_box.append(&right_box);
        window.set_child(Some(&main_box));

        // Shared state
        let state = Rc::new(RefCell::new(WindowState {
            conversations: Vec::new(),
            current_conversation_id: None,
            current_conversation: None,
            pending_request_id: None,
            streaming_buffer: String::new(),
            debug_enabled: false,
        }));

        // Wrap widgets in Rc for closures
        let sidebar = Rc::new(sidebar);
        let chat_view = Rc::new(RefCell::new(chat_view));
        let input_bar = Rc::new(input_bar);
        let status_label = Rc::new(status_label);

        // Client wrapped in Arc for async tasks, Rc<RefCell<>> for GTK thread
        let client: Rc<RefCell<Option<Arc<TransportClient>>>> = Rc::new(RefCell::new(None));

        // Internal channel for transport bootstrap (sends non-Send types via main thread)
        let (internal_tx, mut internal_rx) = mpsc::unbounded_channel::<InternalMsg>();

        // Set up async bridge with UI message handler
        let bridge = {
            let state = Rc::clone(&state);
            let sidebar = Rc::clone(&sidebar);
            let chat_view = Rc::clone(&chat_view);
            let status_label = Rc::clone(&status_label);

            AsyncBridge::new(move |msg| {
                handle_ui_message(msg, &state, &sidebar, &chat_view, &status_label);
            })
        };
        let bridge = Rc::new(bridge);

        // Spawn a local future to receive the transport client on the main thread
        {
            let client_ref = Rc::clone(&client);
            let bridge_ref = Rc::clone(&bridge);
            glib::spawn_future_local(async move {
                while let Some(msg) = internal_rx.recv().await {
                    match msg {
                        InternalMsg::TransportReady {
                            client: transport,
                            signal_rx,
                        } => {
                            *client_ref.borrow_mut() = Some(transport);
                            bridge_ref.forward_signals(signal_rx);
                        }
                    }
                }
            });
        }

        // Connect transport on startup
        {
            let tx = bridge.ui_sender();
            let config = config.clone();
            bridge.spawn(async move {
                match connect_transport(&config).await {
                    Ok((transport, signal_rx)) => {
                        let transport = Arc::new(transport);

                        let _ = tx.send(UiMessage::StatusUpdate(
                            transport_label(config.transport_mode).to_string(),
                        ));

                        // Send client + signal_rx to GTK thread via internal channel
                        let _ = internal_tx.send(InternalMsg::TransportReady {
                            client: Arc::clone(&transport),
                            signal_rx,
                        });

                        // List conversations
                        match transport.list_conversations().await {
                            Ok(convs) => {
                                let _ = tx.send(UiMessage::ConversationsLoaded(convs));
                            }
                            Err(e) => {
                                let _ =
                                    tx.send(UiMessage::Error(format!("Load conversations: {e}")));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(UiMessage::Error(format!("Connection failed: {e}")));
                    }
                }
            });
        }

        // Sidebar row activation → load conversation
        {
            let client_ref = Rc::clone(&client);
            let state = Rc::clone(&state);
            let bridge = Rc::clone(&bridge);
            sidebar.list_box.connect_row_activated(move |_, row| {
                let idx = row.index() as usize;
                let state_borrow = state.borrow();
                if let Some(conv) = state_borrow.conversations.get(idx) {
                    let conv_id = conv.id.clone();
                    drop(state_borrow);

                    if let Some(client) = client_ref.borrow().clone() {
                        let tx = bridge.ui_sender();
                        bridge.spawn(async move {
                            match client.get_conversation(&conv_id).await {
                                Ok(detail) => {
                                    let _ = tx.send(UiMessage::ConversationLoaded(detail));
                                }
                                Err(e) => {
                                    let _ = tx.send(UiMessage::Error(format!(
                                        "Load conversation: {e}"
                                    )));
                                }
                            }
                        });
                    }
                }
            });
        }

        // New conversation button
        {
            let client_ref = Rc::clone(&client);
            let bridge = Rc::clone(&bridge);
            sidebar.new_button.connect_clicked(move |_| {
                if let Some(client) = client_ref.borrow().clone() {
                    let tx = bridge.ui_sender();
                    bridge.spawn(async move {
                        match client.create_conversation("New Conversation").await {
                            Ok(id) => {
                                let _ = tx.send(UiMessage::ConversationCreated { id: id.clone() });
                                // Refresh conversation list
                                if let Ok(convs) = client.list_conversations().await {
                                    let _ = tx.send(UiMessage::ConversationsLoaded(convs));
                                }
                                // Load the new conversation
                                if let Ok(detail) = client.get_conversation(&id).await {
                                    let _ = tx.send(UiMessage::ConversationLoaded(detail));
                                }
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(UiMessage::Error(format!("Create conversation: {e}")));
                            }
                        }
                    });
                }
            });
        }

        // Send button / Enter key → send prompt
        {
            let client_ref = Rc::clone(&client);
            let bridge_ref = Rc::clone(&bridge);
            let state = Rc::clone(&state);
            let input_bar_ref = Rc::clone(&input_bar);
            let chat_view_ref = Rc::clone(&chat_view);

            let send_action = Rc::new(move || {
                let text = input_bar_ref.take_text();
                let text = text.trim().to_string();
                if text.is_empty() {
                    return;
                }
                let state_borrow = state.borrow();
                let conv_id = match &state_borrow.current_conversation_id {
                    Some(id) => id.clone(),
                    None => return,
                };
                drop(state_borrow);

                // Show user message immediately
                chat_view_ref.borrow_mut().add_user_message(&text);

                // Track in local conversation copy
                {
                    let mut s = state.borrow_mut();
                    if let Some(ref mut conv) = s.current_conversation {
                        conv.messages.push(ChatMessage {
                            role: "user".to_string(),
                            content: text.clone(),
                        });
                    }
                }

                if let Some(client) = client_ref.borrow().clone() {
                    let tx = bridge_ref.ui_sender();
                    let text = text.clone();
                    bridge_ref.spawn(async move {
                        match client.send_prompt(&conv_id, &text).await {
                            Ok(request_id) => {
                                let _ = tx.send(UiMessage::PromptSent { request_id });
                            }
                            Err(e) => {
                                let _ = tx.send(UiMessage::Error(format!("Send error: {e}")));
                            }
                        }
                    });
                }
            });

            // Send button click
            let send_action_click = Rc::clone(&send_action);
            input_bar.send_button.connect_clicked(move |_| {
                send_action_click();
            });

            // Enter key in text view (Shift+Enter for newline)
            let send_action_key = Rc::clone(&send_action);
            let key_controller = gtk4::EventControllerKey::new();
            key_controller.connect_key_pressed(move |_, key, _, modifiers| {
                if key == gdk::Key::Return
                    && !modifiers.contains(gdk::ModifierType::SHIFT_MASK)
                    && !modifiers.contains(gdk::ModifierType::CONTROL_MASK)
                {
                    send_action_key();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            input_bar.text_view.add_controller(key_controller);
        }

        // Debug checkbox toggle → re-fetch conversation with filtering
        {
            let client_ref = Rc::clone(&client);
            let bridge_ref = Rc::clone(&bridge);
            let state = Rc::clone(&state);
            debug_check.connect_toggled(move |btn| {
                state.borrow_mut().debug_enabled = btn.is_active();
                let conv_id = state.borrow().current_conversation_id.clone();
                if let Some(conv_id) = conv_id {
                    if let Some(client) = client_ref.borrow().clone() {
                        let tx = bridge_ref.ui_sender();
                        bridge_ref.spawn(async move {
                            match client.get_conversation(&conv_id).await {
                                Ok(detail) => {
                                    let _ = tx.send(UiMessage::ConversationLoaded(detail));
                                }
                                Err(e) => {
                                    let _ = tx.send(UiMessage::Error(format!(
                                        "Reload conversation: {e}"
                                    )));
                                }
                            }
                        });
                    }
                }
            });
        }

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn handle_ui_message(
    msg: UiMessage,
    state: &Rc<RefCell<WindowState>>,
    sidebar: &Rc<Sidebar>,
    chat_view: &Rc<RefCell<ChatView>>,
    status_label: &Rc<Label>,
) {
    match msg {
        UiMessage::ConversationsLoaded(convs) => {
            sidebar.set_conversations(&convs);
            state.borrow_mut().conversations = convs;
        }
        UiMessage::ConversationLoaded(detail) => {
            let id = detail.id.clone();
            let debug = state.borrow().debug_enabled;
            let filtered = filter_messages(&detail, debug);
            let mut s = state.borrow_mut();
            s.current_conversation = Some(detail);
            s.current_conversation_id = Some(id);
            drop(s);
            chat_view.borrow_mut().load_conversation(&filtered);
        }
        UiMessage::ConversationCreated { id } => {
            state.borrow_mut().current_conversation_id = Some(id);
        }
        UiMessage::PromptSent { request_id } => {
            let mut s = state.borrow_mut();
            if request_id.is_empty() {
                // WS doesn't return request ID upfront; use a sentinel.
                s.pending_request_id = Some("__pending__".to_string());
            } else {
                s.pending_request_id = Some(request_id);
            }
            s.streaming_buffer.clear();
        }
        UiMessage::StreamChunk { request_id, chunk } => {
            let mut s = state.borrow_mut();
            // Claim request ID if pending
            if s.pending_request_id.as_deref() == Some("__pending__") {
                s.pending_request_id = Some(request_id.clone());
            }
            if s.pending_request_id.as_deref() == Some(&request_id) {
                s.streaming_buffer.push_str(&chunk);
                drop(s);
                chat_view.borrow_mut().receive_chunk(&chunk);
            }
        }
        UiMessage::StreamComplete {
            request_id,
            full_response,
        } => {
            let mut s = state.borrow_mut();
            if s.pending_request_id.as_deref() == Some("__pending__") {
                s.pending_request_id = Some(request_id.clone());
            }
            if s.pending_request_id.as_deref() == Some(&request_id) {
                s.pending_request_id = None;
                s.streaming_buffer.clear();
                if let Some(ref mut conv) = s.current_conversation {
                    conv.messages.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: full_response.clone(),
                    });
                }
                drop(s);
                chat_view.borrow_mut().complete_streaming(&full_response);
            }
        }
        UiMessage::StreamError { request_id, error } => {
            let mut s = state.borrow_mut();
            if s.pending_request_id.as_deref() == Some("__pending__") {
                s.pending_request_id = Some(request_id.clone());
            }
            if s.pending_request_id.as_deref() == Some(&request_id) {
                s.pending_request_id = None;
                s.streaming_buffer.clear();
                drop(s);
                status_label.set_text(&format!("Error: {error}"));
            }
        }
        UiMessage::TitleChanged {
            conversation_id,
            title,
        } => {
            let mut s = state.borrow_mut();
            for conv in &mut s.conversations {
                if conv.id == conversation_id {
                    conv.title = title.clone();
                }
            }
            let convs = s.conversations.clone();
            drop(s);
            sidebar.set_conversations(&convs);
        }
        UiMessage::StatusUpdate(text) => {
            status_label.set_text(&text);
        }
        UiMessage::Error(text) => {
            status_label.set_text(&format!("Error: {text}"));
        }
    }
}

/// Filter a conversation's messages based on debug mode.
///
/// When debug is off, only user and assistant messages are shown.
/// When debug is on, tool messages are included as well.
fn filter_messages(detail: &ConversationDetail, debug: bool) -> ConversationDetail {
    ConversationDetail {
        id: detail.id.clone(),
        title: detail.title.clone(),
        messages: detail
            .messages
            .iter()
            .filter(|m| {
                if debug {
                    return true;
                }
                match m.role.as_str() {
                    "user" => true,
                    // Hide empty assistant messages (tool_calls-only)
                    "assistant" => !m.content.trim().is_empty(),
                    _ => false,
                }
            })
            .cloned()
            .collect(),
    }
}
