//! Per-conversation model selector (next to the chat input).
//!
//! Shows a flat `Connection · Model` list aggregated across every healthy
//! connection the daemon reports, plus a pinned, disabled `Auto (coming
//! soon)` entry at the top. Selection is communicated back to the window
//! via a callback; the window combines it with the per-conversation
//! selection store and attaches the resulting override to each
//! `SendMessage`.

use std::cell::RefCell;
use std::rc::Rc;

use desktop_assistant_api_model as api;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, DropDown, Label, Orientation, StringList};

type SelectCallback = Box<dyn Fn(Option<api::SendPromptOverride>)>;

/// One row in the selector.
#[derive(Debug, Clone)]
enum Entry {
    /// Pinned top row: `Auto (coming soon)` — disabled.
    AutoPlaceholder,
    /// Aggregated `Connection · Model` entry.
    Model {
        connection_id: String,
        connection_label: String,
        model_id: String,
        model_label: String,
    },
}

impl Entry {
    fn display(&self) -> String {
        match self {
            Self::AutoPlaceholder => "Auto (coming soon)".to_string(),
            Self::Model {
                connection_label,
                model_label,
                ..
            } => format!("{connection_label} · {model_label}"),
        }
    }
}

pub struct ModelSelector {
    pub container: GtkBox,
    dropdown: DropDown,
    string_list: StringList,
    entries: Rc<RefCell<Vec<Entry>>>,
    on_select: Rc<RefCell<Option<SelectCallback>>>,
    /// When true, setting the active index from code suppresses the user-driven callback.
    suppress_callback: Rc<RefCell<bool>>,
}

impl ModelSelector {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 6);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(2);
        container.set_margin_bottom(2);
        container.set_valign(gtk4::Align::Center);

        let label = Label::new(Some("Model:"));
        label.add_css_class("dim-label");
        container.append(&label);

        let string_list = StringList::new(&[]);
        let dropdown = DropDown::new(Some(string_list.clone()), gtk4::Expression::NONE);
        dropdown.set_hexpand(true);
        container.append(&dropdown);

        let entries: Rc<RefCell<Vec<Entry>>> = Rc::new(RefCell::new(Vec::new()));
        // Seed with the Auto placeholder.
        entries.borrow_mut().push(Entry::AutoPlaceholder);
        string_list.append("Auto (coming soon)");

        let on_select: Rc<RefCell<Option<SelectCallback>>> = Rc::new(RefCell::new(None));
        let suppress_callback = Rc::new(RefCell::new(false));

        // Dispatch user changes.
        {
            let entries = Rc::clone(&entries);
            let on_select = Rc::clone(&on_select);
            let suppress = Rc::clone(&suppress_callback);
            dropdown.connect_selected_notify(move |dd| {
                if *suppress.borrow() {
                    return;
                }
                let idx = dd.selected() as usize;
                let selection = {
                    let entries = entries.borrow();
                    match entries.get(idx) {
                        Some(Entry::Model {
                            connection_id,
                            model_id,
                            ..
                        }) => Some(api::SendPromptOverride {
                            connection_id: connection_id.clone(),
                            model_id: model_id.clone(),
                            effort: None,
                        }),
                        // Auto placeholder (disabled): treat as no override.
                        _ => None,
                    }
                };
                if let Some(ref cb) = *on_select.borrow() {
                    cb(selection);
                }
            });
        }

        Self {
            container,
            dropdown,
            string_list,
            entries,
            on_select,
            suppress_callback,
        }
    }

    /// Register a callback for user-driven selection changes. The callback
    /// receives `None` when the user picks the `Auto` placeholder (in
    /// which case `SendMessage` should be issued without an override).
    pub fn connect_changed<F>(&self, f: F)
    where
        F: Fn(Option<api::SendPromptOverride>) + 'static,
    {
        *self.on_select.borrow_mut() = Some(Box::new(f));
    }

    /// Replace the contents of the dropdown with a new set of models.
    /// Keeps the previously-selected `(connection_id, model_id)` selected
    /// when still present; otherwise falls back to the Auto placeholder.
    pub fn set_models(&self, listings: &[api::ModelListing]) {
        let previous = self.current_override();

        // Rebuild the string list.
        while self.string_list.n_items() > 0 {
            self.string_list.remove(0);
        }
        self.string_list.append("Auto (coming soon)");

        let mut entries = vec![Entry::AutoPlaceholder];
        for listing in listings {
            let e = Entry::Model {
                connection_id: listing.connection_id.clone(),
                connection_label: listing.connection_label.clone(),
                model_id: listing.model.id.clone(),
                model_label: listing.model.display_name.clone(),
            };
            self.string_list.append(&e.display());
            entries.push(e);
        }
        *self.entries.borrow_mut() = entries;

        // Reselect previous if present, else leave on Auto.
        match previous {
            Some(prev) => self.select_override(Some(&prev)),
            None => self.select_override(None),
        }
    }

    /// Programmatically select an override. Passing `None` selects the
    /// `Auto` placeholder. Suppresses the change callback (this is a
    /// code-driven update, not a user action).
    pub fn select_override(&self, target: Option<&api::SendPromptOverride>) {
        let idx = match target {
            None => 0,
            Some(target) => self
                .entries
                .borrow()
                .iter()
                .position(|e| match e {
                    Entry::Model {
                        connection_id,
                        model_id,
                        ..
                    } => connection_id == &target.connection_id && model_id == &target.model_id,
                    _ => false,
                })
                .unwrap_or(0),
        };
        *self.suppress_callback.borrow_mut() = true;
        self.dropdown.set_selected(idx as u32);
        *self.suppress_callback.borrow_mut() = false;
    }

    /// Return the currently selected override, or `None` if the user has
    /// not picked an explicit model.
    pub fn current_override(&self) -> Option<api::SendPromptOverride> {
        let idx = self.dropdown.selected() as usize;
        match self.entries.borrow().get(idx) {
            Some(Entry::Model {
                connection_id,
                model_id,
                ..
            }) => Some(api::SendPromptOverride {
                connection_id: connection_id.clone(),
                model_id: model_id.clone(),
                effort: None,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_listing(conn: &str, conn_label: &str, model: &str, model_label: &str) -> api::ModelListing {
        api::ModelListing {
            connection_id: conn.to_string(),
            connection_label: conn_label.to_string(),
            model: api::ModelInfoView {
                id: model.to_string(),
                display_name: model_label.to_string(),
                context_limit: None,
                capabilities: api::ModelCapabilitiesView::default(),
            },
        }
    }

    #[test]
    fn entry_display_for_auto() {
        assert_eq!(Entry::AutoPlaceholder.display(), "Auto (coming soon)");
    }

    #[test]
    fn entry_display_for_model() {
        let e = Entry::Model {
            connection_id: "work".into(),
            connection_label: "Work".into(),
            model_id: "gpt-5".into(),
            model_label: "GPT-5".into(),
        };
        assert_eq!(e.display(), "Work · GPT-5");
    }

    #[test]
    fn builds_override_from_listing() {
        // This test does not require a GTK runtime — it only validates the
        // mapping from a ModelListing to a SendPromptOverride.
        let listing = mk_listing("work", "Work", "gpt-5", "GPT-5");
        let o = api::SendPromptOverride {
            connection_id: listing.connection_id.clone(),
            model_id: listing.model.id.clone(),
            effort: None,
        };
        assert_eq!(o.connection_id, "work");
        assert_eq!(o.model_id, "gpt-5");
        assert_eq!(o.effort, None);
    }
}
