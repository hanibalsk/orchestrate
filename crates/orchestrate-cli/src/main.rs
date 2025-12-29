//! Orchestrate CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchestrate_claude::{AgentLoop, ClaudeCliClient, ClaudeClient};
use orchestrate_core::{
    Agent, AgentState, AgentType, CustomInstruction, Database, Epic, EpicStatus,
    LearningEngine, PatternStatus, Schedule, ScheduleRun, ShellState, Story, StoryStatus, Worktree,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{error, info, warn, Level};
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

    let filter =
        EnvFilter::from_default_env().add_directive(format!("orchestrate={}", level).parse()?);

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbose >= 2) // Show module path at debug+
        .with_file(verbose >= 3) // Show file:line at trace
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
    #[arg(
        long,
        env = "ORCHESTRATE_DB_PATH",
        default_value = "~/.orchestrate/orchestrate.db"
    )]
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
    /// Story management
    Story {
        #[command(subcommand)]
        action: StoryAction,
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
    /// Review agent history and past actions
    History {
        #[command(subcommand)]
        action: HistoryAction,
    },
    /// Token usage and cost tracking
    Tokens {
        #[command(subcommand)]
        action: TokensAction,
    },
    /// Schedule management
    Schedule {
        #[command(subcommand)]
        action: ScheduleAction,
    },
}

#[derive(Subcommand)]
enum DaemonAction {
    /// Start the daemon
    Start {
        /// Port for web API (0 to disable)
        #[arg(short, long, default_value = "8080")]
        port: u16,
        /// Maximum concurrent agents
        #[arg(short = 'c', long, default_value = "3")]
        max_concurrent: usize,
        /// Poll interval in seconds
        #[arg(short = 'i', long, default_value = "5")]
        poll_interval: u64,
        /// Claude model to use
        #[arg(short, long, default_value = "sonnet")]
        model: String,
        /// Use claude CLI instead of direct API (uses OAuth auth)
        #[arg(long)]
        use_cli: bool,
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
    Show { id: String },
    /// Pause an agent
    Pause { id: String },
    /// Resume an agent
    Resume { id: String },
    /// Terminate an agent
    Terminate { id: String },
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
    Remove { name: String },
}

#[derive(Subcommand)]
enum BmadAction {
    /// Process epics from docs/bmad/epics/
    Process {
        /// Pattern to match epic files (e.g., "epic-001-*")
        pattern: Option<String>,
        /// Epics directory (default: docs/bmad/epics)
        #[arg(short, long, default_value = "docs/bmad/epics")]
        dir: PathBuf,
        /// Dry run - show what would be done without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Show BMAD status for all epics
    Status,
    /// Reset BMAD state (clear all epics and stories)
    Reset {
        /// Force reset without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum StoryAction {
    /// List all stories
    List {
        /// Filter by epic ID
        #[arg(short, long)]
        epic: Option<String>,
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Show story details
    Show {
        /// Story ID (e.g., epic-001.1)
        id: String,
    },
    /// Create a new story
    Create {
        /// Epic ID
        #[arg(short, long)]
        epic_id: String,
        /// Story title
        #[arg(short, long)]
        title: String,
        /// Story description
        #[arg(short, long)]
        description: Option<String>,
    },
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

#[derive(Subcommand)]
enum HistoryAction {
    /// List agents with pagination and filters
    Agents {
        /// Filter by state (running, paused, completed, terminated)
        #[arg(short, long)]
        state: Option<String>,
        /// Filter by agent type
        #[arg(short = 't', long)]
        agent_type: Option<String>,
        /// Number of results to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: i64,
    },
    /// Show messages for an agent
    Messages {
        /// Agent ID
        agent_id: String,
        /// Number of messages to show
        #[arg(short, long, default_value = "50")]
        limit: i64,
        /// Offset for pagination
        #[arg(short, long, default_value = "0")]
        offset: i64,
        /// Show full message content
        #[arg(long)]
        full: bool,
    },
    /// Show agent statistics
    Stats {
        /// Agent ID
        agent_id: String,
    },
    /// Show tool errors for an agent
    Errors {
        /// Agent ID
        agent_id: String,
        /// Number of errors to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
    },
    /// Show a summary of recent agent activity
    Summary {
        /// Number of recent agents to analyze
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },
}

#[derive(Subcommand)]
enum TokensAction {
    /// Show daily token usage and costs
    Daily {
        /// Number of days to show (default: 7)
        #[arg(short, long, default_value = "7")]
        days: i32,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show token stats for a specific agent
    Agent {
        /// Agent ID
        agent_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show token stats for a session
    Session {
        /// Session ID
        session_id: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show overall token usage summary
    Summary {
        /// Number of days to analyze
        #[arg(short, long, default_value = "30")]
        days: i32,
    },
}

#[derive(Subcommand)]
enum ScheduleAction {
    /// Add a new schedule
    Add {
        /// Schedule name
        #[arg(short, long)]
        name: String,
        /// Cron expression
        #[arg(short, long)]
        cron: String,
        /// Agent type
        #[arg(short, long)]
        agent: String,
        /// Task description
        #[arg(short, long)]
        task: String,
    },
    /// List all schedules
    List,
    /// Show schedule details
    Show {
        /// Schedule name
        name: String,
    },
    /// Pause a schedule
    Pause {
        /// Schedule name
        name: String,
    },
    /// Resume a schedule
    Resume {
        /// Schedule name
        name: String,
    },
    /// Delete a schedule
    Delete {
        /// Schedule name
        name: String,
    },
    /// Run a schedule immediately
    RunNow {
        /// Schedule name
        name: String,
    },
    /// Show schedule execution history
    History {
        /// Schedule name
        name: String,
        /// Number of runs to show
        #[arg(short, long, default_value = "10")]
        limit: i64,
    },
    /// Add a schedule from a built-in template
    AddTemplate {
        /// Template name (security-scan, dependency-check, code-quality, documentation-check, database-backup)
        template_name: String,
        /// Optional custom schedule name (defaults to template name)
        #[arg(short, long)]
        name: Option<String>,
    },
    /// List available schedule templates
    ListTemplates,
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
            DaemonAction::Start {
                port,
                max_concurrent,
                poll_interval,
                model,
                use_cli,
            } => {
                run_daemon(db, port, max_concurrent, poll_interval, model, use_cli).await?;
            }
            DaemonAction::Stop => {
                println!("Stopping daemon...");
                // TODO: Implement daemon stop via signal/file
                println!("Note: Use Ctrl+C to stop the running daemon");
            }
            DaemonAction::Status => {
                println!("Daemon status: Check if process is running");
                // TODO: Implement status check via PID file
            }
        },

        Commands::Agent { action } => match action {
            AgentAction::Spawn {
                agent_type,
                task,
                worktree,
            } => {
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
            PrAction::Create {
                worktree: _,
                title: _,
            } => {
                println!("Creating PR... (not implemented)");
            }
            PrAction::Merge { number, strategy } => {
                println!("Merging PR #{} with {} strategy...", number, strategy);
                // TODO: Implement merge
            }
            PrAction::Queue => {
                // Read from shell state file for compatibility
                let shell_state = ShellState::new(".");
                let queue = shell_state.queue_list().unwrap_or_default();
                let current_pr = shell_state.current_pr().unwrap_or(None);

                println!("=== PR Queue ({} items) ===", queue.len());
                if queue.is_empty() {
                    println!("  (empty)");
                } else {
                    for (i, entry) in queue.iter().enumerate() {
                        println!("  {}. {} - {}", i + 1, entry.worktree, entry.title);
                    }
                }
                println!();
                println!("=== Current PR ===");
                if let Some(pr_num) = current_pr {
                    println!("  PR #{} (checking status...)", pr_num);
                    // Try to get more info from gh
                    if let Ok(output) = std::process::Command::new("gh")
                        .args([
                            "pr",
                            "view",
                            &pr_num.to_string(),
                            "--json",
                            "title,state,url",
                        ])
                        .output()
                    {
                        if output.status.success() {
                            if let Ok(json) =
                                serde_json::from_slice::<serde_json::Value>(&output.stdout)
                            {
                                println!("  Title: {}", json["title"].as_str().unwrap_or("-"));
                                println!("  State: {}", json["state"].as_str().unwrap_or("-"));
                                println!("  URL: {}", json["url"].as_str().unwrap_or("-"));
                            }
                        }
                    }
                } else {
                    println!("  (none)");
                }

                // Also show database PRs for reference
                let db_prs = db.get_pending_prs().await?;
                if !db_prs.is_empty() {
                    println!();
                    println!("=== Database PRs ===");
                    for pr in db_prs {
                        println!("  - {} ({:?})", pr.branch_name, pr.status);
                    }
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
            BmadAction::Process {
                pattern,
                dir,
                dry_run,
            } => {
                process_bmad_epics(&db, &dir, pattern.as_deref(), dry_run).await?;
            }
            BmadAction::Status => {
                show_bmad_status(&db).await?;
            }
            BmadAction::Reset { force } => {
                reset_bmad_state(&db, force).await?;
            }
        },

        Commands::Story { action } => match action {
            StoryAction::List { epic, status } => {
                let status_filter = status.as_ref().map(|s| parse_story_status(s)).transpose()?;
                let epic_filter = epic.as_deref();

                let stories = if let Some(epic_id) = epic_filter {
                    db.get_stories_for_epic(epic_id).await?
                } else {
                    db.list_stories(None).await?
                };

                // Filter by status
                let stories: Vec<_> = if let Some(filter_status) = status_filter {
                    stories
                        .into_iter()
                        .filter(|s| s.status == filter_status)
                        .collect()
                } else {
                    stories
                };

                if stories.is_empty() {
                    println!("No stories found");
                    return Ok(());
                }

                println!("{:<20} {:<15} {:<15} {}", "ID", "EPIC", "STATUS", "TITLE");
                println!("{}", "-".repeat(100));
                for story in stories {
                    println!(
                        "{:<20} {:<15} {:<15} {}",
                        story.id,
                        story.epic_id,
                        format!("{:?}", story.status),
                        &story.title[..story.title.len().min(40)]
                    );
                }
            }
            StoryAction::Show { id } => {
                show_story(&db, &id).await?;
            }
            StoryAction::Create {
                epic_id,
                title,
                description,
            } => {
                // Generate story ID
                let stories = db.get_stories_for_epic(&epic_id).await?;
                let story_num = stories.len() + 1;
                let story_id = format!("{}.{}", epic_id, story_num);

                // Create story
                let mut story = Story::new(&story_id, &epic_id, &title);
                story.description = description.clone();

                // Save to database
                db.upsert_story(&story).await?;

                println!("✓ Story created: {}", story_id);
                println!("  Epic: {}", epic_id);
                println!("  Title: {}", title);
                if let Some(ref desc) = description {
                    println!("  Description: {}", desc);
                }
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

            // Get shell state
            let shell_state = ShellState::new(".");
            let queue = shell_state.queue_list().unwrap_or_default();
            let current_pr = shell_state.current_pr().unwrap_or(None);
            let shepherd_locks = shell_state.shepherd_locks().unwrap_or_default();
            let active_shepherds: Vec<_> = shepherd_locks.iter().filter(|l| l.is_active).collect();

            if json {
                println!(
                    r#"{{"total_agents":{},"running":{},"paused":{},"queue_size":{},"current_pr":{},"active_shepherds":{}}}"#,
                    agents.len(),
                    running,
                    paused,
                    queue.len(),
                    current_pr
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "null".to_string()),
                    active_shepherds.len()
                );
            } else {
                println!("╔══════════════════════════════════════════════════╗");
                println!("║              ORCHESTRATE STATUS                  ║");
                println!("╠══════════════════════════════════════════════════╣");
                println!("║ Database Agents                                  ║");
                println!(
                    "║   Total: {:3}  Running: {:3}  Paused: {:3}         ║",
                    agents.len(),
                    running,
                    paused
                );
                println!("╠══════════════════════════════════════════════════╣");
                println!("║ PR Queue                                         ║");
                println!(
                    "║   Queued: {:3}                                    ║",
                    queue.len()
                );
                if let Some(pr_num) = current_pr {
                    println!(
                        "║   Current PR: #{}                                ║",
                        pr_num
                    );
                } else {
                    println!("║   Current PR: (none)                             ║");
                }
                println!("╠══════════════════════════════════════════════════╣");
                println!("║ Active Shepherds                                 ║");
                if active_shepherds.is_empty() {
                    println!("║   (none)                                         ║");
                } else {
                    for lock in &active_shepherds {
                        println!(
                            "║   PR #{} (PID: {})                            ║",
                            lock.pr_number, lock.pid
                        );
                    }
                }
                println!("╚══════════════════════════════════════════════════╝");
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
                println!(
                    "RUST_LOG: {}",
                    std::env::var("RUST_LOG").unwrap_or_else(|_| "(not set)".to_string())
                );
            }
            DebugAction::Dump { target } => {
                match target.as_str() {
                    "agents" | "all" => {
                        let agents = db.list_agents().await?;
                        println!("=== Agents ({}) ===", agents.len());
                        for agent in &agents {
                            println!("{:#?}", agent);
                        }
                        if target == "agents" {
                            return Ok(());
                        }
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
                        if target == "prs" {
                            return Ok(());
                        }
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
                        if target == "epics" {
                            return Ok(());
                        }
                    }
                    _ => {}
                }
                if !["agents", "prs", "epics", "all"].contains(&target.as_str()) {
                    anyhow::bail!(
                        "Unknown dump target: {}. Use: agents, prs, epics, all",
                        target
                    );
                }
            }
        },

        Commands::Instructions { action } => match action {
            InstructionAction::List {
                enabled_only,
                learned_only,
            } => {
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

                println!(
                    "{:<6} {:<25} {:<10} {:<8} {:<10} {}",
                    "ID", "NAME", "SCOPE", "ENABLED", "SOURCE", "CONTENT"
                );
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
                        if inst.name.len() > 25 {
                            format!("{}...", &inst.name[..22])
                        } else {
                            inst.name
                        },
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
            InstructionAction::Create {
                name,
                content,
                scope,
                agent_type,
                priority,
            } => {
                let instruction = if scope == "agent_type" {
                    let agent_type = agent_type.ok_or_else(|| {
                        anyhow::anyhow!("--agent-type required for scope=agent_type")
                    })?;
                    let agent_type = parse_agent_type(&agent_type)?;
                    CustomInstruction::for_agent_type(&name, &content, agent_type)
                        .with_priority(priority)
                } else {
                    CustomInstruction::global(&name, &content).with_priority(priority)
                };

                let id = db.insert_instruction(&instruction).await?;
                println!("Created instruction: {} (ID: {})", name, id);
            }
            InstructionAction::Enable { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                db.set_instruction_enabled(instruction.id, true).await?;
                println!(
                    "Enabled instruction: {} (ID: {})",
                    instruction.name, instruction.id
                );
            }
            InstructionAction::Disable { id_or_name } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;
                db.set_instruction_enabled(instruction.id, false).await?;
                println!(
                    "Disabled instruction: {} (ID: {})",
                    instruction.name, instruction.id
                );
            }
            InstructionAction::Delete { id_or_name, force } => {
                let instruction = get_instruction_by_id_or_name(&db, &id_or_name).await?;

                if !force {
                    print!(
                        "Delete instruction '{}' (ID: {})? [y/N] ",
                        instruction.name, instruction.id
                    );
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
                println!(
                    "Deleted instruction: {} (ID: {})",
                    instruction.name, instruction.id
                );
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
                        println!(
                            "No effectiveness data for instruction: {}",
                            instruction.name
                        );
                    }
                } else {
                    let instructions = db.list_instructions(false, None, None).await?;
                    if instructions.is_empty() {
                        println!("No instructions found");
                        return Ok(());
                    }

                    println!(
                        "{:<6} {:<25} {:<8} {:<8} {:<8} {:<10}",
                        "ID", "NAME", "USAGE", "SUCCESS", "FAILURE", "PENALTY"
                    );
                    println!("{}", "-".repeat(80));
                    for inst in instructions {
                        if let Some(eff) = db.get_instruction_effectiveness(inst.id).await? {
                            println!(
                                "{:<6} {:<25} {:<8} {:<8} {:<8} {:<10.2}",
                                inst.id,
                                if inst.name.len() > 25 {
                                    format!("{}...", &inst.name[..22])
                                } else {
                                    inst.name
                                },
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

                println!(
                    "{:<6} {:<20} {:<15} {:<8} {:<15}",
                    "ID", "TYPE", "AGENT_TYPE", "COUNT", "STATUS"
                );
                println!("{}", "-".repeat(80));
                for pattern in patterns {
                    println!(
                        "{:<6} {:<20} {:<15} {:<8} {:<15}",
                        pattern.id,
                        pattern.pattern_type.as_str(),
                        pattern
                            .agent_type
                            .map(|t| t.as_str().to_string())
                            .unwrap_or_else(|| "global".to_string()),
                        pattern.occurrence_count,
                        pattern.status.as_str()
                    );
                }
            }
            LearnAction::Approve { pattern_id } => {
                let pattern = db
                    .get_pattern(pattern_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Pattern not found: {}", pattern_id))?;

                let engine = LearningEngine::new();
                let instruction = engine
                    .generate_instruction_from_pattern(&pattern)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Could not generate instruction from pattern")
                    })?;

                let instruction_id = db.insert_instruction(&instruction).await?;
                db.update_pattern_status(pattern_id, PatternStatus::Approved, Some(instruction_id))
                    .await?;

                println!(
                    "Approved pattern {} and created instruction {}",
                    pattern_id, instruction_id
                );
            }
            LearnAction::Reject { pattern_id } => {
                let _ = db
                    .get_pattern(pattern_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("Pattern not found: {}", pattern_id))?;

                db.update_pattern_status(pattern_id, PatternStatus::Rejected, None)
                    .await?;
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
                println!(
                    "Penalty disable threshold: {}",
                    config.penalty_disable_threshold
                );
                println!("Min usage for deletion: {}", config.min_usage_for_deletion);
                println!(
                    "Deletion success rate threshold: {}",
                    config.deletion_success_rate_threshold
                );
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
                println!(
                    "Reset penalty for instruction: {} (ID: {})",
                    instruction.name, instruction.id
                );
            }
        },

        Commands::History { action } => match action {
            HistoryAction::Agents {
                state,
                agent_type,
                limit,
                offset,
            } => {
                let state_filter = state.as_ref().map(|s| parse_agent_state(s)).transpose()?;
                let type_filter = agent_type
                    .as_ref()
                    .map(|t| parse_agent_type(t))
                    .transpose()?;

                let agents = db
                    .list_agents_paginated(limit, offset, state_filter, type_filter)
                    .await?;
                let total = db.count_agents().await?;

                if agents.is_empty() {
                    println!("No agents found");
                    return Ok(());
                }

                println!(
                    "Showing {} of {} agents (offset {})",
                    agents.len(),
                    total,
                    offset
                );
                println!();
                println!(
                    "{:<36} {:<18} {:<12} {:<20}",
                    "ID", "TYPE", "STATE", "CREATED"
                );
                println!("{}", "-".repeat(90));
                for agent in agents {
                    println!(
                        "{:<36} {:<18} {:<12} {:<20}",
                        agent.id,
                        agent.agent_type.as_str(),
                        format!("{:?}", agent.state),
                        agent.created_at.format("%Y-%m-%d %H:%M:%S")
                    );
                }
                println!();
                if offset + limit < total {
                    println!("Use --offset {} to see more", offset + limit);
                }
            }
            HistoryAction::Messages {
                agent_id,
                limit,
                offset,
                full,
            } => {
                let uuid = uuid::Uuid::parse_str(&agent_id)?;
                let messages = db.get_messages_paginated(uuid, limit, offset).await?;
                let total = db.count_messages(uuid).await?;

                if messages.is_empty() {
                    println!("No messages found for agent {}", agent_id);
                    return Ok(());
                }

                // Get agent info
                if let Some(agent) = db.get_agent(uuid).await? {
                    println!("Agent: {} ({:?})", agent.id, agent.agent_type);
                    println!("Task: {}", &agent.task[..agent.task.len().min(80)]);
                    println!();
                }

                println!(
                    "Showing {} of {} messages (offset {})",
                    messages.len(),
                    total,
                    offset
                );
                println!();

                for (i, msg) in messages.iter().enumerate() {
                    let role = match msg.role {
                        orchestrate_core::MessageRole::User => "USER",
                        orchestrate_core::MessageRole::Assistant => "ASST",
                        orchestrate_core::MessageRole::System => "SYS",
                        orchestrate_core::MessageRole::Tool => "TOOL",
                    };

                    let content = if full {
                        msg.content.clone()
                    } else {
                        // Truncate content for display
                        let first_line = msg.content.lines().next().unwrap_or(&msg.content);
                        if first_line.len() > 100 {
                            format!("{}...", &first_line[..97])
                        } else if msg.content.lines().count() > 1 {
                            format!("{}...", first_line)
                        } else {
                            first_line.to_string()
                        }
                    };

                    let tokens = if msg.input_tokens > 0 || msg.output_tokens > 0 {
                        format!(" [in:{} out:{}]", msg.input_tokens, msg.output_tokens)
                    } else {
                        String::new()
                    };

                    println!("[{}] {:4}{}", role, i + offset as usize + 1, tokens);
                    if full {
                        println!("{}", content);
                        println!("{}", "-".repeat(60));
                    } else {
                        println!("  {}", content);
                    }
                }

                println!();
                if offset + limit < total {
                    println!("Use --offset {} to see more", offset + limit);
                }
            }
            HistoryAction::Stats { agent_id } => {
                let uuid = uuid::Uuid::parse_str(&agent_id)?;

                // Get agent info
                if let Some(agent) = db.get_agent(uuid).await? {
                    println!("Agent Statistics");
                    println!("{}", "=".repeat(50));
                    println!("ID: {}", agent.id);
                    println!("Type: {:?}", agent.agent_type);
                    println!("State: {:?}", agent.state);
                    println!("Task: {}", agent.task);
                    println!(
                        "Worktree: {}",
                        agent.worktree_id.as_deref().unwrap_or("(none)")
                    );
                    println!("Created: {}", agent.created_at);
                    println!("Updated: {}", agent.updated_at);
                    println!();

                    let stats = db.get_agent_stats(uuid).await?;
                    println!("Message Statistics");
                    println!("{}", "-".repeat(50));
                    println!("Total messages: {}", stats.message_count);
                    println!("Input tokens: {}", stats.total_input_tokens);
                    println!("Output tokens: {}", stats.total_output_tokens);
                    println!("Total tokens: {}", stats.total_tokens);
                    println!("Tool calls: {}", stats.tool_call_count);
                    println!("Errors: {}", stats.error_count);
                    if let Some(ref first) = stats.first_message_at {
                        println!("First message: {}", first);
                    }
                    if let Some(ref last) = stats.last_message_at {
                        println!("Last message: {}", last);
                    }
                } else {
                    println!("Agent not found: {}", agent_id);
                }
            }
            HistoryAction::Errors { agent_id, limit } => {
                let uuid = uuid::Uuid::parse_str(&agent_id)?;
                let errors = db.get_tool_errors(uuid, limit).await?;

                if errors.is_empty() {
                    println!("No errors found for agent {}", agent_id);
                    return Ok(());
                }

                println!("Tool Errors for Agent {}", agent_id);
                println!("{}", "=".repeat(60));

                for (i, msg) in errors.iter().enumerate() {
                    println!("[{}] {}", i + 1, msg.created_at.format("%Y-%m-%d %H:%M:%S"));
                    // Extract error message from content
                    let content = if msg.content.len() > 200 {
                        format!("{}...", &msg.content[..197])
                    } else {
                        msg.content.clone()
                    };
                    println!("{}", content);
                    println!("{}", "-".repeat(60));
                }
            }
            HistoryAction::Summary { limit } => {
                let agents = db.list_agents_paginated(limit, 0, None, None).await?;

                if agents.is_empty() {
                    println!("No agents found");
                    return Ok(());
                }

                println!("╔══════════════════════════════════════════════════════════════════════════════╗");
                println!("║                         AGENT ACTIVITY SUMMARY                               ║");
                println!("╠══════════════════════════════════════════════════════════════════════════════╣");

                for agent in agents {
                    let stats = db.get_agent_stats(agent.id).await?;
                    let type_str = format!("{:?}", agent.agent_type);
                    let state_str = format!("{:?}", agent.state);

                    println!("║ {} ║", agent.id);
                    println!(
                        "║   Type: {:<15} State: {:<12} Msgs: {:<6} Tokens: {:<10} ║",
                        &type_str[..type_str.len().min(15)],
                        &state_str[..state_str.len().min(12)],
                        stats.message_count,
                        stats.total_tokens
                    );
                    println!(
                        "║   Task: {:<68} ║",
                        if agent.task.len() > 68 {
                            format!("{}...", &agent.task[..65])
                        } else {
                            agent.task.clone()
                        }
                    );
                    println!("╟──────────────────────────────────────────────────────────────────────────────╢");
                }

                println!("╚══════════════════════════════════════════════════════════════════════════════╝");
            }
        },

        Commands::Tokens { action } => match action {
            TokensAction::Daily { days, json } => {
                let usage = db.get_daily_token_usage(days).await?;

                if json {
                    println!("{}", serde_json::to_string_pretty(&usage)?);
                    return Ok(());
                }

                if usage.is_empty() {
                    println!("No token usage data found for the last {} days", days);
                    return Ok(());
                }

                println!("Daily Token Usage (Last {} Days)", days);
                println!("{}", "=".repeat(110));
                println!(
                    "{:<12} {:<25} {:>12} {:>12} {:>12} {:>10} {:>12}",
                    "DATE", "MODEL", "INPUT", "OUTPUT", "CACHE_READ", "REQUESTS", "EST. COST"
                );
                println!("{}", "-".repeat(110));

                let mut total_cost = 0.0;
                for day in &usage {
                    let cost_str = day
                        .estimated_cost_usd
                        .map(|c| format!("${:.4}", c))
                        .unwrap_or_else(|| "-".to_string());
                    if let Some(c) = day.estimated_cost_usd {
                        total_cost += c;
                    }

                    println!(
                        "{:<12} {:<25} {:>12} {:>12} {:>12} {:>10} {:>12}",
                        day.date,
                        &day.model[..day.model.len().min(25)],
                        format_tokens(day.total_input_tokens),
                        format_tokens(day.total_output_tokens),
                        format_tokens(day.total_cache_read_tokens),
                        day.request_count,
                        cost_str
                    );
                }

                println!("{}", "-".repeat(110));
                println!("{:>97} ${:.4}", "TOTAL:", total_cost);
            }
            TokensAction::Agent { agent_id, json } => {
                let uuid = uuid::Uuid::parse_str(&agent_id)?;
                let stats = db.get_agent_token_stats(uuid).await?;

                if json {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                    return Ok(());
                }

                // Get agent info
                if let Some(agent) = db.get_agent(uuid).await? {
                    println!("Token Statistics for Agent {}", agent_id);
                    println!("{}", "=".repeat(50));
                    println!("Type: {:?}", agent.agent_type);
                    println!("State: {:?}", agent.state);
                    println!(
                        "Task: {}",
                        if agent.task.len() > 60 {
                            format!("{}...", &agent.task[..57])
                        } else {
                            agent.task.clone()
                        }
                    );
                    println!();
                }

                println!("Token Usage");
                println!("{}", "-".repeat(50));
                println!("Turns:                  {:>12}", stats.turn_count);
                println!(
                    "Input tokens:           {:>12}",
                    format_tokens(stats.total_input_tokens)
                );
                println!(
                    "Output tokens:          {:>12}",
                    format_tokens(stats.total_output_tokens)
                );
                println!(
                    "Total tokens:           {:>12}",
                    format_tokens(stats.total_input_tokens + stats.total_output_tokens)
                );
                println!();
                println!("Cache Performance");
                println!("{}", "-".repeat(50));
                println!(
                    "Cache reads:            {:>12}",
                    format_tokens(stats.total_cache_read_tokens)
                );
                println!(
                    "Cache writes:           {:>12}",
                    format_tokens(stats.total_cache_write_tokens)
                );
                println!("Cache hit rate:         {:>11.1}%", stats.cache_hit_rate);
                println!();
                println!("Context Usage");
                println!("{}", "-".repeat(50));
                println!("Avg context used:       {:>12.0}", stats.avg_context_used);
                println!(
                    "Avg messages included:  {:>12.1}",
                    stats.avg_messages_included
                );
                println!(
                    "Messages summarized:    {:>12}",
                    stats.total_messages_summarized
                );
            }
            TokensAction::Session { session_id, json } => {
                let stats = db.get_session_token_stats(&session_id).await?;

                if json {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                    return Ok(());
                }

                println!("Token Statistics for Session {}", session_id);
                println!("{}", "=".repeat(50));
                println!("Turns:                  {:>12}", stats.turn_count);
                println!(
                    "Input tokens:           {:>12}",
                    format_tokens(stats.total_input_tokens)
                );
                println!(
                    "Output tokens:          {:>12}",
                    format_tokens(stats.total_output_tokens)
                );
                println!(
                    "Total tokens:           {:>12}",
                    format_tokens(stats.total_input_tokens + stats.total_output_tokens)
                );
                println!();
                println!("Cache Performance");
                println!("{}", "-".repeat(50));
                println!(
                    "Cache reads:            {:>12}",
                    format_tokens(stats.total_cache_read_tokens)
                );
                println!(
                    "Cache writes:           {:>12}",
                    format_tokens(stats.total_cache_write_tokens)
                );
                println!("Cache hit rate:         {:>11.1}%", stats.cache_hit_rate);
            }
            TokensAction::Summary { days } => {
                let usage = db.get_daily_token_usage(days).await?;

                if usage.is_empty() {
                    println!("No token usage data found for the last {} days", days);
                    return Ok(());
                }

                let total_input: i64 = usage.iter().map(|d| d.total_input_tokens).sum();
                let total_output: i64 = usage.iter().map(|d| d.total_output_tokens).sum();
                let total_cache_read: i64 = usage.iter().map(|d| d.total_cache_read_tokens).sum();
                let total_cache_write: i64 = usage.iter().map(|d| d.total_cache_write_tokens).sum();
                let total_requests: i64 = usage.iter().map(|d| d.request_count).sum();
                let total_cost: f64 = usage.iter().filter_map(|d| d.estimated_cost_usd).sum();

                let cache_hit_rate = if total_input > 0 {
                    (total_cache_read as f64 / total_input as f64) * 100.0
                } else {
                    0.0
                };

                println!("╔══════════════════════════════════════════════════════════════╗");
                println!(
                    "║               TOKEN USAGE SUMMARY ({} Days)                ║",
                    days
                );
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║  Total Tokens                                                ║");
                println!(
                    "║    Input:            {:>20}                    ║",
                    format_tokens(total_input)
                );
                println!(
                    "║    Output:           {:>20}                    ║",
                    format_tokens(total_output)
                );
                println!(
                    "║    Combined:         {:>20}                    ║",
                    format_tokens(total_input + total_output)
                );
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║  Cache Performance                                           ║");
                println!(
                    "║    Cache reads:      {:>20}                    ║",
                    format_tokens(total_cache_read)
                );
                println!(
                    "║    Cache writes:     {:>20}                    ║",
                    format_tokens(total_cache_write)
                );
                println!(
                    "║    Hit rate:         {:>19.1}%                    ║",
                    cache_hit_rate
                );
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║  Activity                                                    ║");
                println!(
                    "║    Total requests:   {:>20}                    ║",
                    total_requests
                );
                println!(
                    "║    Days with usage:  {:>20}                    ║",
                    usage.len()
                );
                println!("╠══════════════════════════════════════════════════════════════╣");
                println!("║  Estimated Cost                                              ║");
                println!(
                    "║    Total:            {:>19}                     ║",
                    format!("${:.4}", total_cost)
                );
                println!(
                    "║    Avg per day:      {:>19}                     ║",
                    format!("${:.4}", total_cost / usage.len() as f64)
                );
                println!("╚══════════════════════════════════════════════════════════════╝");
            }
        },

        Commands::Schedule { action } => match action {
            ScheduleAction::Add {
                name,
                cron,
                agent,
                task,
            } => {
                // Create and validate schedule
                let mut schedule = Schedule::new(name.clone(), cron.clone(), agent.clone(), task.clone());

                // Validate cron expression
                if let Err(e) = schedule.validate_cron() {
                    anyhow::bail!("Invalid cron expression: {}", e);
                }

                // Check if schedule with this name already exists
                if db.get_schedule_by_name(&name).await?.is_some() {
                    anyhow::bail!("Schedule '{}' already exists", name);
                }

                // Calculate next run
                schedule.update_next_run()?;

                // Insert into database
                let id = db.insert_schedule(&schedule).await?;

                println!("Schedule '{}' added successfully (ID: {})", name, id);
                if let Some(next_run) = schedule.next_run {
                    println!("Next run: {}", next_run.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }

            ScheduleAction::List => {
                let schedules = db.list_schedules(false).await?;

                if schedules.is_empty() {
                    println!("No schedules found");
                    return Ok(());
                }

                println!("{:<20} {:<15} {:<20} {:<10} {:<25}", "NAME", "CRON", "AGENT", "STATUS", "NEXT RUN");
                println!("{}", "-".repeat(100));

                for schedule in schedules {
                    let status = if schedule.enabled { "enabled" } else { "disabled" };
                    let next_run = schedule.next_run
                        .map(|nr| nr.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<20} {:<15} {:<20} {:<10} {:<25}",
                        schedule.name,
                        schedule.cron_expression,
                        schedule.agent_type,
                        status,
                        next_run
                    );
                }
            }

            ScheduleAction::Show { name } => {
                let schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                println!("Schedule: {}", schedule.name);
                println!("Cron: {}", schedule.cron_expression);
                println!("Agent: {}", schedule.agent_type);
                println!("Task: {}", schedule.task);
                println!("Enabled: {}", schedule.enabled);
                println!("Created: {}", schedule.created_at.format("%Y-%m-%d %H:%M:%S UTC"));

                if let Some(last_run) = schedule.last_run {
                    println!("Last run: {}", last_run.format("%Y-%m-%d %H:%M:%S UTC"));
                }

                if let Some(next_run) = schedule.next_run {
                    println!("Next run: {}", next_run.format("%Y-%m-%d %H:%M:%S UTC"));
                }

                // Show recent runs
                let runs = db.get_schedule_runs(schedule.id, 5).await?;
                if !runs.is_empty() {
                    println!("\nRecent executions:");
                    for run in runs {
                        let status = format!("{:?}", run.status);
                        let duration = run.completed_at
                            .map(|c| format!("{:.1}s", c.signed_duration_since(run.started_at).num_milliseconds() as f64 / 1000.0))
                            .unwrap_or_else(|| "-".to_string());
                        println!("  {} - {} ({})",
                            run.started_at.format("%Y-%m-%d %H:%M:%S"),
                            status,
                            duration
                        );
                    }
                }
            }

            ScheduleAction::Pause { name } => {
                let mut schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                schedule.enabled = false;
                db.update_schedule(&schedule).await?;

                println!("Schedule '{}' paused", name);
            }

            ScheduleAction::Resume { name } => {
                let mut schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                schedule.enabled = true;
                // Recalculate next run when resuming
                schedule.update_next_run()?;
                db.update_schedule(&schedule).await?;

                println!("Schedule '{}' resumed", name);
                if let Some(next_run) = schedule.next_run {
                    println!("Next run: {}", next_run.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }

            ScheduleAction::Delete { name } => {
                let schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                let deleted = db.delete_schedule(schedule.id).await?;

                if deleted {
                    println!("Schedule '{}' deleted", name);
                } else {
                    println!("Failed to delete schedule '{}'", name);
                }
            }

            ScheduleAction::RunNow { name } => {
                let schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                // Create a schedule run record
                let mut run = ScheduleRun::new(schedule.id);
                let run_id = db.insert_schedule_run(&run).await?;

                // Spawn the agent
                let agent_type = parse_agent_type(&schedule.agent_type)?;
                let agent = Agent::new(agent_type, schedule.task.clone());
                db.insert_agent(&agent).await?;

                // Update run record with agent ID
                run.mark_completed(agent.id.to_string());
                db.update_schedule_run(&run).await?;

                println!("Triggered schedule '{}' (run ID: {}, agent ID: {})", name, run_id, agent.id);
            }

            ScheduleAction::History { name, limit } => {
                let schedule = db.get_schedule_by_name(&name).await?
                    .ok_or_else(|| anyhow::anyhow!("Schedule not found: {}", name))?;

                let runs = db.get_schedule_runs(schedule.id, limit).await?;

                if runs.is_empty() {
                    println!("No execution history for schedule '{}'", name);
                    return Ok(());
                }

                println!("Execution history for '{}' (last {} runs)", name, limit);
                println!("{:<20} {:<15} {:<38} {:<10}", "STARTED", "STATUS", "AGENT", "DURATION");
                println!("{}", "-".repeat(90));

                for run in runs {
                    let status = format!("{:?}", run.status);
                    let agent_id = run.agent_id.as_deref().unwrap_or("-");
                    let duration = run.completed_at
                        .map(|c| format!("{:.1}s", c.signed_duration_since(run.started_at).num_milliseconds() as f64 / 1000.0))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<20} {:<15} {:<38} {:<10}",
                        run.started_at.format("%Y-%m-%d %H:%M:%S"),
                        status,
                        agent_id,
                        duration
                    );

                    if let Some(error) = &run.error_message {
                        println!("  Error: {}", error);
                    }
                }
            }

            ScheduleAction::AddTemplate { template_name, name } => {
                // Get the template
                let template = orchestrate_core::schedule_template::get_template(&template_name)
                    .ok_or_else(|| anyhow::anyhow!("Template '{}' not found. Use 'orchestrate schedule list-templates' to see available templates", template_name))?;

                // Use custom name if provided, otherwise use template name
                let schedule_name = name.as_ref().unwrap_or(&template.name).clone();

                // Check if schedule with this name already exists
                if db.get_schedule_by_name(&schedule_name).await?.is_some() {
                    anyhow::bail!("Schedule '{}' already exists", schedule_name);
                }

                // Create schedule from template
                let mut schedule = Schedule::new(
                    schedule_name.clone(),
                    template.cron.clone(),
                    template.agent.clone(),
                    template.task.clone(),
                );

                // Calculate next run
                schedule.update_next_run()?;

                // Insert into database
                let id = db.insert_schedule(&schedule).await?;

                println!("Schedule '{}' created from template '{}'", schedule_name, template_name);
                println!("Description: {}", template.description);
                println!("Cron: {}", template.cron);
                println!("Agent: {}", template.agent);
                if let Some(next_run) = schedule.next_run {
                    println!("Next run: {}", next_run.format("%Y-%m-%d %H:%M:%S UTC"));
                }
                println!("Schedule ID: {}", id);
            }

            ScheduleAction::ListTemplates => {
                use orchestrate_core::schedule_template;

                let templates = schedule_template::get_templates();

                if templates.is_empty() {
                    println!("No templates available");
                    return Ok(());
                }

                println!("Available schedule templates:\n");

                // Sort by name for consistent output
                let mut template_names: Vec<String> = templates.keys().cloned().collect();
                template_names.sort();

                for name in template_names {
                    let template = &templates[&name];
                    println!("Template: {}", template.name);
                    println!("  Description: {}", template.description);
                    println!("  Schedule: {}", template.cron);
                    println!("  Agent: {}", template.agent);
                    println!("  Task: {}", template.task);
                    println!();
                }

                println!("To add a template, use: orchestrate schedule add-template <template-name>");
            }
        },
    }

    Ok(())
}

async fn get_instruction_by_id_or_name(
    db: &Database,
    id_or_name: &str,
) -> Result<CustomInstruction> {
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

fn parse_agent_state(s: &str) -> Result<orchestrate_core::AgentState> {
    use orchestrate_core::AgentState;
    match s.to_lowercase().as_str() {
        "created" => Ok(AgentState::Created),
        "initializing" => Ok(AgentState::Initializing),
        "running" => Ok(AgentState::Running),
        "waiting-for-input" | "waitingforinput" => Ok(AgentState::WaitingForInput),
        "waiting-for-external" | "waitingforexternal" => Ok(AgentState::WaitingForExternal),
        "paused" => Ok(AgentState::Paused),
        "completed" => Ok(AgentState::Completed),
        "failed" => Ok(AgentState::Failed),
        "terminated" => Ok(AgentState::Terminated),
        _ => anyhow::bail!("Unknown agent state: {}. Valid: created, initializing, running, paused, completed, failed, terminated", s),
    }
}

/// Format token count with K/M suffix for readability
fn format_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.2}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Client type for daemon
#[derive(Clone)]
enum DaemonClient {
    Api(ClaudeClient),
    Cli(ClaudeCliClient),
}

/// Run the daemon to execute agents
async fn run_daemon(
    db: Database,
    port: u16,
    max_concurrent: usize,
    poll_interval: u64,
    model: String,
    use_cli: bool,
) -> Result<()> {
    // Create client based on mode
    let client = if use_cli {
        // Check if claude CLI is available
        let output = std::process::Command::new("claude")
            .arg("--version")
            .output();

        if output.is_err() || !output.unwrap().status.success() {
            anyhow::bail!("claude CLI not found. Install Claude Code or use API mode.");
        }

        DaemonClient::Cli(ClaudeCliClient::with_model(&model))
    } else {
        // Get API key from environment
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("CLAUDE_API_KEY"))
            .map_err(|_| {
                anyhow::anyhow!(
                    "ANTHROPIC_API_KEY or CLAUDE_API_KEY not set. Use --use-cli for OAuth."
                )
            })?;

        DaemonClient::Api(ClaudeClient::new(api_key))
    };

    let mode_str = if use_cli { "CLI (OAuth)" } else { "API" };

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    ORCHESTRATE DAEMON                        ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Mode:            {:<42} ║", mode_str);
    println!(
        "║  Model:           {:<42} ║",
        &model[..model.len().min(42)]
    );
    println!("║  Max concurrent:  {:<42} ║", max_concurrent);
    println!(
        "║  Poll interval:   {:<42} ║",
        format!("{}s", poll_interval)
    );
    if port > 0 {
        println!(
            "║  Web API:         {:<42} ║",
            format!("http://localhost:{}", port)
        );
    } else {
        println!("║  Web API:         {:<42} ║", "disabled");
    }
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Press Ctrl+C to stop the daemon");
    println!();

    // Setup shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        info!("Shutdown signal received");
        shutdown_clone.store(true, Ordering::SeqCst);
    });

    // Create semaphore for concurrency control
    let semaphore = Arc::new(Semaphore::new(max_concurrent));

    // Start web server (API + UI) if port > 0
    if port > 0 {
        let db_clone = db.clone();
        tokio::spawn(async move {
            let state = Arc::new(orchestrate_web::api::AppState::new(db_clone, None));
            let router = orchestrate_web::create_router(state);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
                .await
                .expect("Failed to bind to port");
            info!("Web server listening on port {} (API + UI)", port);
            axum::serve(listener, router).await.ok();
        });
    }

    // Main polling loop
    let mut active_agents: std::collections::HashSet<uuid::Uuid> = std::collections::HashSet::new();

    while !shutdown.load(Ordering::SeqCst) {
        // Get pending agents (Created state)
        let pending = match db.list_agents_by_state(AgentState::Created).await {
            Ok(agents) => agents,
            Err(e) => {
                error!("Failed to query pending agents: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
                continue;
            }
        };

        // Filter out agents we're already running
        let new_agents: Vec<_> = pending
            .into_iter()
            .filter(|a| !active_agents.contains(&a.id))
            .collect();

        if !new_agents.is_empty() {
            info!("Found {} new agent(s) to run", new_agents.len());
        }

        // Spawn tasks for new agents
        for agent in new_agents {
            let permit = match semaphore.clone().try_acquire_owned() {
                Ok(p) => p,
                Err(_) => {
                    info!("Max concurrent agents reached, waiting...");
                    break;
                }
            };

            let agent_id = agent.id;
            active_agents.insert(agent_id);

            let db_clone = db.clone();
            let client_clone = client.clone();
            let model_clone = model.clone();
            let shutdown_clone = shutdown.clone();

            tokio::spawn(async move {
                let _permit = permit; // Hold permit until done

                info!("[AGENT {}] Starting execution", agent_id);

                match run_single_agent(db_clone, client_clone, agent, model_clone, shutdown_clone)
                    .await
                {
                    Ok(()) => {
                        info!("[AGENT {}] Completed successfully", agent_id);
                    }
                    Err(e) => {
                        error!("[AGENT {}] Failed: {}", agent_id, e);
                    }
                }
            });
        }

        // Clean up completed agents from tracking set
        let running = db
            .list_agents_by_state(AgentState::Running)
            .await
            .unwrap_or_default();
        let running_ids: std::collections::HashSet<_> = running.iter().map(|a| a.id).collect();
        active_agents.retain(|id| running_ids.contains(id));

        // Wait before next poll
        tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
    }

    info!("Daemon shutting down...");

    // Wait for running agents to complete (with timeout)
    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    while semaphore.available_permits() < max_concurrent {
        if start.elapsed() > timeout {
            warn!("Timeout waiting for agents to complete, forcing shutdown");
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    println!("Daemon stopped");
    Ok(())
}

/// Run a single agent to completion
async fn run_single_agent(
    db: Database,
    client: DaemonClient,
    mut agent: Agent,
    model: String,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    match client {
        DaemonClient::Api(api_client) => {
            run_agent_with_api(db, api_client, &mut agent, model, shutdown).await
        }
        DaemonClient::Cli(cli_client) => {
            run_agent_with_cli(db, cli_client, &mut agent, model, shutdown).await
        }
    }
}

/// Run agent using direct API
async fn run_agent_with_api(
    db: Database,
    client: ClaudeClient,
    agent: &mut Agent,
    model: String,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    use orchestrate_claude::loop_runner::LoopConfig;

    let config = LoopConfig {
        model,
        max_turns: 80,
        enable_instructions: true,
        enable_learning: true,
        max_idle_turns: 5,
        max_consecutive_errors: 3,
        enable_token_optimization: true,
        enable_sessions: true,
    };

    let agent_loop = AgentLoop::new(client, db.clone(), config);

    // Run with periodic shutdown check
    let result = tokio::select! {
        result = agent_loop.run(agent) => result,
        _ = async {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        } => {
            // Shutdown requested, mark agent as paused
            agent.transition_to(AgentState::Paused).ok();
            db.update_agent(agent).await.ok();
            return Ok(());
        }
    };

    // Update final state
    db.update_agent(agent).await?;

    result
}

/// Run agent using claude CLI (OAuth auth)
async fn run_agent_with_cli(
    db: Database,
    _cli_client: ClaudeCliClient,
    agent: &mut Agent,
    model: String,
    shutdown: Arc<AtomicBool>,
) -> Result<()> {
    use orchestrate_core::Message;
    use tokio::process::Command;

    // Transition through proper state machine: Created -> Initializing -> Running
    agent.transition_to(AgentState::Initializing)?;
    db.update_agent(agent).await?;

    agent.transition_to(AgentState::Running)?;
    db.update_agent(agent).await?;

    // Build the prompt
    let prompt = format!(
        "You are an autonomous agent. Complete this task:\n\n{}\n\nUse the available tools to complete the task. When done, output STATUS: DONE.",
        agent.task
    );

    // Build command
    let mut cmd = Command::new("claude");
    cmd.arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--model")
        .arg(&model)
        .arg("--dangerously-skip-permissions"); // For autonomous operation

    // Set working directory to worktree path if available
    let working_dir: Option<String> = if let Some(ref worktree_id) = agent.worktree_id {
        match db.get_worktree_path(worktree_id).await {
            Ok(Some(path)) => {
                cmd.current_dir(&path);
                info!("[AGENT {}] Using worktree: {}", agent.id, path);
                Some(path)
            }
            Ok(None) => {
                warn!("[AGENT {}] Worktree {} not found", agent.id, worktree_id);
                None
            }
            Err(e) => {
                warn!("[AGENT {}] Failed to get worktree path: {}", agent.id, e);
                None
            }
        }
    } else {
        None
    };

    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    info!(
        "[AGENT {}] Running via CLI with model {}{}",
        agent.id,
        model,
        working_dir
            .as_ref()
            .map(|p| format!(" in {}", p))
            .unwrap_or_default()
    );

    let mut child = cmd.spawn()?;

    // Write prompt
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(prompt.as_bytes()).await?;
        drop(stdin); // Close stdin
    }

    // Wait for completion with shutdown check
    let output = tokio::select! {
        output = child.wait_with_output() => output,
        _ = async {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        } => {
            // Shutdown - we can't kill since wait_with_output consumed child
            agent.transition_to(AgentState::Paused)?;
            db.update_agent(agent).await.ok();
            return Ok(());
        }
    };

    let result = match output {
        Ok(out) => {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);

                // Parse response to get result
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
                    let result_text = json
                        .get("result")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Task completed");

                    // Store final message
                    let msg = Message::assistant(agent.id, result_text);
                    db.insert_message(&msg).await.ok();

                    // Update token usage if available
                    if let Some(usage) = json.get("usage") {
                        let input = usage
                            .get("input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let output_tokens = usage
                            .get("output_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let cache_read = usage
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let cache_write = usage
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        db.update_daily_token_usage(
                            &model,
                            input,
                            output_tokens,
                            cache_read,
                            cache_write,
                        )
                        .await
                        .ok();
                    }

                    agent.transition_to(AgentState::Completed)?;
                } else {
                    agent.transition_to(AgentState::Completed)?;
                }
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&out.stderr);
                agent.error_message = Some(stderr.to_string());
                agent.transition_to(AgentState::Failed)?;
                anyhow::bail!("CLI failed: {}", stderr)
            }
        }
        Err(e) => {
            agent.error_message = Some(e.to_string());
            agent.transition_to(AgentState::Failed)?;
            Err(e.into())
        }
    };

    db.update_agent(agent).await?;
    result
}

// ==================== Story Functions ====================

/// Parse story status from string
fn parse_story_status(s: &str) -> Result<StoryStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(StoryStatus::Pending),
        "in-progress" | "inprogress" => Ok(StoryStatus::InProgress),
        "completed" => Ok(StoryStatus::Completed),
        "blocked" => Ok(StoryStatus::Blocked),
        "skipped" => Ok(StoryStatus::Skipped),
        _ => anyhow::bail!(
            "Unknown story status: {}. Valid: pending, in-progress, completed, blocked, skipped",
            s
        ),
    }
}

/// Show detailed story information
async fn show_story(db: &Database, id: &str) -> Result<()> {
    let story = db
        .get_story(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Story not found: {}", id))?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                      STORY DETAILS                           ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Story: {}", story.id);
    println!("Title: {}", story.title);
    println!("Epic: {}", story.epic_id);
    println!("Status: {:?}", story.status);
    println!();

    if let Some(ref desc) = story.description {
        println!("Description:");
        println!("{}", "-".repeat(60));
        println!("{}", desc);
        println!();
    }

    if let Some(ref criteria) = story.acceptance_criteria {
        println!("Acceptance Criteria:");
        println!("{}", "-".repeat(60));

        // Try to parse as array of strings
        if let Some(array) = criteria.as_array() {
            for (i, item) in array.iter().enumerate() {
                if let Some(text) = item.as_str() {
                    println!("  {}. {}", i + 1, text);
                }
            }
        } else if let Some(text) = criteria.as_str() {
            println!("{}", text);
        } else {
            println!("{}", serde_json::to_string_pretty(criteria)?);
        }
        println!();
    }

    if let Some(agent_id) = story.agent_id {
        println!("Assigned Agent: {}", agent_id);

        // Try to get agent details
        if let Ok(Some(agent)) = db.get_agent(agent_id).await {
            println!("  Type: {:?}", agent.agent_type);
            println!("  State: {:?}", agent.state);
        }
        println!();
    }

    println!("Created: {}", story.created_at);
    println!("Updated: {}", story.updated_at);

    Ok(())
}

// ==================== BMAD Functions ====================

/// Process BMAD epics from the specified directory
async fn process_bmad_epics(
    db: &Database,
    epics_dir: &std::path::Path,
    pattern: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    use regex::Regex;
    use std::fs;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    BMAD EPIC PROCESSOR                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check if directory exists
    if !epics_dir.exists() {
        println!("📁 Epics directory does not exist: {}", epics_dir.display());
        println!("   Creating directory...");
        if !dry_run {
            fs::create_dir_all(epics_dir)?;
        }
        println!("   ✓ Created {}", epics_dir.display());
        println!();
        println!("To add epics, create markdown files in this directory:");
        println!("   {}/epic-001-my-feature.md", epics_dir.display());
        return Ok(());
    }

    // Find epic files
    let entries: Vec<_> = fs::read_dir(epics_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".md") && name.starts_with("epic-")
        })
        .collect();

    if entries.is_empty() {
        println!("📭 No epic files found in {}", epics_dir.display());
        println!();
        println!("Expected format: epic-<id>-<name>.md");
        println!("Example: epic-001-user-authentication.md");
        return Ok(());
    }

    // Filter by pattern if provided
    let pattern_regex = pattern
        .map(|p| {
            let regex_pattern = p.replace("*", ".*").replace("?", ".");
            Regex::new(&format!("^{}$", regex_pattern)).ok()
        })
        .flatten();

    let filtered_entries: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            if let Some(ref re) = pattern_regex {
                let name = e.file_name().to_string_lossy().to_string();
                re.is_match(&name)
            } else {
                true
            }
        })
        .collect();

    println!(
        "📋 Found {} epic file(s){}",
        filtered_entries.len(),
        pattern
            .map(|p| format!(" matching '{}'", p))
            .unwrap_or_default()
    );
    println!();

    if dry_run {
        println!("🔍 DRY RUN - No changes will be made");
        println!();
    }

    for entry in filtered_entries {
        let path = entry.path();
        let filename = entry.file_name().to_string_lossy().to_string();

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("📖 Processing: {}", filename);

        // Parse the epic file
        let content = fs::read_to_string(&path)?;
        let (epic, stories) = parse_epic_file(&filename, &content)?;

        println!("   Title: {}", epic.title);
        println!("   Stories: {}", stories.len());

        if dry_run {
            println!(
                "   [DRY RUN] Would create epic and {} stories",
                stories.len()
            );
            for story in &stories {
                println!("      - {}: {}", story.id, story.title);
            }
            continue;
        }

        // Save epic to database
        db.upsert_epic(&epic).await?;
        println!("   ✓ Epic saved to database");

        // Save stories to database
        for story in &stories {
            db.upsert_story(story).await?;
        }
        println!("   ✓ {} stories saved", stories.len());

        // Create worktree for the epic
        let worktree_name = format!("epic-{}", epic.id.replace("epic-", ""));
        let worktree_path = format!(".worktrees/{}", worktree_name);
        let branch_name = format!("feat/{}", worktree_name);

        if !std::path::Path::new(&worktree_path).exists() {
            println!("   Creating worktree: {}", worktree_path);

            let output = tokio::process::Command::new("git")
                .args(["worktree", "add", &worktree_path, "-b", &branch_name])
                .output()
                .await?;

            if output.status.success() {
                // Save worktree to database
                let worktree = Worktree::new(&worktree_name, &worktree_path, &branch_name, "main");
                db.insert_worktree(&worktree).await?;
                println!("   ✓ Worktree created: {}", worktree_path);
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("   ⚠ Failed to create worktree: {}", stderr.trim());
            }
        } else {
            println!("   ⏭ Worktree already exists: {}", worktree_path);
        }

        // Create agents for pending stories
        let pending_stories: Vec<_> = stories
            .iter()
            .filter(|s| s.status == StoryStatus::Pending)
            .collect();

        if !pending_stories.is_empty() {
            println!(
                "   Creating agents for {} pending stories...",
                pending_stories.len()
            );

            for story in pending_stories {
                // Check if agent already exists for this story
                let existing = db
                    .list_stories(None)
                    .await?
                    .iter()
                    .find(|s| s.id == story.id && s.agent_id.is_some())
                    .is_some();

                if existing {
                    println!("      ⏭ Story {} already has an agent", story.id);
                    continue;
                }

                // Create story-developer agent
                let task = format!(
                    "Implement story {}: {}\n\n{}",
                    story.id,
                    story.title,
                    story
                        .description
                        .as_deref()
                        .unwrap_or("No description provided.")
                );

                let agent = Agent::new(AgentType::StoryDeveloper, &task);
                db.insert_agent(&agent).await?;

                // Link story to agent
                db.update_story_status(&story.id, StoryStatus::Pending, Some(agent.id))
                    .await?;

                println!("      ✓ Created agent for story {}", story.id);
            }
        }

        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ BMAD processing complete");
    println!();
    println!("Next steps:");
    println!("  1. Start the daemon: orchestrate daemon start --use-cli");
    println!("  2. Monitor progress: orchestrate bmad status");
    println!("  3. View agents: orchestrate agent list");

    Ok(())
}

/// Parse an epic markdown file into Epic and Stories
fn parse_epic_file(filename: &str, content: &str) -> Result<(Epic, Vec<Story>)> {
    use regex::Regex;

    // Extract epic ID from filename (e.g., "epic-001-user-auth.md" -> "epic-001")
    let id_regex = Regex::new(r"^(epic-\d+)")?;
    let epic_id = id_regex
        .captures(filename)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| filename.replace(".md", ""));

    // Parse title from first # heading
    let title_regex = Regex::new(r"^#\s+(.+)$")?;
    let title = content
        .lines()
        .find_map(|line| {
            title_regex
                .captures(line)
                .map(|c| c.get(1).unwrap().as_str().to_string())
        })
        .unwrap_or_else(|| filename.replace(".md", "").replace("-", " "));

    // Create epic
    let mut epic = Epic::new(&epic_id, &title);
    epic.source_file = Some(filename.to_string());

    // Parse stories from ## Story headings or numbered lists
    let story_heading_regex = Regex::new(r"^##\s+Story\s+(\S+):\s*(.+)$")?;
    let story_list_regex = Regex::new(r"^\d+\.\s+\*\*(.+?)\*\*:?\s*(.*)$")?;
    let checkbox_regex = Regex::new(r"^-\s+\[([ x])\]\s+(.+)$")?;

    let mut stories = Vec::new();
    let mut current_story: Option<(String, String, Vec<String>)> = None;
    let mut in_story_section = false;

    for line in content.lines() {
        // Check for story heading
        if let Some(caps) = story_heading_regex.captures(line) {
            // Save previous story if exists
            if let Some((id, title, criteria)) = current_story.take() {
                let mut story = Story::new(&id, &epic_id, &title);
                if !criteria.is_empty() {
                    story.acceptance_criteria = Some(serde_json::json!(criteria));
                }
                stories.push(story);
            }

            let story_id = format!("{}.{}", epic_id, caps.get(1).unwrap().as_str());
            let story_title = caps.get(2).unwrap().as_str().to_string();
            current_story = Some((story_id, story_title, Vec::new()));
            in_story_section = true;
            continue;
        }

        // Check for numbered list story format
        if let Some(caps) = story_list_regex.captures(line) {
            // Save previous story if exists
            if let Some((id, title, criteria)) = current_story.take() {
                let mut story = Story::new(&id, &epic_id, &title);
                if !criteria.is_empty() {
                    story.acceptance_criteria = Some(serde_json::json!(criteria));
                }
                stories.push(story);
            }

            let story_num = stories.len() + 1;
            let story_id = format!("{}.{}", epic_id, story_num);
            let story_title = caps.get(1).unwrap().as_str().to_string();
            let description = caps.get(2).map(|m| m.as_str().to_string());

            let mut story = Story::new(&story_id, &epic_id, &story_title);
            story.description = description;
            stories.push(story);
            current_story = None;
            continue;
        }

        // Parse acceptance criteria (checkboxes)
        if in_story_section {
            if let Some(caps) = checkbox_regex.captures(line) {
                if let Some((_, _, ref mut criteria)) = current_story {
                    criteria.push(caps.get(2).unwrap().as_str().to_string());
                }
            }
        }

        // Check for section end
        if line.starts_with("## ") && !line.contains("Story") {
            in_story_section = false;
            if let Some((id, title, criteria)) = current_story.take() {
                let mut story = Story::new(&id, &epic_id, &title);
                if !criteria.is_empty() {
                    story.acceptance_criteria = Some(serde_json::json!(criteria));
                }
                stories.push(story);
            }
        }
    }

    // Don't forget the last story
    if let Some((id, title, criteria)) = current_story {
        let mut story = Story::new(&id, &epic_id, &title);
        if !criteria.is_empty() {
            story.acceptance_criteria = Some(serde_json::json!(criteria));
        }
        stories.push(story);
    }

    Ok((epic, stories))
}

/// Show BMAD status for all epics
async fn show_bmad_status(db: &Database) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                      BMAD STATUS                             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Get all epics (we need a list_epics method, but for now use pending)
    let pending_epics = db.get_pending_epics().await?;

    if pending_epics.is_empty() {
        println!("📭 No epics found in the system.");
        println!();
        println!("To add epics, run: orchestrate bmad process");
        return Ok(());
    }

    for epic in &pending_epics {
        let stories = db.get_stories_for_epic(&epic.id).await?;
        let completed = stories
            .iter()
            .filter(|s| s.status == StoryStatus::Completed)
            .count();
        let in_progress = stories
            .iter()
            .filter(|s| s.status == StoryStatus::InProgress)
            .count();
        let pending = stories
            .iter()
            .filter(|s| s.status == StoryStatus::Pending)
            .count();
        let blocked = stories
            .iter()
            .filter(|s| s.status == StoryStatus::Blocked)
            .count();

        let phase_str = epic
            .current_phase
            .map(|p| format!("{}", p))
            .unwrap_or_else(|| "NOT_STARTED".to_string());

        let status_icon = match epic.status {
            EpicStatus::Pending => "⏳",
            EpicStatus::InProgress => "🔄",
            EpicStatus::Completed => "✅",
            EpicStatus::Blocked => "🚫",
            EpicStatus::Skipped => "⏭",
        };

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("{} Epic: {} - {}", status_icon, epic.id, epic.title);
        println!("   Phase: {}", phase_str);
        println!("   Stories: {}/{} complete", completed, stories.len());

        if in_progress > 0 || pending > 0 || blocked > 0 {
            println!(
                "   Progress: {} in progress, {} pending, {} blocked",
                in_progress, pending, blocked
            );
        }

        // Show story details
        if !stories.is_empty() {
            println!();
            for story in &stories {
                let icon = match story.status {
                    StoryStatus::Pending => "○",
                    StoryStatus::InProgress => "⏳",
                    StoryStatus::Completed => "✓",
                    StoryStatus::Blocked => "✗",
                    StoryStatus::Skipped => "⏭",
                };
                let agent_str = story
                    .agent_id
                    .map(|id| format!(" [agent: {}]", &id.to_string()[..8]))
                    .unwrap_or_default();
                println!("      {} {}: {}{}", icon, story.id, story.title, agent_str);
            }
        }
        println!();
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

/// Reset BMAD state
async fn reset_bmad_state(db: &Database, force: bool) -> Result<()> {
    if !force {
        println!("⚠️  This will delete all epics and stories from the database.");
        println!("   Run with --force to confirm.");
        return Ok(());
    }

    println!("🗑️  Resetting BMAD state...");

    // Get all stories and delete them
    let stories = db.list_stories(None).await?;
    for story in &stories {
        db.delete_story(&story.id).await?;
    }
    println!("   ✓ Deleted {} stories", stories.len());

    // Note: We would need a delete_epic method or similar
    // For now, just report what we did
    println!();
    println!("✅ BMAD state reset complete");
    println!("   Note: Epics may need manual cleanup if delete_epic is not implemented.");

    Ok(())
}
