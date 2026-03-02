use desktop_assistant_client_common::ConversationSummary;
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, SelectionMode,
};

/// Sidebar widget displaying the conversation list and a "New" button.
pub struct Sidebar {
    pub container: GtkBox,
    pub list_box: ListBox,
    pub new_button: Button,
    pub scrolled_window: ScrolledWindow,
}

impl Sidebar {
    pub fn new() -> Self {
        let container = GtkBox::new(Orientation::Vertical, 0);
        container.set_width_request(280);

        let header = Label::new(Some("Conversations"));
        header.add_css_class("sidebar-header");
        header.set_halign(gtk4::Align::Start);
        header.set_margin_start(12);
        header.set_margin_top(8);
        header.set_margin_bottom(8);
        container.append(&header);

        let scrolled_window = ScrolledWindow::new();
        scrolled_window.set_vexpand(true);

        let list_box = ListBox::new();
        list_box.set_selection_mode(SelectionMode::Single);
        list_box.add_css_class("conversation-list");
        scrolled_window.set_child(Some(&list_box));
        container.append(&scrolled_window);

        let new_button = Button::with_label("+ New Conversation");
        new_button.add_css_class("new-conversation-button");
        new_button.set_margin_start(8);
        new_button.set_margin_end(8);
        new_button.set_margin_top(8);
        new_button.set_margin_bottom(8);
        container.append(&new_button);

        Self {
            container,
            list_box,
            new_button,
            scrolled_window,
        }
    }

    /// Replace the conversation list contents.
    pub fn set_conversations(&self, conversations: &[ConversationSummary]) {
        // Remove all existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }

        for conv in conversations {
            let row = ListBoxRow::new();
            let hbox = GtkBox::new(Orientation::Horizontal, 8);
            hbox.set_margin_start(12);
            hbox.set_margin_end(12);
            hbox.set_margin_top(6);
            hbox.set_margin_bottom(6);

            let title_label = Label::new(Some(&conv.title));
            title_label.set_halign(gtk4::Align::Start);
            title_label.set_hexpand(true);
            title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            hbox.append(&title_label);

            let count_label = Label::new(Some(&format!("({})", conv.message_count)));
            count_label.add_css_class("dim-label");
            hbox.append(&count_label);

            row.set_child(Some(&hbox));
            self.list_box.append(&row);
        }
    }

    /// Get the index of the currently selected row.
    pub fn selected_index(&self) -> Option<usize> {
        let row = self.list_box.selected_row()?;
        Some(row.index() as usize)
    }

    /// Select a row by index.
    pub fn select_index(&self, index: usize) {
        if let Some(row) = self.list_box.row_at_index(index as i32) {
            self.list_box.select_row(Some(&row));
        }
    }
}
