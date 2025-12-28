//! Orchestrate CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchestrate_core::{
    Agent, AgentType, CustomInstruction, Database,
    LearningEngine, PatternStatus,
};
use std::path::PathBuf;
use tracing::Level;
use tracing_subscriber::EnvFilter;

/// Initialize logging with the specified verbosity level
fn init_logging(verbose: u8, quiet: bool, json: bool) -> Result<()> {
    let level = if quiet {
        Level::ERROR
    } else {
        match verbose {
            0 => Level::WARN,
            1 => Level::INFO,
            2 => Level::DEBUG,
            _ => Level::TRACE,
        }
    };

    let filter = EnvFilter::from_default_env()
        .add_directive(format!("orchestrate={}", level).parse()?);

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbose >= 2)      // Show module path at debug+
        .with_file(verbose >= 3)        // Show file:line at trace
        .with_line_number(verbose >= 3);

    if json {
        builder.json().init();
    } else {
        builder.init();
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "orchestrate")]
#[command(about = "Multi-agent orchestrator for Claude Code")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Database path
    #[arg(long, env = "ORCHESTRATE_DB_PATH", default_value = "~/.orchestrate/orchestrate.db")]
    db_path: String,

    /// Increase verbosity (-v: info, -vv: debug, -vvv: trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Quiet mode (suppress non-error output)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Output logs as JSON (for machine parsing)
    #[arg(long, global = true)]
    log_json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Daemon management
    Daemon {
        #[command(subcommand)]
        action: DaemonAction,
    },
    /// Agent management
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// PR management
    Pr {
        #[command(subcommand)]
        action: PrAction,
    },
    /// Worktree management
    Wt {
        #[command(subcommand)]
        action: WtAction,
    },
    /// BMAD workflow
    Bmad {
        #[command(subcommand)]
        action: BmadAction,
    },
    /// Start web interface
    Web {
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Show system status
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Debug utilities
    Debug {
        #[command(subcommand)]
        action: DebugAction,
    },
    /// Manage custom instructions
    Instructions {
        #[command(subcommand)]
        action: InstructionAction,
    },
    /// Learning and pattern management
    Learn {
        #[command(subcommand)]
        action: LearnAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start {
        #[arg(short, long, default_value = "9999")]
        port: u16,
    },
    /// Stop the daemon
    Stop,
    /// Show daemon status
    Status,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Spawn a new agent
    Spawn {
        #[arg(short = 't', long)]
        agent_type: String,
        #[arg(short = 'T', long)]
        task: String,
        #[arg(short, long)]
        worktree: Option<String>,
    },
    /// List agents
    List {
        #[arg(short, long)]
        state: Option<String>,
    },
    /// Show agent details
    Show {
        id: String,
    },
    /// Pause an agent
    Pause {
        id: String,
    },
    /// Resume an agent
    Resume {
        id: String,
    },
    /// Terminate an agent
    Terminate {
        id: String,
    },
}

#[derive(Subcommand)]
enum PrAction {
    /// List PRs
    List {
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Create a PR
    Create {
        #[arg(short, long)]
        worktree: Option<String>,
        #[arg(short, long)]
        title: Option<String>,
    },
    /// Merge a PR
    Merge {
        number: i32,
        #[arg(short, long, default_value = "squash")]
        strategy: String,
    },
    /// Show PR queue
    Queue,
}

#[derive(Subcommand)]
enum WtAction {
    /// Create a worktree
    Create {
        name: String,
        #[arg(short, long, default_value = "main")]
        base: String,
    },
    /// List worktrees
    List,
    /// Remove a worktree
    Remove {
        name: String,
    },
}

#[derive(Subcommand)]
enum BmadAction {
    /// Process epics
    Process {
        pattern: Option<String>,
    },
    /// Show BMAD status
    Status,
    /// Reset BMAD state
    Reset,
}

#[derive(Subcommand)]
enum DebugAction {
    /// Show database info and statistics
    Db,
    /// Test database connection
    Ping,
    /// Show current configuration
    Config,
    /// Dump internal state
    Dump {
        /// What to dump: agents, prs, epics, all
        #[arg(default_value = "all")]
        target: String,
    },
}

#[derive(Subcommand)]
enum InstructionAction {
    /// List all instructions
    List {
        /// Show only enabled instructions
        #[arg(long)]
        enabled_only: bool,
        /// Show only learned instructions
        #[arg(long)]
        learned_only: bool,
    },
    /// Show instruction details
    Show {
        /// Instruction ID or name
        id_or_name: String,
    },
    /// Create a new instruction
    Create {
        /// Instruction name
        #[arg(short, long)]
        name: String,
        /// Instruction content
        #[arg(short, long)]
        content: String,
        /// Scope (global or agent_type)
        #[arg(short, long, default_value = "global")]
        scope: String,
        /// Agent type (required if scope is agent_type)
        #[arg(short = 't', long)]
        agent_type: Option<String>,
        /// Priority (higher = injected earlier)
        #[arg(short, long, default_value = "100")]
        priority: i32,
    },
    /// Enable an instruction
    Enable {
        /// Instruction ID or name
        id_or_name: String,
    },
    /// Disable an instruction
    Disable {
        /// Instruction ID or name
        id_or_name: String,
    },
    /// Delete an instruction
    Delete {
        /// Instruction ID or name
        id_or_name: String,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Show instruction effectiveness statistics
    Stats {
        /// Instruction ID or name (optional, shows all if not specified)
        id_or_name: Option<String>,
    },
}

#[derive(Subcommand)]
enum LearnAction {
    /// List learning patterns
    Patterns {
        /// Show only pending patterns
        #[arg(long)]
        pending_only: bool,
    },
    /// Approve a pattern and create instruction
    Approve {
        /// Pattern ID
        pattern_id: i64,
    },
    /// Reject a pattern
    Reject {
        /// Pattern ID
        pattern_id: i64,
    },
    /// Process patterns and create instructions
    Analyze,
    /// Show learning configuration
    Config,
    /// Cleanup ineffective instructions
    Cleanup,
    /// Reset penalty score for an instruction
    ResetPenalty {
        /// Instruction ID or name
        id_or_name: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging with CLI options
    init_logging(cli.verbose, cli.quiet, cli.log_json)?;

    // Expand home directory
    let db_path = shellexpand::tilde(&cli.db_path).to_string();
    let db_path = PathBuf::from(db_path);

    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = Database::new(&db_path).await?;

    match cli.command {
        Commands::Daemon { action } => match action {
            DaemonAction::Start { port } => {
                println!("Starting daemon on port {}...", port);
                // TODO: Implement daemon
                println!("Daemon started");
            }
            DaemonAction::Stop => {
                println!("Stopping daemon...");
                // TODO: Implement daemon stop
            }
            DaemonAction::Status => {
                println!("Daemon status: not implemented");
            }
        },

        Commands::Agent { action } => match action {
            AgentAction::Spawn { agent_type, task, worktree } => {
                let agent_type = parse_agent_type(&agent_type)?;
                let mut agent = Agent::new(agent_type, task);

                if let Some(wt) = worktree {
                    agent = agent.with_worktree(wt);
                }

                db.insert_agent(&agent).await?;
                println!("Agent spawned: {}", agent.id);
            }
            AgentAction::List { state: _ } => {
                let agents = db.list_agents().await?;
                println!("{:<36} {:<20} {:<15} {}", "ID", "TYPE", "STATE", "TASK");
                println!("{}", "-".repeat(100));
                for agent in agents {
                    println!(
                        "{:<36} {:<20} {:<15} {}",
                        agent.id,
                        format!("{:?}", agent.agent_type),
                        format!("{:?}", agent.state),
                        &agent.task[..agent.task.len().min(40)]
                    );
                }
            }
            AgentAction::Show { id } => {
                let uuid = uuid::Uuid::parse_str(&id)?;
                if let Some(agent) = db.get_agent(uuid).await? {
                    println!("Agent: {}", agent.id);
                    println!("Type: {:?}", agent.agent_type);
                    println!("State: {:?}", agent.state);
                    println!("Task: {}", agent.task);
                    println!("Created: {}", agent.created_at);
                    println!("Updated: {}", agent.updated_at);
                } else {
                    println!("Agent not found: {}", id);
                }
            }
            AgentAction::Pause { id } => {
                let uuid = uuid::Uuid::parse_str(&id)?;
                if let Some(mut agent) = db.get_agent(uuid).await? {
                    agent.transition_to(orchestrate_core::AgentState::Paused)?;
                    db.update_agent(&agent).await?;
                    println!("Agent paused: {}", id);
                } else {
                    println!("Agent not found: {}", id);
                }
            }
            AgentAction::Resume { id } => {
                let uuid = uuid::Uuid::parse_str(&id)?;
                if let Some(mut agent) = db.get_agent(uuid).await? {
                    agent.transition_to(orchestrate_core::AgentState::Running)?;
                    db.update_agent(&agent).await?;
                    println!("Agent resumed: {}", id);
                } else {
                    println!("Agent not found: {}", id);
                }
            }
            AgentAction::Terminate { id } => {
                let uuid = uuid::Uuid::parse_str(&id)?;
                if let Some(mut agent) = db.get_agent(uuid).await? {
                    agent.transition_to(orchestrate_core::AgentState::Terminated)?;
                    db.update_agent(&agent).await?;
                    println!("Agent terminated: {}", id);
                } else {
                    println!("Agent not found: {}", id);
                }
            }
        },

        Commands::Pr { action } => match action {
            PrAction::List { status: _ } => {
                let prs = db.get_pending_prs().await?;
                println!("{:<6} {:<20} {:<15} {}", "ID", "BRANCH", "STATUS", "TITLE");
                println!("{}", "-".repeat(80));
                for pr in prs {
                    println!(
                        "{:<6} {:<20} {:<15} {}",
                        pr.id,
                        &pr.branch_name[..pr.branch_name.len().min(20)],
                        format!("{:?}", pr.status),
                        pr.title.as_deref().unwrap_or("-")
                    );
                }
            }
            PrAction::Create { worktree: _, title: _ } => {
                println!("Creating PR... (not implemented)");
            }
            PrAction::Merge { number, strategy } => {
                println!("Merging PR #{} with {} strategy...", number, strategy);
                // TODO: Implement merge
            }
            PrAction::Queue => {
                let prs = db.get_pending_prs().await?;
                println!("PR Queue ({} items):", prs.len());
                for (i, pr) in prs.iter().enumerate() {
                    println!(
                        "  {}. {} ({:?})",
                        i + 1,
                        pr.branch_name,
                        pr.status
                    );
                }
            }
        },

        Commands::Wt { action } => match action {
            WtAction::Create { name, base } => {
                println!("Creating worktree {} from {}...", name, base);
                // TODO: Implement worktree creation
            }
            WtAction::List => {
                println!("Worktrees: (not implemented)");
            }
            WtAction::Remove { name } => {
                println!("Removing worktree {}...", name);
                // TODO: Implement worktree removal
            }
        },

        Commands::Bmad { action } => match action {
            BmadAction::Process { pattern } => {
                println!(
                    "Processing epics{}...",
                    pattern.as_ref().map(|p| format!(" matching '{}'", p)).unwrap_or_default()
                );
                // TODO: Implement BMAD processing
            }
            BmadAction::Status => {
                let epics = db.get_pending_epics().await?;
                println!("Pending epics: {}", epics.len());
                for epic in epics {
                    println!("  - {}: {}", epic.id, epic.title);
                }
            }
            BmadAction::Reset => {
                println!("Resetting BMAD state... (not implemented)");
            }
        },

        Commands::Web { port } => {
            use orchestrate_web::{api::AppState, create_router};
            use std::sync::Arc;

            println!("Starting web server on http://localhost:{}", port);

            // Get API key from environment if set
            let api_key = std::env::var("ORCHESTRATE_API_KEY").ok();
            if api_key.is_some() {
                println!("API key authentication enabled");
            }

            let state = Arc::new(AppState::new(db, api_key));
            let app = create_router(state);

            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
            axum::serve(listener, app).await?;
        }

        Commands::Status { json } => {
            let agents = db.list_agents().await?;
            let running = agents
                .iter()
                .filter(|a| a.state == orchestrate_core::AgentState::Running)
                .count();
            let paused = agents
                .iter()
                .filter(|a| a.state == orchestrate_core::AgentState::Paused)
                .count();

            if json {
                println!(
                    r#"{{"total_agents":{},"running":{},"paused":{}}}"#,
                    agents.len(),
                    running,
                    paused
                );
            } else {
                println!("System Status");
                println!("=============");
                println!("Total agents: {}", agents.len());
                println!("Running: {}", running);
                println!("Paused: {}", paused);
            }
        }

        Commands::Debug { action } => match action {
            DebugAction::Db => {
                println!("Database Info");
                println!("=============");
                println!("Path: {}", db_path.display());
                let agents = db.list_agents().await?;
                let prs = db.get_pending_prs().await?;
                let epics = db.get_pending_epics().await?;
                println!("Agents: {}", agents.len());
                println!("Pending PRs: {}", prs.len());
                println!("Pending Epics: {}", epics.len());
            }
            DebugAction::Ping => {
                let start = std::time::Instant::now();
                let _ = db.list_agents().await?;
                let elapsed = start.elapsed();
                println!("Database ping: {:?}", elapsed);
                println!("Connection: OK");
            }
            DebugAction::Config => {
                println!("Configuration");
                println!("=============");
                println!("Database path: {}", db_path.display());
                println!("Verbosity: {}", cli.verbose);
                println!("Quiet mode: {}", cli.quiet);
                println!("JSON logging: {}", cli.log_json);
                println!("RUST_LOG: {}", std::env::var("RUST_LOG").unwrap_or_else(|_| "(not set)".to_string()));
            }
            DebugAction::Dump { target } => {
                match target.as_str() {
                    "agents" | "all" => {
                        let agents = db.list_agents().await?;
                        println!("=== Agents ({}) ===", agents.len());
                        for agent in &agents {
                            println!("{:#?}", agent);
                        }
                        if target == "agents" { return Ok(()); }
                    }
                    _ => {}
                }
                match target.as_str() {
                    "prs" | "all" => {
                        let prs = db.get_pending_prs().await?;
                        println!("=== PRs ({}) ===", prs.len());
                        for pr in &prs {
                            println!("{:#?}", pr);
                        }
                        if target == "prs" { return Ok(()); }
                    }
                    _ => {}
                }
                match target.as_str() {
                    "epics" | "all" => {
                        let epics = db.get_pending_epics().await?;
                        println!("=== Epics ({}) ===", epics.len());
                        for epic in &epics {
                            println!("{:#?}", epic);
                        }
                        if target == "epics" { return Ok(()); }
                    }
                    _ => {}
                }
                if !["agents", "prs", "epics", "all"].contains(&target.as_str()) {
                    anyhow::bail!("Unknown dump target: {}. Use: agents, prs, epics, all", target);
                }
            }
        },

        Commands::Instructions { action } => match action {
            InstructionAction::List { enabled_only, learned_only } => {
                let source = if learned_only {
                    Some(orchestrate_core::InstructionSource::Learned)
                } else {
                    None
                };
                let instructions = db.list_instructions(enabled_only, None, source).await?;

                if instructions.is_empty() {
                    println!("No instructions found");
                    return Ok(());
                }

                println!("{:<6} {:<25} {:<10} {:<8} {:<10} {}", "ID", "NAME", "SCOPE", "ENABLED", "SOURCE", "CONTENT");
                println!("{}", "-".repeat(100));
                for inst in instructions {
                    let content_preview = if inst.content.len() > 40 {
                        format!("{}...", &inst.content[..37])
                    } else {
                        inst.content.clone()
                    };
                    println!(
                        "{:<6} {:<25} {:<10} {:<8} {:<10} {}",
                        inst.id,
                        if inst.name.len() > 25 { format!("{}...", &inst.name[..22]) } else { inst.name },
                        inst.scope.as_str(),
                        if inst.enabled { "yes" } else { "no" },
                        inst.source.as_str(),
                        content_preview
                    );
                }
            }
            InstructionAction::Show { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                println!("Instruction: {}", instruction.name);
                println!("{}", "=".repeat(40));
                println!("ID: {}", instruction.id);
                println!("Scope: {}", instruction.scope.as_str());
                if let Some(agent_type) = instruction.agent_type {
                    println!("Agent Type: {}", agent_type.as_str());
                }
                println!("Priority: {}", instruction.priority);
                println!("Enabled: {}", instruction.enabled);
                println!("Source: {}", instruction.source.as_str());
                println!("Confidence: {:.2}", instruction.confidence);
                if !instruction.tags.is_empty() {
                    println!("Tags: {}", instruction.tags.join(", "));
                }
                println!("Created: {}", instruction.created_at);
                println!("Updated: {}", instruction.updated_at);
                if let Some(ref created_by) = instruction.created_by {
                    println!("Created By: {}", created_by);
                }
                println!();
                println!("Content:");
                println!("{}", "-".repeat(40));
                println!("{}", instruction.content);
            }
            InstructionAction::Create { name, content, scope, agent_type, priority } => {
                let instruction = if scope == "agent_type" {
                    let agent_type = agent_type
                        .ok_or_else(|| anyhow::anyhow!("--agent-type required for scope=agent_type"))?;
                    let agent_type = parse_agent_type(&agent_type)?;
                    CustomInstruction::for_agent_type(&name, &content, agent_type)
                        .with_priority(priority)
                } else {
                    CustomInstruction::global(&name, &content)
                        .with_priority(priority)
                };

                let id = db.insert_instruction(&instruction).await?;
                println!("Created instruction: {} (ID: {})", name, id);
            }
            InstructionAction::Enable { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                db.set_instruction_enabled(instruction.id, true).await?;
                println!("Enabled instruction: {} (ID: {})", instruction.name, instruction.id);
            }
            InstructionAction::Disable { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                db.set_instruction_enabled(instruction.id, false).await?;
                println!("Disabled instruction: {} (ID: {})", instruction.name, instruction.id);
            }
            InstructionAction::Delete { id_or_name, force } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;

                if !force {
                    print!("Delete instruction '{}' (ID: {})? [y/N] ", instruction.name, instruction.id);
                    use std::io::{self, Write};
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    if !input.trim().eq_ignore_ascii_case("y") {
                        println!("Aborted");
                        return Ok(());
                    }
                }

                db.delete_instruction(instruction.id).await?;
                println!("Deleted instruction: {} (ID: {})", instruction.name, instruction.id);
            }
            InstructionAction::Stats { id_or_name } => {
                if let Some(ref id_or_name) = id_or_name {
                    let instruction = get_instruction_by_id_or_name(&db, id_or_name).await?;
                    if let Some(eff) = db.get_instruction_effectiveness(instruction.id).await? {
                        println!("Instruction: {} (ID: {})", instruction.name, instruction.id);
                        println!("{}", "-".repeat(40));
                        println!("Usage count: {}", eff.usage_count);
                        println!("Success count: {}", eff.success_count);
                        println!("Failure count: {}", eff.failure_count);
                        println!("Success rate: {:.1}%", eff.success_rate * 100.0);
                        println!("Penalty score: {:.2}", eff.penalty_score);
                        if let Some(time) = eff.avg_completion_time {
                            println!("Avg completion time: {:.1}s", time);
                        }
                        if let Some(ref dt) = eff.last_success_at {
                            println!("Last success: {}", dt);
                        }
                        if let Some(ref dt) = eff.last_failure_at {
                            println!("Last failure: {}", dt);
                        }
                    } else {
                        println!("No effectiveness data for instruction: {}", instruction.name);
                    }
                } else {
                    let instructions = db.list_instructions(false, None, None).await?;
                    if instructions.is_empty() {
                        println!("No instructions found");
                        return Ok(());
                    }

                    println!("{:<6} {:<25} {:<8} {:<8} {:<8} {:<10}", "ID", "NAME", "USAGE", "SUCCESS", "FAILURE", "PENALTY");
                    println!("{}", "-".repeat(80));
                    for inst in instructions {
                        if let Some(eff) = db.get_instruction_effectiveness(inst.id).await? {
                            println!(
                                "{:<6} {:<25} {:<8} {:<8} {:<8} {:<10.2}",
                                inst.id,
                                if inst.name.len() > 25 { format!("{}...", &inst.name[..22]) } else { inst.name },
                                eff.usage_count,
                                eff.success_count,
                                eff.failure_count,
                                eff.penalty_score
                            );
                        }
                    }
                }
            }
        },

        Commands::Learn { action } => match action {
            LearnAction::Patterns { pending_only } => {
                let status = if pending_only {
                    Some(PatternStatus::Observed)
                } else {
                    None
                };
                let patterns = db.list_patterns(status).await?;

                if patterns.is_empty() {
                    println!("No patterns found");
                    return Ok(());
                }

                println!("{:<6} {:<20} {:<15} {:<8} {:<15}", "ID", "TYPE", "AGENT_TYPE", "COUNT", "STATUS");
                println!("{}", "-".repeat(80));
                for pattern in patterns {
                    println!(
                        "{:<6} {:<20} {:<15} {:<8} {:<15}",
                        pattern.id,
                        pattern.pattern_type.as_str(),
                        pattern.agent_type.map(|t| t.as_str().to_string()).unwrap_or_else(|| "global".to_string()),
                        pattern.occurrence_count,
                        pattern.status.as_str()
                    );
                }
            }
            LearnAction::Approve { pattern_id } => {
                let pattern = db.get_pattern(pattern_id).await?
                    .ok_or_else(|| anyhow::anyhow!("Pattern not found: {}", pattern_id))?;

                let engine = LearningEngine::new();
                let instruction = engine.generate_instruction_from_pattern(&pattern)
                    .ok_or_else(|| anyhow::anyhow!("Could not generate instruction from pattern"))?;

                let instruction_id = db.insert_instruction(&instruction).await?;
                db.update_pattern_status(pattern_id, PatternStatus::Approved, Some(instruction_id)).await?;

                println!("Approved pattern {} and created instruction {}", pattern_id, instruction_id);
            }
            LearnAction::Reject { pattern_id } => {
                let _ = db.get_pattern(pattern_id).await?
                    .ok_or_else(|| anyhow::anyhow!("Pattern not found: {}", pattern_id))?;

                db.update_pattern_status(pattern_id, PatternStatus::Rejected, None).await?;
                println!("Rejected pattern {}", pattern_id);
            }
            LearnAction::Analyze => {
                let engine = LearningEngine::new();
                let created = engine.process_patterns(&db).await?;

                if created.is_empty() {
                    println!("No new instructions created");
                } else {
                    println!("Created {} instructions:", created.len());
                    for inst in created {
                        println!("  - {} (ID: {})", inst.name, inst.id);
                    }
                }
            }
            LearnAction::Config => {
                let config = orchestrate_core::LearningConfig::default();
                println!("Learning Configuration");
                println!("{}", "=".repeat(40));
                println!("Min occurrences: {}", config.min_occurrences);
                println!("Auto-approve threshold: {}", config.auto_approve_threshold);
                println!("Auto-enable: {}", config.auto_enable);
                println!("Penalty disable threshold: {}", config.penalty_disable_threshold);
                println!("Min usage for deletion: {}", config.min_usage_for_deletion);
                println!("Deletion success rate threshold: {}", config.deletion_success_rate_threshold);
                println!("Enabled pattern types: {:?}", config.enabled_pattern_types);
            }
            LearnAction::Cleanup => {
                let engine = LearningEngine::new();
                let result = engine.cleanup(&db).await?;

                println!("Cleanup Results");
                println!("{}", "=".repeat(40));
                println!("Instructions disabled: {}", result.disabled_count);
                println!("Instructions deleted: {}", result.deleted_names.len());
                if !result.deleted_names.is_empty() {
                    println!("Deleted: {}", result.deleted_names.join(", "));
                }
            }
            LearnAction::ResetPenalty { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                db.reset_penalty(instruction.id).await?;
                println!("Reset penalty for instruction: {} (ID: {})", instruction.name, instruction.id);
            }
        },
    }

    Ok(())
}

async fn get_instruction_by_id_or_name(db: &Database, id_or_name: &str) -> Result<CustomInstruction> {
    // Try parsing as ID first
    if let Ok(id) = id_or_name.parse::<i64>() {
        if let Some(inst) = db.get_instruction(id).await? {
            return Ok(inst);
        }
    }

    // Try as name
    if let Some(inst) = db.get_instruction_by_name(id_or_name).await? {
        return Ok(inst);
    }

    anyhow::bail!("Instruction not found: {}", id_or_name)
}

fn parse_agent_type(s: &str) -> Result<AgentType> {
    match s.to_lowercase().as_str() {
        "story-developer" | "storydeveloper" => Ok(AgentType::StoryDeveloper),
        "code-reviewer" | "codereviewer" => Ok(AgentType::CodeReviewer),
        "issue-fixer" | "issuefixer" => Ok(AgentType::IssueFixer),
        "explorer" => Ok(AgentType::Explorer),
        "bmad-orchestrator" | "bmadorchestrator" => Ok(AgentType::BmadOrchestrator),
        "bmad-planner" | "bmadplanner" => Ok(AgentType::BmadPlanner),
        "pr-shepherd" | "prshepherd" => Ok(AgentType::PrShepherd),
        "pr-controller" | "prcontroller" => Ok(AgentType::PrController),
        "conflict-resolver" | "conflictresolver" => Ok(AgentType::ConflictResolver),
        _ => anyhow::bail!("Unknown agent type: {}", s),
    }
}
