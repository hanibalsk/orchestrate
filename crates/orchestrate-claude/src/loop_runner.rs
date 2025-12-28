//! Agent loop runner
//!
//! Features:
//! - Token optimization with message windowing
//! - Prompt caching for reduced costs
//! - Session management for continuity
//! - Dynamic output token allocation

use anyhow::Result;
use orchestrate_core::{
    Agent, AgentState, AgentType, CustomInstruction, Database, LearningEngine, Message, Session,
};
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info, warn};

use crate::client::{ClaudeClient, ContentBlock, CreateMessageRequest, MessageContent};
use crate::token::{ContextManager, TokenConfig, TokenEstimator};
use crate::tools::ToolExecutor;

/// Configuration for the agent loop
pub struct LoopConfig {
    pub max_turns: u32,
    pub model: String,
    /// Enable custom instruction injection
    pub enable_instructions: bool,
    /// Enable learning from agent runs
    pub enable_learning: bool,
    /// Max consecutive turns without progress before considering stuck
    pub max_idle_turns: u32,
    /// Max consecutive errors before aborting
    pub max_consecutive_errors: u32,
    /// Enable token optimization (windowing, caching)
    pub enable_token_optimization: bool,
    /// Enable session tracking
    pub enable_sessions: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_turns: 80,
            model: "claude-sonnet-4-20250514".to_string(),
            enable_instructions: true,
            enable_learning: true,
            max_idle_turns: 5,
            max_consecutive_errors: 3,
            enable_token_optimization: true,
            enable_sessions: true,
        }
    }
}

/// Agent loop runner
pub struct AgentLoop {
    client: ClaudeClient,
    db: Database,
    tool_executor: ToolExecutor,
    config: LoopConfig,
    learning_engine: LearningEngine,
    context_manager: ContextManager,
    token_estimator: TokenEstimator,
}

impl AgentLoop {
    /// Create a new agent loop
    pub fn new(client: ClaudeClient, db: Database, config: LoopConfig) -> Self {
        let context_manager = ContextManager::for_model(&config.model);
        Self {
            client,
            db,
            tool_executor: ToolExecutor::new(),
            context_manager,
            token_estimator: TokenEstimator::new(),
            config,
            learning_engine: LearningEngine::new(),
        }
    }

    /// Create a new agent loop with custom learning configuration
    pub fn with_learning_config(
        client: ClaudeClient,
        db: Database,
        config: LoopConfig,
        learning_engine: LearningEngine,
    ) -> Self {
        let context_manager = ContextManager::for_model(&config.model);
        Self {
            client,
            db,
            tool_executor: ToolExecutor::new(),
            context_manager,
            token_estimator: TokenEstimator::new(),
            config,
            learning_engine,
        }
    }

    /// Run the agent loop
    #[tracing::instrument(skip(self, agent), fields(agent_id = %agent.id, agent_type = ?agent.agent_type))]
    pub async fn run(&self, agent: &mut Agent) -> Result<()> {
        let start_time = Instant::now();
        info!("Starting agent loop for agent {}", agent.id);

        // Transition to initializing
        agent.transition_to(AgentState::Initializing)?;
        self.db.update_agent(agent).await?;

        // Load custom instructions for this agent type
        let (instructions, instruction_ids) = if self.config.enable_instructions {
            let insts = self.db.get_instructions_for_agent(agent.agent_type).await?;
            let ids: Vec<i64> = insts.iter().map(|i| i.id).collect();

            // Record instruction usage
            for inst in &insts {
                if let Err(e) = self.db.record_instruction_usage(
                    inst.id,
                    agent.id,
                    agent.session_id.as_deref(),
                ).await {
                    warn!("Failed to record instruction usage: {}", e);
                }
            }

            (insts, ids)
        } else {
            (Vec::new(), Vec::new())
        };

        debug!("Loaded {} custom instructions for agent {}", instructions.len(), agent.id);

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
        let mut was_blocked = false;
        let mut idle_turns = 0;  // Turns without tool calls or status signal
        let mut consecutive_errors = 0;  // Consecutive API or tool errors
        let mut last_tool_error: Option<String> = None;  // Track repeated errors
        let mut total_input_tokens = 0i32;
        let mut total_output_tokens = 0i32;
        let mut total_cache_read_tokens = 0i32;
        let mut total_cache_write_tokens = 0i32;

        // Create or get session for this agent
        let session_id = if self.config.enable_sessions {
            let session = Session::new(agent.id);
            if let Err(e) = self.db.create_session(&session).await {
                warn!("Failed to create session: {}", e);
            }
            // Update agent with session ID
            agent.session_id = Some(session.id.clone());
            self.db.update_agent(agent).await?;
            Some(session.id)
        } else {
            agent.session_id.clone()
        };

        info!(
            "[AGENT {}] Starting loop | Type: {:?} | Max turns: {} | Task: {}",
            agent.id, agent.agent_type, self.config.max_turns,
            agent.task.chars().take(100).collect::<String>()
        );

        loop {
            turn += 1;
            let turn_start = Instant::now();

            info!(
                "[AGENT {}] Turn {}/{} | Idle: {}/{} | Errors: {}/{} | Messages: {}",
                agent.id, turn, self.config.max_turns,
                idle_turns, self.config.max_idle_turns,
                consecutive_errors, self.config.max_consecutive_errors,
                messages.len()
            );

            // Check max turns
            if turn > self.config.max_turns {
                error!(
                    "[AGENT {}] STUCK: Reached max turns ({}) without completion",
                    agent.id, self.config.max_turns
                );
                error!(
                    "[AGENT {}] Last message: {}",
                    agent.id,
                    messages.last().map(|m| m.content.chars().take(200).collect::<String>()).unwrap_or_default()
                );
                agent.fail("Max turns reached - agent may be stuck in a loop")?;
                break;
            }

            // Check idle turns (no progress)
            if idle_turns >= self.config.max_idle_turns {
                error!(
                    "[AGENT {}] STUCK: {} consecutive turns without progress (no tools, no status signal)",
                    agent.id, idle_turns
                );
                error!(
                    "[AGENT {}] Last response: {}",
                    agent.id,
                    messages.last().map(|m| m.content.chars().take(500).collect::<String>()).unwrap_or_default()
                );
                agent.fail(&format!(
                    "Agent stuck: {} turns without progress. Last response had no tool calls or status signals.",
                    idle_turns
                ))?;
                break;
            }

            // Check consecutive errors
            if consecutive_errors >= self.config.max_consecutive_errors {
                error!(
                    "[AGENT {}] STUCK: {} consecutive errors",
                    agent.id, consecutive_errors
                );
                if let Some(ref err) = last_tool_error {
                    error!("[AGENT {}] Last error: {}", agent.id, err);
                }
                agent.fail(&format!(
                    "Agent stuck: {} consecutive errors. Last error: {}",
                    consecutive_errors,
                    last_tool_error.as_deref().unwrap_or("unknown")
                ))?;
                break;
            }

            // Apply message windowing if enabled
            let (api_messages, windowed_info) = if self.config.enable_token_optimization {
                let windowed = self.context_manager.window_messages(&messages);
                let mut msgs = Vec::new();

                // Add summary as first user message if present
                if let Some(ref summary) = windowed.summary {
                    msgs.push(MessageContent {
                        role: "user".to_string(),
                        content: serde_json::json!(summary),
                    });
                }

                // Add windowed messages
                msgs.extend(self.messages_to_api(&windowed.messages));

                debug!(
                    "[AGENT {}] Windowed messages: {} -> {} (summarized: {})",
                    agent.id, windowed.original_count, windowed.messages.len(), windowed.summarized_count
                );

                (msgs, Some(windowed))
            } else {
                (self.messages_to_api(&messages), None)
            };

            // Calculate dynamic max_tokens based on context usage
            let estimated_context = self.token_estimator.estimate_messages(&messages);
            let max_tokens = if self.config.enable_token_optimization {
                self.context_manager.calculate_output_tokens(estimated_context) as u32
            } else {
                4096
            };

            // Create request with prompt caching
            let (base_prompt, dynamic_suffix) = self.get_system_prompt_parts(agent, &instructions);
            let tools = self.tool_executor.get_tool_definitions(&agent.agent_type);

            // Build request - use caching if client supports it
            let request = if self.client.caching_enabled() && self.config.enable_token_optimization {
                CreateMessageRequest::new(self.config.model.clone(), max_tokens, api_messages)
                    .with_cached_system(&base_prompt, Some(&dynamic_suffix))
                    .with_tools(tools)
            } else {
                let full_prompt = if dynamic_suffix.is_empty() {
                    base_prompt
                } else {
                    format!("{}\n\n{}", base_prompt, dynamic_suffix)
                };
                CreateMessageRequest::new(self.config.model.clone(), max_tokens, api_messages)
                    .with_system(full_prompt)
                    .with_tools(tools)
            };

            // Call Claude API with error handling
            let response = match self.client.create_message(request).await {
                Ok(resp) => {
                    consecutive_errors = 0;  // Reset on success
                    resp
                }
                Err(e) => {
                    consecutive_errors += 1;
                    last_tool_error = Some(format!("API error: {}", e));
                    error!(
                        "[AGENT {}] API error (attempt {}/{}): {}",
                        agent.id, consecutive_errors, self.config.max_consecutive_errors, e
                    );
                    continue;  // Retry
                }
            };

            // Track token usage
            total_input_tokens += response.usage.input_tokens;
            total_output_tokens += response.usage.output_tokens;
            total_cache_read_tokens += response.usage.cache_read_input_tokens;
            total_cache_write_tokens += response.usage.cache_creation_input_tokens;

            // Log cache efficiency
            if response.usage.cache_read_input_tokens > 0 || response.usage.cache_creation_input_tokens > 0 {
                debug!(
                    "[AGENT {}] Cache stats: read={} write={} hit_rate={:.1}%",
                    agent.id,
                    response.usage.cache_read_input_tokens,
                    response.usage.cache_creation_input_tokens,
                    response.usage.cache_hit_rate()
                );
            }

            // Record token stats for this turn
            if let Some(ref sid) = session_id {
                let (msgs_included, msgs_summarized) = if let Some(ref w) = windowed_info {
                    (w.messages.len() as i32, w.summarized_count as i32)
                } else {
                    (messages.len() as i32, 0)
                };

                if let Err(e) = self.db.record_session_tokens(
                    sid,
                    agent.id,
                    turn as i32,
                    response.usage.input_tokens,
                    response.usage.output_tokens,
                    response.usage.cache_read_input_tokens,
                    response.usage.cache_creation_input_tokens,
                    estimated_context as i32,
                    msgs_included,
                    msgs_summarized,
                ).await {
                    warn!("Failed to record session tokens: {}", e);
                }
            }

            // Update daily token usage
            if let Err(e) = self.db.update_daily_token_usage(
                &self.config.model,
                response.usage.input_tokens,
                response.usage.output_tokens,
                response.usage.cache_read_input_tokens,
                response.usage.cache_creation_input_tokens,
            ).await {
                warn!("Failed to update daily token usage: {}", e);
            }

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

            debug!(
                "[AGENT {}] Response: {} chars, {} tool calls, stop_reason: {:?}",
                agent.id, text_content.len(), tool_calls.len(), response.stop_reason
            );

            // Store assistant message
            let assistant_msg = Message::assistant(agent.id, &text_content)
                .with_tool_calls(tool_calls.clone())
                .with_tokens(response.usage.input_tokens, response.usage.output_tokens);
            self.db.insert_message(&assistant_msg).await?;
            messages.push(assistant_msg);

            // Check for blocked status
            if self.is_blocked_signal(&text_content) {
                warn!("[AGENT {}] Agent signaled BLOCKED", agent.id);
                was_blocked = true;
                let reason = self.extract_blocked_reason(&text_content);
                warn!("[AGENT {}] Block reason: {}", agent.id, reason);
                agent.fail(&format!("Agent blocked: {}", reason))?;
                break;
            }

            // Check for completion
            if response.stop_reason.as_deref() == Some("end_turn") && tool_calls.is_empty() {
                if self.is_completion_signal(&text_content) {
                    info!("[AGENT {}] Completed successfully!", agent.id);
                    agent.transition_to(AgentState::Completed)?;
                    break;
                } else {
                    // No tool calls and no status signal - this is an idle turn
                    idle_turns += 1;
                    warn!(
                        "[AGENT {}] Idle turn {}/{}: No tool calls, no status signal",
                        agent.id, idle_turns, self.config.max_idle_turns
                    );
                    warn!(
                        "[AGENT {}] Response preview: {}...",
                        agent.id,
                        text_content.chars().take(200).collect::<String>()
                    );
                    // Continue to next turn - maybe agent will self-correct
                    continue;
                }
            }

            // Execute tool calls if any
            if !tool_calls.is_empty() {
                idle_turns = 0;  // Reset idle counter - we're making progress
                let mut results = Vec::new();
                let mut had_error = false;

                for tool_call in &tool_calls {
                    debug!(
                        "[AGENT {}] Executing tool: {} with input: {}",
                        agent.id, tool_call.name,
                        serde_json::to_string(&tool_call.input).unwrap_or_default().chars().take(200).collect::<String>()
                    );

                    let result = self
                        .tool_executor
                        .execute(&tool_call.name, &tool_call.input, agent)
                        .await;

                    let is_error = result.starts_with("Error:");
                    if is_error {
                        had_error = true;
                        last_tool_error = Some(result.clone());
                        warn!(
                            "[AGENT {}] Tool '{}' error: {}",
                            agent.id, tool_call.name,
                            result.chars().take(200).collect::<String>()
                        );
                    } else {
                        debug!(
                            "[AGENT {}] Tool '{}' success: {} chars",
                            agent.id, tool_call.name, result.len()
                        );
                    }

                    results.push(orchestrate_core::message::ToolResult {
                        tool_call_id: tool_call.id.clone(),
                        content: result.clone(),
                        is_error,
                    });
                }

                // Track consecutive errors
                if had_error {
                    consecutive_errors += 1;
                } else {
                    consecutive_errors = 0;
                }

                // Store tool results
                let tool_msg = Message::tool_result(agent.id, results);
                self.db.insert_message(&tool_msg).await?;
                messages.push(tool_msg);
            }

            // Check if waiting for external
            if self.needs_external_wait(&text_content) {
                info!("[AGENT {}] Waiting for external event", agent.id);
                agent.transition_to(AgentState::WaitingForExternal)?;
                break;
            }

            let turn_elapsed = turn_start.elapsed();
            debug!(
                "[AGENT {}] Turn {} completed in {:?}",
                agent.id, turn, turn_elapsed
            );
        }

        let total_elapsed = start_time.elapsed();

        // Calculate cache savings
        let cache_hit_rate = if total_input_tokens > 0 {
            (total_cache_read_tokens as f64 / total_input_tokens as f64) * 100.0
        } else {
            0.0
        };

        info!(
            "[AGENT {}] Loop finished | State: {:?} | Turns: {} | Time: {:?} | Tokens: in={} out={} cache_hit={:.1}%",
            agent.id, agent.state, turn, total_elapsed,
            total_input_tokens, total_output_tokens, cache_hit_rate
        );

        // Close session
        if let Some(ref sid) = session_id {
            if let Err(e) = self.db.close_session(sid).await {
                warn!("Failed to close session: {}", e);
            }

            // Update session total tokens
            if let Err(e) = self.db.update_session_tokens(sid, (total_input_tokens + total_output_tokens) as i64).await {
                warn!("Failed to update session tokens: {}", e);
            }
        }

        // Record instruction outcomes and apply learning
        let success = agent.state == AgentState::Completed;
        let completion_time = start_time.elapsed().as_secs_f64();

        if self.config.enable_instructions && !instruction_ids.is_empty() {
            // Record outcomes for each instruction
            for &id in &instruction_ids {
                if let Err(e) = self.db.record_instruction_outcome(id, success, Some(completion_time)).await {
                    warn!("Failed to record instruction outcome: {}", e);
                }

                // Track tokens per instruction
                if let Err(e) = self.db.update_instruction_tokens(
                    id,
                    total_input_tokens,
                    total_output_tokens,
                    total_cache_read_tokens,
                    total_cache_write_tokens,
                ).await {
                    warn!("Failed to update instruction tokens: {}", e);
                }
            }

            // Apply penalties/decay based on outcome
            if let Err(e) = self.learning_engine.apply_outcome_penalties(
                &self.db,
                &instruction_ids,
                success,
                was_blocked,
            ).await {
                warn!("Failed to apply outcome penalties: {}", e);
            }
        }

        // Analyze agent run for learning patterns
        if self.config.enable_learning && !success {
            // Reload messages for analysis
            let all_messages = self.db.get_messages(agent.id).await?;

            if let Err(e) = self.learning_engine.analyze_agent_run(
                &self.db,
                agent.id,
                agent.agent_type,
                &all_messages,
                success,
            ).await {
                warn!("Failed to analyze agent run for learning: {}", e);
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
        let (base, suffix) = self.get_system_prompt_parts(agent, &[]);
        if suffix.is_empty() {
            base
        } else {
            format!("{}\n\n{}", base, suffix)
        }
    }

    /// Get system prompt split into cacheable base and dynamic suffix
    ///
    /// The base prompt (agent identity, tools, status signals) is static and cacheable.
    /// The suffix (current task, custom instructions) changes per run.
    fn get_system_prompt_parts(&self, agent: &Agent, instructions: &[CustomInstruction]) -> (String, String) {
        // Try to load agent prompt from .claude/agents/ file
        let agent_prompt = self.load_agent_prompt(&agent.agent_type);

        // Base prompt - static, cacheable
        let base_prompt = if let Some(ref custom_prompt) = agent_prompt {
            format!(
                r#"{}

## Status Signals

When you complete your task, respond with "STATUS: COMPLETE" in your message.
If you need to wait for an external event (like PR review or CI), respond with "STATUS: WAITING".
If you encounter an error you cannot resolve, respond with "STATUS: BLOCKED: <reason>"."#,
                custom_prompt
            )
        } else {
            format!(
                r#"You are an autonomous agent of type: {:?}

You have access to these tools: {:?}

## Status Signals

When you complete your task, respond with "STATUS: COMPLETE" in your message.
If you need to wait for an external event (like PR review or CI), respond with "STATUS: WAITING".
If you encounter an error you cannot resolve, respond with "STATUS: BLOCKED: <reason>"."#,
                agent.agent_type,
                agent.agent_type.allowed_tools()
            )
        };

        // Dynamic suffix - changes per run
        let mut suffix_parts = Vec::new();

        // Add current task
        suffix_parts.push(format!("## Current Task\n\n{}", agent.task));

        // Add custom instructions if any
        if !instructions.is_empty() {
            let mut sorted_instructions: Vec<_> = instructions.iter().collect();
            sorted_instructions.sort_by(|a, b| b.priority.cmp(&a.priority));

            let instructions_block = sorted_instructions
                .iter()
                .map(|i| format!("- {}", i.content))
                .collect::<Vec<_>>()
                .join("\n");

            suffix_parts.push(format!(
                "## Custom Instructions\n\nThe following instructions have been learned from previous agent runs. Please follow them carefully:\n\n{}",
                instructions_block
            ));
        }

        (base_prompt, suffix_parts.join("\n\n"))
    }

    /// Load agent prompt from .claude/agents/<type>.md file
    fn load_agent_prompt(&self, agent_type: &AgentType) -> Option<String> {
        let filename = match agent_type {
            AgentType::StoryDeveloper => "story-developer.md",
            AgentType::CodeReviewer => "code-reviewer.md",
            AgentType::IssueFixer => "issue-fixer.md",
            AgentType::Explorer => "explorer.md",
            AgentType::BmadOrchestrator => "bmad-orchestrator.md",
            AgentType::BmadPlanner => "bmad-planner.md",
            AgentType::PrShepherd => "pr-shepherd.md",
            AgentType::PrController => "pr-controller.md",
            AgentType::ConflictResolver => "conflict-resolver.md",
            AgentType::BackgroundController => "background-controller.md",
            AgentType::Scheduler => "scheduler.md",
        };

        // Look for agent file in common locations
        let paths = [
            Path::new(".claude/agents").join(filename),
            Path::new(".").join(".claude/agents").join(filename),
        ];

        for path in &paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    // Parse frontmatter and extract content after ---
                    let prompt = self.extract_prompt_content(&content);
                    if !prompt.is_empty() {
                        debug!("Loaded agent prompt from {:?}", path);
                        return Some(prompt);
                    }
                }
            }
        }

        debug!("No custom prompt found for agent type {:?}", agent_type);
        None
    }

    /// Extract prompt content from markdown file with frontmatter
    fn extract_prompt_content(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        // Check for frontmatter
        if lines.first() == Some(&"---") {
            // Find closing ---
            if let Some(end_pos) = lines.iter().skip(1).position(|&l| l == "---") {
                // Return content after frontmatter
                return lines[end_pos + 2..].join("\n").trim().to_string();
            }
        }

        // No frontmatter, return full content
        content.trim().to_string()
    }

    fn is_completion_signal(&self, text: &str) -> bool {
        text.contains("STATUS: COMPLETE")
    }

    fn needs_external_wait(&self, text: &str) -> bool {
        text.contains("STATUS: WAITING")
    }

    fn is_blocked_signal(&self, text: &str) -> bool {
        text.contains("STATUS: BLOCKED")
    }

    fn extract_blocked_reason(&self, text: &str) -> String {
        if let Some(pos) = text.find("STATUS: BLOCKED:") {
            let after = &text[pos + "STATUS: BLOCKED:".len()..];
            // Take the rest of the line
            let reason = after.lines().next().unwrap_or("").trim();
            if !reason.is_empty() {
                return reason.to_string();
            }
        }
        "Unknown reason".to_string()
    }
}
