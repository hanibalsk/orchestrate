//! Agent loop runner

use anyhow::Result;
use orchestrate_core::{Agent, AgentState, Database, Message};
use tracing::{debug, info, warn};

use crate::client::{ClaudeClient, ContentBlock, CreateMessageRequest, MessageContent};
use crate::tools::ToolExecutor;

/// Configuration for the agent loop
pub struct LoopConfig {
    pub max_turns: u32,
    pub model: String,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_turns: 80,
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }
}

/// Agent loop runner
pub struct AgentLoop {
    client: ClaudeClient,
    db: Database,
    tool_executor: ToolExecutor,
    config: LoopConfig,
}

impl AgentLoop {
    /// Create a new agent loop
    pub fn new(client: ClaudeClient, db: Database, config: LoopConfig) -> Self {
        Self {
            client,
            db,
            tool_executor: ToolExecutor::new(),
            config,
        }
    }

    /// Run the agent loop
    pub async fn run(&self, agent: &mut Agent) -> Result<()> {
        info!("Starting agent loop for agent {}", agent.id);

        // Transition to initializing
        agent.transition_to(AgentState::Initializing)?;
        self.db.update_agent(agent).await?;

        // Load message history
        let mut messages = self.db.get_messages(agent.id).await?;

        // Add initial task as user message if no messages
        if messages.is_empty() {
            let user_msg = Message::user(agent.id, &agent.task);
            self.db.insert_message(&user_msg).await?;
            messages.push(user_msg);
        }

        // Transition to running
        agent.transition_to(AgentState::Running)?;
        self.db.update_agent(agent).await?;

        let mut turn = 0;

        loop {
            turn += 1;
            debug!("Agent {} turn {}/{}", agent.id, turn, self.config.max_turns);

            if turn > self.config.max_turns {
                warn!("Agent {} reached max turns", agent.id);
                agent.fail("Max turns reached")?;
                break;
            }

            // Convert messages to API format
            let api_messages = self.messages_to_api(&messages);

            // Create request
            let request = CreateMessageRequest {
                model: self.config.model.clone(),
                max_tokens: 4096,
                messages: api_messages,
                system: Some(self.get_system_prompt(agent)),
                tools: Some(self.tool_executor.get_tool_definitions(&agent.agent_type)),
            };

            // Call Claude API
            let response = self.client.create_message(request).await?;

            // Extract text and tool calls
            let mut text_content = String::new();
            let mut tool_calls = Vec::new();

            for block in &response.content {
                match block {
                    ContentBlock::Text { text } => {
                        text_content.push_str(text);
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        tool_calls.push(orchestrate_core::message::ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });
                    }
                }
            }

            // Store assistant message
            let assistant_msg = Message::assistant(agent.id, &text_content)
                .with_tool_calls(tool_calls.clone())
                .with_tokens(response.usage.input_tokens, response.usage.output_tokens);
            self.db.insert_message(&assistant_msg).await?;
            messages.push(assistant_msg);

            // Check for completion
            if response.stop_reason.as_deref() == Some("end_turn") && tool_calls.is_empty() {
                // Check if agent signaled completion
                if self.is_completion_signal(&text_content) {
                    info!("Agent {} completed", agent.id);
                    agent.transition_to(AgentState::Completed)?;
                    break;
                }
            }

            // Execute tool calls if any
            if !tool_calls.is_empty() {
                let mut results = Vec::new();

                for tool_call in &tool_calls {
                    let result = self
                        .tool_executor
                        .execute(&tool_call.name, &tool_call.input, agent)
                        .await;

                    results.push(orchestrate_core::message::ToolResult {
                        tool_call_id: tool_call.id.clone(),
                        content: result.clone(),
                        is_error: result.starts_with("Error:"),
                    });
                }

                // Store tool results
                let tool_msg = Message::tool_result(agent.id, results);
                self.db.insert_message(&tool_msg).await?;
                messages.push(tool_msg);
            }

            // Check if waiting for external
            if self.needs_external_wait(&text_content) {
                info!("Agent {} waiting for external event", agent.id);
                agent.transition_to(AgentState::WaitingForExternal)?;
                break;
            }
        }

        self.db.update_agent(agent).await?;
        Ok(())
    }

    fn messages_to_api(&self, messages: &[Message]) -> Vec<MessageContent> {
        messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    orchestrate_core::message::MessageRole::User => "user",
                    orchestrate_core::message::MessageRole::Assistant => "assistant",
                    orchestrate_core::message::MessageRole::System => "user", // System messages as user
                    orchestrate_core::message::MessageRole::Tool => "user",   // Tool results as user
                };

                let content = if let Some(tool_results) = &msg.tool_results {
                    // Format tool results
                    serde_json::json!(tool_results
                        .iter()
                        .map(|r| {
                            serde_json::json!({
                                "type": "tool_result",
                                "tool_use_id": r.tool_call_id,
                                "content": r.content,
                            })
                        })
                        .collect::<Vec<_>>())
                } else {
                    serde_json::json!(msg.content)
                };

                MessageContent {
                    role: role.to_string(),
                    content,
                }
            })
            .collect()
    }

    fn get_system_prompt(&self, agent: &Agent) -> String {
        format!(
            r#"You are an autonomous agent working on the following task:

{}

Your agent type is: {:?}

You have access to these tools: {:?}

When you complete your task, respond with "STATUS: COMPLETE" in your message.
If you need to wait for an external event (like PR review or CI), respond with "STATUS: WAITING".
If you encounter an error you cannot resolve, respond with "STATUS: BLOCKED: <reason>".
"#,
            agent.task,
            agent.agent_type,
            agent.agent_type.allowed_tools()
        )
    }

    fn is_completion_signal(&self, text: &str) -> bool {
        text.contains("STATUS: COMPLETE")
    }

    fn needs_external_wait(&self, text: &str) -> bool {
        text.contains("STATUS: WAITING")
    }
}
