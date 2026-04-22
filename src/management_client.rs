//! Thin wrapper over the live `TransportClient` for named-connection
//! management commands.
//!
//! Uses the WS connection's raw `send_command` escape hatch (exposed by
//! `client-common`). Returns typed results so callers don't need to pattern
//! match the protocol envelope.

use anyhow::{Result, anyhow};
use desktop_assistant_api_model as api;
use desktop_assistant_client_common::TransportClient;

fn ws<'a>(transport: &'a TransportClient) -> Result<&'a desktop_assistant_client_common::ws_client::WsClient> {
    transport
        .as_ws()
        .ok_or_else(|| anyhow!("named-connection management requires a WebSocket transport"))
}

pub async fn list_connections(transport: &TransportClient) -> Result<Vec<api::ConnectionView>> {
    let result = ws(transport)?.send_command(api::Command::ListConnections).await?;
    match result {
        api::CommandResult::Connections(list) => Ok(list),
        other => Err(anyhow!("unexpected response for ListConnections: {other:?}")),
    }
}

pub async fn create_connection(
    transport: &TransportClient,
    id: String,
    config: api::ConnectionConfigView,
) -> Result<()> {
    let result = ws(transport)?
        .send_command(api::Command::CreateConnection { id, config })
        .await?;
    match result {
        api::CommandResult::Ack => Ok(()),
        other => Err(anyhow!("unexpected response for CreateConnection: {other:?}")),
    }
}

pub async fn update_connection(
    transport: &TransportClient,
    id: String,
    config: api::ConnectionConfigView,
) -> Result<()> {
    let result = ws(transport)?
        .send_command(api::Command::UpdateConnection { id, config })
        .await?;
    match result {
        api::CommandResult::Ack => Ok(()),
        other => Err(anyhow!("unexpected response for UpdateConnection: {other:?}")),
    }
}

pub async fn delete_connection(
    transport: &TransportClient,
    id: String,
    force: bool,
) -> Result<()> {
    let result = ws(transport)?
        .send_command(api::Command::DeleteConnection { id, force })
        .await?;
    match result {
        api::CommandResult::Ack => Ok(()),
        other => Err(anyhow!("unexpected response for DeleteConnection: {other:?}")),
    }
}

pub async fn list_available_models(
    transport: &TransportClient,
    connection_id: Option<String>,
    refresh: bool,
) -> Result<Vec<api::ModelListing>> {
    let result = ws(transport)?
        .send_command(api::Command::ListAvailableModels {
            connection_id,
            refresh,
        })
        .await?;
    match result {
        api::CommandResult::Models(m) => Ok(m),
        other => Err(anyhow!("unexpected response for ListAvailableModels: {other:?}")),
    }
}

pub async fn get_purposes(transport: &TransportClient) -> Result<api::PurposesView> {
    let result = ws(transport)?.send_command(api::Command::GetPurposes).await?;
    match result {
        api::CommandResult::Purposes(p) => Ok(p),
        other => Err(anyhow!("unexpected response for GetPurposes: {other:?}")),
    }
}

pub async fn set_purpose(
    transport: &TransportClient,
    purpose: api::PurposeKindApi,
    config: api::PurposeConfigView,
) -> Result<()> {
    let result = ws(transport)?
        .send_command(api::Command::SetPurpose { purpose, config })
        .await?;
    match result {
        api::CommandResult::Ack => Ok(()),
        other => Err(anyhow!("unexpected response for SetPurpose: {other:?}")),
    }
}

pub async fn send_prompt_with_override(
    transport: &TransportClient,
    conversation_id: String,
    content: String,
    override_selection: Option<api::SendPromptOverride>,
) -> Result<()> {
    let result = ws(transport)?
        .send_command(api::Command::SendMessage {
            conversation_id,
            content,
            override_selection,
        })
        .await?;
    match result {
        api::CommandResult::Ack => Ok(()),
        other => Err(anyhow!("unexpected response for SendMessage: {other:?}")),
    }
}

/// Fetch the raw `api::ConversationView` so callers can inspect warnings
/// (which the higher-level `ConversationDetail` strips). Falls back to a
/// view with empty warnings when the transport is not WebSocket.
pub async fn get_conversation_view(
    transport: &TransportClient,
    id: &str,
) -> Result<api::ConversationView> {
    let result = ws(transport)?
        .send_command(api::Command::GetConversation { id: id.to_string() })
        .await?;
    match result {
        api::CommandResult::Conversation(view) => Ok(view),
        other => Err(anyhow!("unexpected response for GetConversation: {other:?}")),
    }
}
