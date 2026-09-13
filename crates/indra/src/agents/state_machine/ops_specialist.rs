//! Implements the INDRA Specialist and Operation Router state-machine operation.
//! Classifies incoming user prompts, selects the appropriate domain specialist,
//! injects specialist system prompt framing, and intercepts stub requests.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

use crate::agents::specialists::SpecialistRegistry;
use crate::agents::state_machine::effects::GooseEffect;
use crate::agents::state_machine::{
    messages_since_kickoff, not_applicable, yielded_with, Emitter, Operation, OperationResult,
};
use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::session::Session;

pub struct SpecialistOperation {
    registry: SpecialistRegistry,
}

impl Default for SpecialistOperation {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialistOperation {
    pub fn new() -> Self {
        Self {
            registry: SpecialistRegistry::new(),
        }
    }

    fn extract_latest_user_text(conversation: &Conversation) -> Option<String> {
        conversation.messages().iter().rev().find_map(|m| {
            if m.role == rmcp::model::Role::User {
                let text = m
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        MessageContent::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                if !text.is_empty() {
                    return Some(text);
                }
            }
            None
        })
    }
}

#[async_trait]
impl Operation<Session, GooseEffect> for SpecialistOperation {
    fn name(&self) -> &'static str {
        "specialist"
    }

    async fn prompt_parts(
        &self,
        _session: &Session,
        conversation: &Conversation,
    ) -> Result<Vec<(String, String)>> {
        let Some(user_text) = Self::extract_latest_user_text(conversation) else {
            return Ok(Vec::new());
        };

        let (op, specialist) = self.registry.route(&user_text);

        let mut parts = Vec::new();
        parts.push((
            "specialist_routing".to_string(),
            format!(
                "**Active Specialist**: {}\n**Operation Classified**: {}",
                specialist.name(),
                op.display_name()
            ),
        ));

        if let Some(framing) = specialist.system_prompt_framing() {
            parts.push(("specialist_domain_framing".to_string(), framing.to_string()));
        }

        Ok(parts)
    }

    async fn run(
        &self,
        _session: &Session,
        conversation: &Conversation,
        emit: &Emitter,
    ) -> Result<OperationResult<GooseEffect>> {
        let messages = match messages_since_kickoff(conversation) {
            Ok(m) => m,
            Err(_) => return not_applicable(),
        };

        // If assistant has already spoken in this turn, don't re-run stub checks
        if messages
            .iter()
            .any(|m| m.role == rmcp::model::Role::Assistant)
        {
            return not_applicable();
        }

        let Some(user_text) = Self::extract_latest_user_text(conversation) else {
            return not_applicable();
        };

        let (op, specialist) = self.registry.route(&user_text);

        // If this specialist is a registered stub (e.g. P&ID, Engineering Calc),
        // cleanly inform the user without consuming model inference tokens.
        if specialist.is_stub() {
            if let Some(stub_msg) = specialist.stub_message() {
                let mut response = Message::assistant().with_text(stub_msg);
                self.set_message_meta(&mut response, "specialist_name", json!(specialist.name()));
                self.set_message_meta(&mut response, "operation", json!(op.as_str()));
                let message = emit.message(response).await;
                return yielded_with([message.into()]);
            }
        }

        not_applicable()
    }
}
