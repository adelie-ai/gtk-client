//! Persistence of per-conversation model selections.
//!
//! The daemon stores its own `last_model_selection` on the conversation row
//! and uses it as the implicit override when no per-send override is
//! supplied. The daemon does not currently echo that stored value back on
//! `GetConversation` — so for UI hydration (showing *which* model is
//! "stuck" to a conversation when the user re-opens it) we keep a local
//! mirror keyed by conversation id. The daemon remains the source of
//! truth; this mirror is best-effort.
//!
//! Layout: `~/.config/adele-gtk/conversation_selections.json`.

use anyhow::{Context, Result};
use desktop_assistant_api_model as api;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSelection {
    pub connection_id: String,
    pub model_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<api::EffortLevel>,
}

impl StoredSelection {
    pub fn as_override(&self) -> api::SendPromptOverride {
        api::SendPromptOverride {
            connection_id: self.connection_id.clone(),
            model_id: self.model_id.clone(),
            effort: self.effort,
        }
    }
}

impl From<api::ConversationModelSelectionView> for StoredSelection {
    fn from(v: api::ConversationModelSelectionView) -> Self {
        Self {
            connection_id: v.connection_id,
            model_id: v.model_id,
            effort: v.effort,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct SelectionsFile {
    #[serde(default)]
    selections: BTreeMap<String, StoredSelection>,
}

pub struct SelectionStore {
    path: PathBuf,
    cache: RwLock<BTreeMap<String, StoredSelection>>,
}

impl SelectionStore {
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("adele-gtk")
            .join("conversation_selections.json");
        let cache = RwLock::new(Self::load_from_disk(&path).unwrap_or_default());
        Self { path, cache }
    }

    fn load_from_disk(path: &PathBuf) -> Result<BTreeMap<String, StoredSelection>> {
        if !path.exists() {
            return Ok(BTreeMap::new());
        }
        let data = std::fs::read_to_string(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let file: SelectionsFile =
            serde_json::from_str(&data).with_context(|| "parsing conversation_selections.json")?;
        Ok(file.selections)
    }

    fn save_to_disk(&self) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let map = self.cache.read().expect("selection cache poisoned").clone();
        let file = SelectionsFile { selections: map };
        let data = serde_json::to_string_pretty(&file)?;
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    /// Get the last selection for a given conversation, if any.
    pub fn get(&self, conversation_id: &str) -> Option<StoredSelection> {
        self.cache
            .read()
            .expect("selection cache poisoned")
            .get(conversation_id)
            .cloned()
    }

    /// Persist a selection for a conversation. Errors writing to disk are
    /// logged but not surfaced — the in-memory cache is always updated.
    pub fn set(&self, conversation_id: &str, selection: StoredSelection) {
        self.cache
            .write()
            .expect("selection cache poisoned")
            .insert(conversation_id.to_string(), selection);
        if let Err(e) = self.save_to_disk() {
            tracing::warn!("failed to persist conversation selection: {e}");
        }
    }

    /// Forget any stored selection for a conversation (e.g. on
    /// DanglingModelSelection warning the daemon has already cleared its
    /// side).
    pub fn clear(&self, conversation_id: &str) {
        self.cache
            .write()
            .expect("selection cache poisoned")
            .remove(conversation_id);
        if let Err(e) = self.save_to_disk() {
            tracing::warn!("failed to persist conversation selection: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_selection_roundtrips_through_override() {
        let stored = StoredSelection {
            connection_id: "work".into(),
            model_id: "gpt-5".into(),
            effort: Some(api::EffortLevel::Medium),
        };
        let o = stored.as_override();
        assert_eq!(o.connection_id, "work");
        assert_eq!(o.model_id, "gpt-5");
        assert_eq!(o.effort, Some(api::EffortLevel::Medium));
    }

    #[test]
    fn from_conversation_model_selection_view() {
        let v = api::ConversationModelSelectionView {
            connection_id: "aws".into(),
            model_id: "claude-sonnet-4".into(),
            effort: Some(api::EffortLevel::High),
        };
        let s: StoredSelection = v.into();
        assert_eq!(s.connection_id, "aws");
        assert_eq!(s.model_id, "claude-sonnet-4");
        assert_eq!(s.effort, Some(api::EffortLevel::High));
    }

    #[test]
    fn selection_store_set_get_clear_via_tempdir() {
        let tempdir = tempfile::tempdir().unwrap();
        let store = SelectionStore {
            path: tempdir.path().join("sel.json"),
            cache: RwLock::new(BTreeMap::new()),
        };
        let sel = StoredSelection {
            connection_id: "c".into(),
            model_id: "m".into(),
            effort: None,
        };
        assert!(store.get("conv1").is_none());
        store.set("conv1", sel.clone());
        assert_eq!(store.get("conv1"), Some(sel));
        store.clear("conv1");
        assert!(store.get("conv1").is_none());
    }
}
