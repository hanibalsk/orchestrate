//! Orchestrate CLI

use anyhow::Result;
use clap::{Parser, Subcommand};
use orchestrate_claude::{AgentLoop, ClaudeCliClient, ClaudeClient};
use orchestrate_core::{
    Agent, AgentState, AgentType, CreateEnvironment, CustomInstruction, Database,
    Epic, EpicStatus, EnvironmentType, LearningEngine, PatternStatus, PreDeployValidator,
    Schedule, ScheduleRun, ShellState, Story, StoryStatus, ValidationStatus, Worktree,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use uuid::Uuid;
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
    /// Webhook server management
    Webhook {
        #[command(subcommand)]
        action: WebhookAction,
    },
    /// Pipeline management
    Pipeline {
        #[command(subcommand)]
        action: PipelineAction,
    },
    /// Approval management
    Approval {
        #[command(subcommand)]
        action: ApprovalAction,
    },
    /// Feedback collection
    Feedback {
        #[command(subcommand)]
        action: FeedbackAction,
    },
    /// Environment management
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },
    /// Deployment management
    Deploy {
        #[command(subcommand)]
        action: DeployAction,
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
    /// List success patterns
    Successes {
        /// Filter by pattern type (tool_sequence, prompt_structure, context_size, model_choice, timing)
        #[arg(short = 't', long)]
        pattern_type: Option<String>,
        /// Filter by agent type
        #[arg(short = 'a', long)]
        agent_type: Option<String>,
        /// Show detailed pattern data
        #[arg(long)]
        detailed: bool,
    },
    /// Get success recommendations for an agent type
    Recommend {
        /// Agent type to get recommendations for
        #[arg(short = 't', long)]
        agent_type: String,
        /// Task type to filter recommendations
        #[arg(long)]
        task_type: Option<String>,
    },
    /// Cleanup old success patterns
    CleanupSuccesses {
        /// Remove patterns older than this many days
        #[arg(short, long, default_value = "90")]
        days: i64,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
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

#[derive(Subcommand)]
enum WebhookAction {
    /// Start webhook server
    Start {
        /// Port for webhook server
        #[arg(short, long, default_value = "9000")]
        port: u16,
        /// Webhook secret (defaults to GITHUB_WEBHOOK_SECRET env var)
        #[arg(short, long, env = "GITHUB_WEBHOOK_SECRET")]
        secret: Option<String>,
    },
    /// List recent webhook events
    ListEvents {
        /// Maximum number of events to show
        #[arg(short, long, default_value = "20")]
        limit: i64,
        /// Filter by status (pending, processing, completed, failed, dead_letter)
        #[arg(short, long)]
        status: Option<String>,
    },
    /// Simulate a webhook event for testing
    Simulate {
        /// Event type (e.g., pull_request.opened, check_run.completed)
        event_type: String,
        /// Optional JSON payload file
        #[arg(short, long)]
        payload_file: Option<PathBuf>,
    },
    /// Show webhook server status
    Status,
    /// Manage webhook secret
    Secret {
        #[command(subcommand)]
        action: SecretAction,
    },
}

#[derive(Subcommand)]
enum SecretAction {
    /// Generate and rotate webhook secret
    Rotate,
    /// Show current webhook secret
    Show,
}

#[derive(Subcommand)]
enum PipelineAction {
    /// Create pipeline from YAML file
    Create {
        /// Path to YAML file
        file: PathBuf,
    },
    /// List all pipelines
    List {
        /// Show only enabled pipelines
        #[arg(long)]
        enabled_only: bool,
    },
    /// Show pipeline definition
    Show {
        /// Pipeline name
        name: String,
    },
    /// Update pipeline from YAML file
    Update {
        /// Pipeline name
        name: String,
        /// Path to YAML file
        file: PathBuf,
    },
    /// Delete pipeline
    Delete {
        /// Pipeline name
        name: String,
    },
    /// Enable pipeline
    Enable {
        /// Pipeline name
        name: String,
    },
    /// Disable pipeline
    Disable {
        /// Pipeline name
        name: String,
    },
    /// Trigger pipeline manually
    Run {
        /// Pipeline name
        name: String,
        /// Dry run - show what would be done without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Show pipeline run status
    Status {
        /// Run ID
        run_id: i64,
    },
    /// Cancel running pipeline
    Cancel {
        /// Run ID
        run_id: i64,
    },
    /// Show pipeline run history
    History {
        /// Pipeline name
        name: String,
        /// Number of runs to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Initialize pipeline from template
    Init {
        /// Template name (ci, cd, release, security)
        template: Option<String>,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// List available templates
        #[arg(long)]
        list: bool,
        /// Force overwrite existing file
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ApprovalAction {
    /// List approval requests
    List {
        /// Show only pending approvals
        #[arg(long)]
        pending: bool,
    },
    /// Approve a request
    Approve {
        /// Approval request ID
        id: i64,
        /// Optional comment
        #[arg(short, long)]
        comment: Option<String>,
    },
    /// Reject a request
    Reject {
        /// Approval request ID
        id: i64,
        /// Reason for rejection
        #[arg(short, long)]
        reason: Option<String>,
    },
    /// Delegate approval to another user
    Delegate {
        /// Approval request ID
        id: i64,
        /// User to delegate to
        #[arg(long)]
        to: String,
    },
}

#[derive(Subcommand)]
enum FeedbackAction {
    /// Add feedback for an agent
    Add {
        /// Agent ID (UUID)
        agent_id: String,
        /// Rating (positive, negative, neutral, +, -, pos, neg)
        #[arg(short, long)]
        rating: String,
        /// Optional comment
        #[arg(short, long)]
        comment: Option<String>,
        /// Optional message ID for specific output feedback
        #[arg(short, long)]
        message_id: Option<i64>,
    },
    /// List feedback
    List {
        /// Filter by agent ID
        #[arg(short, long)]
        agent: Option<String>,
        /// Filter by rating (positive, negative, neutral)
        #[arg(short, long)]
        rating: Option<String>,
        /// Filter by source (cli, web, slack, api, automated)
        #[arg(short, long)]
        source: Option<String>,
        /// Maximum number of results
        #[arg(long, default_value = "50")]
        limit: i64,
    },
    /// Show feedback statistics
    Stats {
        /// Show stats for specific agent
        #[arg(short, long)]
        agent: Option<String>,
        /// Group stats by agent type
        #[arg(long)]
        by_type: bool,
    },
    /// Delete feedback
    Delete {
        /// Feedback ID
        id: i64,
    },
}

#[derive(Subcommand)]
enum EnvAction {
    /// List all environments
    List {
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Create a new environment
    Create {
        /// Environment name (e.g., staging, production)
        name: String,
        /// Environment type (dev, staging, production)
        #[arg(short, long)]
        env_type: String,
        /// Environment URL
        #[arg(short, long)]
        url: Option<String>,
        /// Deployment provider (aws, k8s, etc.)
        #[arg(short, long)]
        provider: Option<String>,
        /// Configuration as JSON
        #[arg(short, long)]
        config: Option<String>,
        /// Secrets as JSON (will be encrypted)
        #[arg(short, long)]
        secrets: Option<String>,
        /// Require approval for deployments
        #[arg(long)]
        requires_approval: bool,
    },
    /// Show environment details
    Show {
        /// Environment name
        name: String,
        /// Include secrets in output
        #[arg(long)]
        show_secrets: bool,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },
    /// Delete an environment
    Delete {
        /// Environment name
        name: String,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum DeployAction {
    /// Validate environment is ready for deployment
    Validate {
        /// Environment name (e.g., staging, production)
        #[arg(short, long)]
        env: String,
        /// Version to validate (optional)
        #[arg(long)]
        version: Option<String>,
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
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
            LearnAction::Successes {
                pattern_type,
                agent_type,
                detailed,
            } => {
                use orchestrate_core::SuccessPatternType;

                let type_filter = pattern_type
                    .as_ref()
                    .map(|t| SuccessPatternType::from_str(t))
                    .transpose()?;

                let patterns = db.list_success_patterns(type_filter, 100).await?;

                // Filter by agent type if specified
                let patterns: Vec<_> = if let Some(ref at) = agent_type {
                    let agent_type_filter = parse_agent_type(at)?;
                    patterns
                        .into_iter()
                        .filter(|p| p.agent_type == Some(agent_type_filter))
                        .collect()
                } else {
                    patterns
                };

                if patterns.is_empty() {
                    println!("No success patterns found");
                    return Ok(());
                }

                println!("Success Patterns");
                println!("{}", "=".repeat(80));
                println!(
                    "{:<6} {:<18} {:<15} {:<10} {:<8} {:<8} {:<12}",
                    "ID", "TYPE", "AGENT_TYPE", "TASK", "COUNT", "RATE", "AVG_TIME"
                );
                println!("{}", "-".repeat(80));

                for pattern in &patterns {
                    let task_type = pattern
                        .task_type
                        .as_ref()
                        .map(|s| if s.len() > 10 { &s[..10] } else { s })
                        .unwrap_or("-");
                    let agent_type_str = pattern
                        .agent_type
                        .map(|t| t.as_str().to_string())
                        .unwrap_or_else(|| "global".to_string());
                    let avg_time = pattern
                        .avg_completion_time_ms
                        .map(|t| format!("{}ms", t))
                        .unwrap_or_else(|| "-".to_string());

                    println!(
                        "{:<6} {:<18} {:<15} {:<10} {:<8} {:<8.2} {:<12}",
                        pattern.id,
                        pattern.pattern_type.as_str(),
                        agent_type_str,
                        task_type,
                        pattern.occurrence_count,
                        pattern.success_rate,
                        avg_time
                    );

                    if detailed {
                        println!("  Data: {}", serde_json::to_string(&pattern.pattern_data)?);
                    }
                }
            }
            LearnAction::Recommend {
                agent_type,
                task_type,
            } => {
                let agent_type_parsed = parse_agent_type(&agent_type)?;
                let engine = LearningEngine::new();
                let recommendations = engine
                    .get_success_recommendations(&db, agent_type_parsed, task_type.as_deref())
                    .await?;

                println!("Success Recommendations for {}", agent_type);
                println!("{}", "=".repeat(60));

                if let Some(msg_count) = recommendations.recommended_message_count {
                    println!("Recommended message count: {}", msg_count);
                }
                if let Some(time) = recommendations.expected_completion_time_ms {
                    let seconds = time / 1000;
                    if seconds > 60 {
                        println!("Expected completion time: {}m {}s", seconds / 60, seconds % 60);
                    } else {
                        println!("Expected completion time: {}s", seconds);
                    }
                }

                if !recommendations.successful_prompt_features.is_empty() {
                    println!("\nSuccessful prompt features:");
                    for feature in &recommendations.successful_prompt_features {
                        println!("  - {}", feature);
                    }
                }

                if !recommendations.recommended_tool_sequences.is_empty() {
                    println!("\nCommon tool sequences:");
                    for (i, seq) in recommendations.recommended_tool_sequences.iter().take(5).enumerate() {
                        println!("  {}. {}", i + 1, seq.join(" → "));
                    }
                }
            }
            LearnAction::CleanupSuccesses { days, force } => {
                if !force {
                    println!(
                        "This will delete success patterns older than {} days with < 5 occurrences.",
                        days
                    );
                    println!("Use --force to skip this confirmation.");
                    // In a real CLI we'd prompt for confirmation here
                    return Ok(());
                }

                let deleted = db.cleanup_old_success_patterns(days).await?;
                println!("Deleted {} old success patterns", deleted);
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

        Commands::Webhook { action } => match action {
            WebhookAction::Start { port, secret } => {
                handle_webhook_start(db, port, secret).await?;
            }
            WebhookAction::ListEvents { limit, status } => {
                handle_webhook_list_events(db, limit, status.as_deref()).await?;
            }
            WebhookAction::Simulate {
                event_type,
                payload_file,
            } => {
                handle_webhook_simulate(db, &event_type, payload_file.as_ref()).await?;
            }
            WebhookAction::Status => {
                handle_webhook_status().await?;
            }
            WebhookAction::Secret { action } => match action {
                SecretAction::Rotate => {
                    handle_webhook_secret_rotate().await?;
                }
                SecretAction::Show => {
                    handle_webhook_secret_show().await?;
                }
            },
        },

        Commands::Pipeline { action } => match action {
            PipelineAction::Create { file } => {
                handle_pipeline_create(&db, &file).await?;
            }
            PipelineAction::List { enabled_only } => {
                handle_pipeline_list(&db, enabled_only).await?;
            }
            PipelineAction::Show { name } => {
                handle_pipeline_show(&db, &name).await?;
            }
            PipelineAction::Update { name, file } => {
                handle_pipeline_update(&db, &name, &file).await?;
            }
            PipelineAction::Delete { name } => {
                handle_pipeline_delete(&db, &name).await?;
            }
            PipelineAction::Enable { name } => {
                handle_pipeline_enable(&db, &name).await?;
            }
            PipelineAction::Disable { name } => {
                handle_pipeline_disable(&db, &name).await?;
            }
            PipelineAction::Run { name, dry_run } => {
                handle_pipeline_run(&db, &name, dry_run).await?;
            }
            PipelineAction::Status { run_id } => {
                handle_pipeline_status(&db, run_id).await?;
            }
            PipelineAction::Cancel { run_id } => {
                handle_pipeline_cancel(&db, run_id).await?;
            }
            PipelineAction::History { name, limit } => {
                handle_pipeline_history(&db, &name, limit).await?;
            }
            PipelineAction::Init { template, output, list, force } => {
                handle_pipeline_init(template.as_deref(), output.as_ref(), list, force)?;
            }
        },

        Commands::Approval { action } => match action {
            ApprovalAction::List { pending } => {
                handle_approval_list(&db, pending).await?;
            }
            ApprovalAction::Approve { id, comment } => {
                handle_approval_approve(&db, id, comment.as_deref()).await?;
            }
            ApprovalAction::Reject { id, reason } => {
                handle_approval_reject(&db, id, reason.as_deref()).await?;
            }
            ApprovalAction::Delegate { id, to } => {
                handle_approval_delegate(&db, id, &to).await?;
            }
        },
        Commands::Feedback { action } => match action {
            FeedbackAction::Add {
                agent_id,
                rating,
                comment,
                message_id,
            } => {
                handle_feedback_add(&db, &agent_id, &rating, comment.as_deref(), message_id)
                    .await?;
            }
            FeedbackAction::List {
                agent,
                rating,
                source,
                limit,
            } => {
                handle_feedback_list(&db, agent.as_deref(), rating.as_deref(), source.as_deref(), limit)
                    .await?;
            }
            FeedbackAction::Stats { agent, by_type } => {
                handle_feedback_stats(&db, agent.as_deref(), by_type).await?;
            }
            FeedbackAction::Delete { id } => {
                handle_feedback_delete(&db, id).await?;
            }
        },
        Commands::Env { action } => match action {
            EnvAction::List { format } => {
                handle_env_list(&db, &format).await?;
            }
            EnvAction::Create {
                name,
                env_type,
                url,
                provider,
                config,
                secrets,
                requires_approval,
            } => {
                handle_env_create(
                    &db,
                    &name,
                    &env_type,
                    url.as_deref(),
                    provider.as_deref(),
                    config.as_deref(),
                    secrets.as_deref(),
                    requires_approval,
                )
                .await?;
            }
            EnvAction::Show {
                name,
                show_secrets,
                format,
            } => {
                handle_env_show(&db, &name, show_secrets, &format).await?;
            }
            EnvAction::Delete { name, yes } => {
                handle_env_delete(&db, &name, yes).await?;
            }
        },
        Commands::Deploy { action } => match action {
            DeployAction::Validate {
                env,
                version,
                format,
            } => {
                handle_deploy_validate(&db, &env, version.as_deref(), &format).await?;
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

/// Handle webhook start command
async fn handle_webhook_start(
    db: Database,
    port: u16,
    secret: Option<String>,
) -> Result<()> {
    use orchestrate_web::{WebhookProcessor, WebhookProcessorConfig, create_router_with_webhook};
    use std::sync::Arc;

    info!("Starting webhook server on port {}", port);

    let webhook_secret = secret.or_else(|| std::env::var("GITHUB_WEBHOOK_SECRET").ok());

    if webhook_secret.is_none() {
        warn!("No webhook secret configured. Signature verification will be skipped.");
        warn!("Set GITHUB_WEBHOOK_SECRET environment variable or use --secret flag.");
    }

    let db_arc = Arc::new(db);

    // Start webhook processor in background
    let processor = WebhookProcessor::new(db_arc.clone(), WebhookProcessorConfig::default());
    tokio::spawn(async move {
        processor.run().await;
    });

    // Create AppState for the router
    let app_state = Arc::new(orchestrate_web::api::AppState::new(
        db_arc.as_ref().clone(),
        None, // No API key for webhook-only server
    ));

    // Create router with webhook endpoint
    let app = create_router_with_webhook(app_state, webhook_secret.clone());

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               WEBHOOK SERVER STARTED                         ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Listening on: {:<46} ║", addr);
    println!("║  Webhook URL:  {:<46} ║", format!("http://{}:{}/webhooks/github", "localhost", port));
    println!("║  Secret configured: {:<39} ║", if webhook_secret.is_some() { "Yes" } else { "No" });
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Handle webhook list-events command
async fn handle_webhook_list_events(
    db: Database,
    limit: i64,
    status_filter: Option<&str>,
) -> Result<()> {
    use orchestrate_core::WebhookEventStatus;
    use std::str::FromStr;

    let events = if let Some(status_str) = status_filter {
        let status = WebhookEventStatus::from_str(status_str)?;
        db.get_webhook_events_by_status(status, limit).await?
    } else {
        // Get recent events sorted by created_at DESC
        db.get_recent_webhook_events(limit).await?
    };

    if events.is_empty() {
        println!("No webhook events found");
        return Ok(());
    }

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                           WEBHOOK EVENTS                                     ║");
    println!("╠════════╦═══════════════════╦═════════════╦═════════╦═════════════════════════╣");
    println!("║   ID   ║   Event Type      ║   Status    ║ Retries ║   Received At           ║");
    println!("╠════════╬═══════════════════╬═════════════╬═════════╬═════════════════════════╣");

    for event in events {
        println!(
            "║ {:>6} ║ {:<17} ║ {:<11} ║   {:>3}   ║ {} ║",
            event.id.unwrap_or(0),
            event.event_type.chars().take(17).collect::<String>(),
            event.status.as_str(),
            event.retry_count,
            event.received_at.format("%Y-%m-%d %H:%M:%S")
        );
    }

    println!("╚════════╩═══════════════════╩═════════════╩═════════╩═════════════════════════╝");

    Ok(())
}

/// Handle webhook simulate command
async fn handle_webhook_simulate(
    db: Database,
    event_type: &str,
    payload_file: Option<&PathBuf>,
) -> Result<()> {
    use orchestrate_core::WebhookEvent;
    use uuid::Uuid;

    let payload = if let Some(file_path) = payload_file {
        std::fs::read_to_string(file_path)?
    } else {
        // Generate a minimal test payload based on event type
        generate_test_payload(event_type)
    };

    let delivery_id = format!("sim-{}", Uuid::new_v4());
    let event = WebhookEvent::new(delivery_id.clone(), event_type.to_string(), payload);

    db.insert_webhook_event(&event).await?;

    println!("✅ Simulated event queued successfully");
    println!();
    println!("Event Type:    {}", event_type);
    println!("Delivery ID:   {}", delivery_id);
    println!("Status:        pending");
    println!();
    println!("The event will be processed by the webhook processor.");
    println!("Use 'orchestrate webhook list-events' to check status.");

    Ok(())
}

/// Handle webhook status command
async fn handle_webhook_status() -> Result<()> {
    // TODO: Implement actual status check (e.g., check if server is running via PID file)
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               Webhook Server Status                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Status:           Not implemented yet                       ║");
    println!("║                                                              ║");
    println!("║  Use 'orchestrate webhook start' to start the server        ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Handle webhook secret rotate command
async fn handle_webhook_secret_rotate() -> Result<()> {
    use rand::Rng;

    // Generate a random 32-byte secret
    let secret: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               New Webhook Secret Generated                   ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║                                                              ║");
    println!("║  {:<60} ║", &secret[..60]);
    println!("║  {:<60} ║", &secret[60..]);
    println!("║                                                              ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Setup Instructions:                                         ║");
    println!("║                                                              ║");
    println!("║  1. Set environment variable:                                ║");
    println!("║     export GITHUB_WEBHOOK_SECRET='<secret>'                  ║");
    println!("║                                                              ║");
    println!("║  2. Update GitHub webhook settings:                          ║");
    println!("║     - Go to repository Settings > Webhooks                   ║");
    println!("║     - Edit webhook > Secret                                  ║");
    println!("║     - Paste the secret above                                 ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Handle webhook secret show command
async fn handle_webhook_secret_show() -> Result<()> {
    let secret = std::env::var("GITHUB_WEBHOOK_SECRET").ok();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║               Current Webhook Secret                         ║");
    println!("╠══════════════════════════════════════════════════════════════╣");

    if let Some(s) = secret {
        // Show only first and last 8 characters for security
        let masked = if s.len() > 16 {
            format!("{}...{}", &s[..8], &s[s.len() - 8..])
        } else {
            "***".to_string()
        };
        println!("║  Secret (masked): {:<43} ║", masked);
    } else {
        println!("║  Status: Not configured                                      ║");
        println!("║                                                              ║");
        println!("║  Set GITHUB_WEBHOOK_SECRET environment variable              ║");
        println!("║  or use 'orchestrate webhook secret rotate' to generate     ║");
    }

    println!("╚══════════════════════════════════════════════════════════════╝");

    Ok(())
}

/// Generate a minimal test payload for simulation
fn generate_test_payload(event_type: &str) -> String {
    match event_type {
        "pull_request.opened" | "pull_request" => {
            r#"{"action":"opened","number":1,"pull_request":{"number":1,"title":"Test PR","head":{"ref":"test-branch"},"base":{"ref":"main"}}}"#.to_string()
        }
        "check_run.completed" | "check_run" => {
            r#"{"action":"completed","check_run":{"id":1,"name":"test","conclusion":"failure","head_sha":"abc123","html_url":"https://github.com/test/test/runs/1"}}"#.to_string()
        }
        "check_suite.completed" | "check_suite" => {
            r#"{"action":"completed","check_suite":{"id":1,"conclusion":"failure","head_sha":"abc123","head_branch":"main"}}"#.to_string()
        }
        "push" => {
            r#"{"ref":"refs/heads/main","before":"abc123","after":"def456","commits":[]}"#.to_string()
        }
        "pull_request_review.submitted" | "pull_request_review" => {
            r#"{"action":"submitted","pull_request":{"number":1},"review":{"state":"changes_requested","body":"Please fix"}}"#.to_string()
        }
        "issues.opened" | "issues" => {
            r#"{"action":"opened","issue":{"number":1,"title":"Test Issue","body":"Test body"}}"#.to_string()
        }
        _ => {
            format!(r#"{{"action":"test","event_type":"{}"}}"#, event_type)
        }
    }
}

// ==================== Pipeline Command Handlers ====================

async fn handle_pipeline_create(db: &Database, file: &PathBuf) -> Result<()> {
    use orchestrate_core::Pipeline;
    use std::fs;

    // Read YAML file
    let yaml = fs::read_to_string(file)?;

    // Try to parse pipeline name from YAML (simple approach - look for "name:" line)
    let name = yaml
        .lines()
        .find(|line| line.trim_start().starts_with("name:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|s| s.trim().trim_matches('"').to_string())
        .ok_or_else(|| anyhow::anyhow!("Pipeline YAML must contain 'name' field"))?;

    // Create pipeline (validation will happen when the executor parses it)
    let pipeline = Pipeline::new(name.clone(), yaml);
    db.insert_pipeline(&pipeline).await?;

    println!("Pipeline created: {}", name);
    println!("  File: {:?}", file);
    println!("  Note: Pipeline definition will be validated on first run");

    Ok(())
}

async fn handle_pipeline_list(db: &Database, enabled_only: bool) -> Result<()> {
    let pipelines = if enabled_only {
        db.list_enabled_pipelines().await?
    } else {
        db.list_pipelines().await?
    };

    if pipelines.is_empty() {
        println!("No pipelines found");
        return Ok(());
    }

    println!("{:<30} {:<10} {:<20}", "NAME", "ENABLED", "CREATED");
    println!("{}", "-".repeat(70));

    for pipeline in pipelines {
        let enabled_str = if pipeline.enabled { "yes" } else { "no" };
        let created = pipeline.created_at.format("%Y-%m-%d %H:%M:%S");
        println!("{:<30} {:<10} {:<20}", pipeline.name, enabled_str, created);
    }

    Ok(())
}

async fn handle_pipeline_show(db: &Database, name: &str) -> Result<()> {
    let pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    println!("Pipeline: {}", pipeline.name);
    println!("Enabled: {}", if pipeline.enabled { "yes" } else { "no" });
    println!("Created: {}", pipeline.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("\nDefinition:");
    println!("{}", pipeline.definition);

    Ok(())
}

async fn handle_pipeline_update(db: &Database, name: &str, file: &PathBuf) -> Result<()> {
    use std::fs;

    // Get existing pipeline
    let mut pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    // Read new YAML file
    let yaml = fs::read_to_string(file)?;

    // Update pipeline (validation will happen when the executor parses it)
    pipeline.definition = yaml;
    db.update_pipeline(&pipeline).await?;

    println!("Pipeline updated: {}", name);
    println!("  Note: Pipeline definition will be validated on first run");

    Ok(())
}

async fn handle_pipeline_delete(db: &Database, name: &str) -> Result<()> {
    let pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    db.delete_pipeline(pipeline.id.unwrap()).await?;
    println!("Pipeline deleted: {}", name);

    Ok(())
}

async fn handle_pipeline_enable(db: &Database, name: &str) -> Result<()> {
    let mut pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    pipeline.enabled = true;
    db.update_pipeline(&pipeline).await?;
    println!("Pipeline enabled: {}", name);

    Ok(())
}

async fn handle_pipeline_disable(db: &Database, name: &str) -> Result<()> {
    let mut pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    pipeline.enabled = false;
    db.update_pipeline(&pipeline).await?;
    println!("Pipeline disabled: {}", name);

    Ok(())
}

async fn handle_pipeline_run(db: &Database, name: &str, dry_run: bool) -> Result<()> {
    use orchestrate_core::PipelineRun;

    let pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    if dry_run {
        println!("Dry run: Would trigger pipeline '{}'", name);
        println!("Pipeline definition:");
        println!("{}", pipeline.definition);
        return Ok(());
    }

    // Create pipeline run
    let run = PipelineRun::new(pipeline.id.unwrap(), Some("manual".to_string()));
    let run_id = db.insert_pipeline_run(&run).await?;

    println!("Pipeline run started: {}", run_id);
    println!("  Pipeline: {}", name);
    println!("  Trigger: manual");
    println!("\nNote: Pipeline execution requires the daemon to be running.");
    println!("Use 'orchestrate pipeline status {}' to check progress", run_id);

    Ok(())
}

async fn handle_pipeline_status(db: &Database, run_id: i64) -> Result<()> {
    use orchestrate_core::PipelineRunStatus;

    let run = db
        .get_pipeline_run(run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline run not found: {}", run_id))?;

    let pipeline = db.get_pipeline(run.pipeline_id).await?.ok_or_else(|| {
        anyhow::anyhow!("Pipeline not found for run {}", run_id)
    })?;

    println!("Pipeline Run: {}", run_id);
    println!("  Pipeline: {}", pipeline.name);
    println!("  Status: {:?}", run.status);
    println!("  Trigger: {}", run.trigger_event.as_deref().unwrap_or("unknown"));

    if let Some(started) = run.started_at {
        println!("  Started: {}", started.format("%Y-%m-%d %H:%M:%S"));
    }

    if let Some(completed) = run.completed_at {
        println!("  Completed: {}", completed.format("%Y-%m-%d %H:%M:%S"));
        if let Some(started) = run.started_at {
            let duration = completed - started;
            println!("  Duration: {}s", duration.num_seconds());
        }
    }

    // Show stages
    let stages = db.list_pipeline_stages(run_id).await?;
    if !stages.is_empty() {
        println!("\nStages:");
        for stage in stages {
            let status_str = format!("{:?}", stage.status);
            let agent_str = stage.agent_id.as_deref().unwrap_or("N/A");
            println!("  - {}: {} (agent: {})", stage.stage_name, status_str, agent_str);
        }
    }

    Ok(())
}

async fn handle_pipeline_cancel(db: &Database, run_id: i64) -> Result<()> {
    let mut run = db
        .get_pipeline_run(run_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline run not found: {}", run_id))?;

    run.mark_cancelled();
    db.update_pipeline_run(&run).await?;

    println!("Pipeline run cancelled: {}", run_id);

    Ok(())
}

async fn handle_pipeline_history(db: &Database, name: &str, limit: usize) -> Result<()> {
    let pipeline = db
        .get_pipeline_by_name(name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Pipeline not found: {}", name))?;

    let mut runs = db.list_pipeline_runs(pipeline.id.unwrap()).await?;

    // Sort by created_at descending (newest first)
    runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    // Limit results
    let runs: Vec<_> = runs.into_iter().take(limit).collect();

    if runs.is_empty() {
        println!("No runs found for pipeline: {}", name);
        return Ok(());
    }

    println!("Pipeline: {}", name);
    println!("\n{:<10} {:<15} {:<20} {:<20} {:<10}", "RUN ID", "STATUS", "STARTED", "COMPLETED", "DURATION");
    println!("{}", "-".repeat(90));

    for run in runs {
        let status = format!("{:?}", run.status);
        let started = run
            .started_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());
        let completed = run
            .completed_at
            .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());

        let duration = if let (Some(start), Some(end)) = (run.started_at, run.completed_at) {
            format!("{}s", (end - start).num_seconds())
        } else {
            "-".to_string()
        };

        println!(
            "{:<10} {:<15} {:<20} {:<20} {:<10}",
            run.id.unwrap_or(0),
            status,
            started,
            completed,
            duration
        );
    }

    Ok(())
}

fn handle_pipeline_init(
    template: Option<&str>,
    output: Option<&PathBuf>,
    list: bool,
    force: bool,
) -> Result<()> {
    use orchestrate_core::pipeline_template;
    use std::fs;

    // Handle --list flag
    if list {
        println!("Available pipeline templates:");
        println!();

        let templates = pipeline_template::get_templates();
        let mut names: Vec<_> = templates.keys().collect();
        names.sort();

        for name in names {
            let template = &templates[name];
            println!("  {} - {}", name, template.description);
        }

        return Ok(());
    }

    // Template name is required if not listing
    let template_name = template
        .ok_or_else(|| anyhow::anyhow!("Template name required. Use --list to see available templates"))?;

    // Get template
    let template = pipeline_template::get_template(template_name)
        .ok_or_else(|| anyhow::anyhow!("Template not found: {}. Use --list to see available templates", template_name))?;

    // Determine output file
    let output_file = if let Some(path) = output {
        path.clone()
    } else {
        // Default: <template-name>-pipeline.yaml
        PathBuf::from(format!("{}-pipeline.yaml", template_name))
    };

    // Check if file exists
    if output_file.exists() && !force {
        return Err(anyhow::anyhow!(
            "File already exists: {}. Use --force to overwrite",
            output_file.display()
        ));
    }

    // Write template to file
    fs::write(&output_file, &template.yaml)?;

    println!("Pipeline template '{}' initialized successfully!", template_name);
    println!("  File: {}", output_file.display());
    println!("  Description: {}", template.description);
    println!();
    println!("Next steps:");
    println!("  1. Review and customize the pipeline definition");
    println!("  2. Create the pipeline: orchestrate pipeline create {}", output_file.display());

    Ok(())
}

// ==================== Approval Command Handlers ====================

async fn handle_approval_list(db: &Database, pending_only: bool) -> Result<()> {
    use orchestrate_core::ApprovalStatus;

    let approvals = if pending_only {
        db.list_pending_approvals().await?
    } else {
        // For now, just list pending. In the future, add a list_all_approvals method
        db.list_pending_approvals().await?
    };

    if approvals.is_empty() {
        println!("No approval requests found");
        return Ok(());
    }

    println!("{:<10} {:<10} {:<15} {:<30} {:<20}", "ID", "RUN ID", "STATUS", "APPROVERS", "CREATED");
    println!("{}", "-".repeat(95));

    for approval in approvals {
        let status = format!("{:?}", approval.status);
        let approvers = &approval.required_approvers;
        let created = approval.created_at.format("%Y-%m-%d %H:%M:%S");

        println!(
            "{:<10} {:<10} {:<15} {:<30} {:<20}",
            approval.id.unwrap_or(0),
            approval.run_id,
            status,
            approvers,
            created
        );
    }

    Ok(())
}

async fn handle_approval_approve(db: &Database, id: i64, comment: Option<&str>) -> Result<()> {
    use orchestrate_core::ApprovalDecision;

    // Get approval request
    let mut approval = db
        .get_approval_request(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Approval request not found: {}", id))?;

    // Get current user (for now, use a placeholder)
    let approver = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Create decision
    let decision = ApprovalDecision::new(id, approver.clone(), true, comment.map(String::from));
    db.create_approval_decision(decision).await?;

    // Update approval count
    approval.approval_count += 1;

    // Check if quorum reached
    if approval.has_approval_quorum() {
        approval.mark_approved();
        println!("Approval request {} APPROVED by {}", id, approver);
        println!("  Quorum reached: {}/{}", approval.approval_count, approval.required_count);
    } else {
        println!("Approval recorded from {}", approver);
        println!("  Progress: {}/{}", approval.approval_count, approval.required_count);
    }

    db.update_approval_request(&approval).await?;

    Ok(())
}

async fn handle_approval_reject(db: &Database, id: i64, reason: Option<&str>) -> Result<()> {
    use orchestrate_core::ApprovalDecision;

    // Get approval request
    let mut approval = db
        .get_approval_request(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Approval request not found: {}", id))?;

    // Get current user
    let approver = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Create decision
    let decision = ApprovalDecision::new(id, approver.clone(), false, reason.map(String::from));
    db.create_approval_decision(decision).await?;

    // Update rejection count
    approval.rejection_count += 1;

    // Check if rejection quorum reached
    if approval.has_rejection_quorum() {
        approval.mark_rejected();
        println!("Approval request {} REJECTED by {}", id, approver);
        if let Some(r) = reason {
            println!("  Reason: {}", r);
        }
    } else {
        println!("Rejection recorded from {}", approver);
        println!("  Rejections: {}", approval.rejection_count);
    }

    db.update_approval_request(&approval).await?;

    Ok(())
}

async fn handle_approval_delegate(db: &Database, id: i64, to: &str) -> Result<()> {
    // Get approval request
    let mut approval = db
        .get_approval_request(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Approval request not found: {}", id))?;

    // Update approvers list
    approval.required_approvers = to.to_string();
    approval.mark_delegated();

    db.update_approval_request(&approval).await?;

    println!("Approval request {} delegated to {}", id, to);

    Ok(())
}

// ==================== Feedback Handlers ====================

async fn handle_feedback_add(
    db: &Database,
    agent_id: &str,
    rating: &str,
    comment: Option<&str>,
    message_id: Option<i64>,
) -> Result<()> {
    use orchestrate_core::{Feedback, FeedbackRating, FeedbackSource};
    use std::str::FromStr;

    // Parse agent ID
    let agent_uuid = Uuid::parse_str(agent_id)
        .map_err(|e| anyhow::anyhow!("Invalid agent ID '{}': {}", agent_id, e))?;

    // Verify agent exists
    if db.get_agent(agent_uuid).await?.is_none() {
        anyhow::bail!("Agent not found: {}", agent_id);
    }

    // Parse rating
    let feedback_rating = FeedbackRating::from_str(rating)
        .map_err(|_| anyhow::anyhow!("Invalid rating '{}'. Use: positive, negative, neutral, +, -, pos, neg", rating))?;

    // Get current user
    let created_by = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());

    // Build feedback
    let mut feedback = Feedback::new(agent_uuid, feedback_rating, created_by)
        .with_source(FeedbackSource::Cli);

    if let Some(msg_id) = message_id {
        feedback = feedback.with_message_id(msg_id);
    }

    if let Some(c) = comment {
        feedback = feedback.with_comment(c);
    }

    // Insert feedback
    let id = db.insert_feedback(&feedback).await?;

    println!("Feedback added successfully (ID: {})", id);
    println!("  Agent: {}", agent_id);
    println!("  Rating: {}", feedback_rating);
    if let Some(c) = comment {
        println!("  Comment: {}", c);
    }

    Ok(())
}

async fn handle_feedback_list(
    db: &Database,
    agent: Option<&str>,
    rating: Option<&str>,
    source: Option<&str>,
    limit: i64,
) -> Result<()> {
    use orchestrate_core::{FeedbackRating, FeedbackSource};
    use std::str::FromStr;

    let feedbacks = if let Some(agent_id) = agent {
        let agent_uuid = Uuid::parse_str(agent_id)
            .map_err(|e| anyhow::anyhow!("Invalid agent ID '{}': {}", agent_id, e))?;
        db.list_feedback_for_agent(agent_uuid, limit).await?
    } else {
        let rating_filter = rating
            .map(|r| FeedbackRating::from_str(r))
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid rating filter"))?;
        let source_filter = source
            .map(|s| FeedbackSource::from_str(s))
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid source filter"))?;
        db.list_feedback(rating_filter, source_filter, limit).await?
    };

    if feedbacks.is_empty() {
        println!("No feedback found");
        return Ok(());
    }

    println!("Feedback ({} entries):", feedbacks.len());
    println!("{:-<80}", "");

    for fb in feedbacks {
        let rating_symbol = match fb.rating {
            orchestrate_core::FeedbackRating::Positive => "+",
            orchestrate_core::FeedbackRating::Negative => "-",
            orchestrate_core::FeedbackRating::Neutral => "0",
        };
        println!(
            "[{}] ID: {} | Agent: {} | {} | by {} | {}",
            rating_symbol,
            fb.id,
            &fb.agent_id.to_string()[..8],
            fb.source,
            fb.created_by,
            fb.created_at.format("%Y-%m-%d %H:%M")
        );
        if let Some(comment) = &fb.comment {
            println!("    Comment: {}", comment);
        }
    }

    Ok(())
}

async fn handle_feedback_stats(
    db: &Database,
    agent: Option<&str>,
    by_type: bool,
) -> Result<()> {
    if let Some(agent_id) = agent {
        let agent_uuid = Uuid::parse_str(agent_id)
            .map_err(|e| anyhow::anyhow!("Invalid agent ID '{}': {}", agent_id, e))?;
        let stats = db.get_feedback_stats_for_agent(agent_uuid).await?;

        println!("Feedback Stats for Agent {}", agent_id);
        println!("{:-<50}", "");
        print_feedback_stats(&stats);
    } else if by_type {
        let stats_by_type = db.get_feedback_stats_by_agent_type().await?;

        if stats_by_type.is_empty() {
            println!("No feedback statistics available");
            return Ok(());
        }

        println!("Feedback Stats by Agent Type");
        println!("{:-<60}", "");

        for (agent_type, stats) in stats_by_type {
            println!("\n{}", agent_type.as_str());
            print_feedback_stats(&stats);
        }
    } else {
        let stats = db.get_feedback_stats().await?;

        println!("Overall Feedback Stats");
        println!("{:-<50}", "");
        print_feedback_stats(&stats);
    }

    Ok(())
}

fn print_feedback_stats(stats: &orchestrate_core::FeedbackStats) {
    println!("  Total: {}", stats.total);
    println!(
        "  Positive: {} ({:.1}%)",
        stats.positive, stats.positive_percentage
    );
    println!("  Negative: {}", stats.negative);
    println!("  Neutral: {}", stats.neutral);
    println!("  Score: {:.2}", stats.score);
}

async fn handle_feedback_delete(db: &Database, id: i64) -> Result<()> {
    if db.delete_feedback(id).await? {
        println!("Feedback {} deleted", id);
    } else {
        println!("Feedback {} not found", id);
    }
    Ok(())
}

// ==================== Environment Handlers ====================

async fn handle_env_list(db: &Database, format: &str) -> Result<()> {
    let envs = db.list_environments().await?;

    if envs.is_empty() {
        println!("No environments found");
        return Ok(());
    }

    match format {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&envs)?);
        }
        _ => {
            // Table format
            println!("{:<20} {:<15} {:<40} {:<10}", "NAME", "TYPE", "URL", "APPROVAL");
            println!("{:-<90}", "");
            for env in envs {
                println!(
                    "{:<20} {:<15} {:<40} {:<10}",
                    env.name,
                    env.env_type.to_string(),
                    env.url.as_deref().unwrap_or("-"),
                    if env.requires_approval { "required" } else { "-" }
                );
            }
        }
    }

    Ok(())
}

async fn handle_env_create(
    db: &Database,
    name: &str,
    env_type: &str,
    url: Option<&str>,
    provider: Option<&str>,
    config_json: Option<&str>,
    secrets_json: Option<&str>,
    requires_approval: bool,
) -> Result<()> {
    use std::str::FromStr;

    // Parse environment type
    let env_type = EnvironmentType::from_str(env_type)?;

    // Parse config JSON
    let config = if let Some(json) = config_json {
        serde_json::from_str(json)?
    } else {
        std::collections::HashMap::new()
    };

    // Parse secrets JSON
    let secrets = if let Some(json) = secrets_json {
        serde_json::from_str(json)?
    } else {
        std::collections::HashMap::new()
    };

    let create_env = CreateEnvironment {
        name: name.to_string(),
        env_type,
        url: url.map(String::from),
        provider: provider.map(String::from),
        config,
        secrets,
        requires_approval,
    };

    let env = db.create_environment(create_env).await?;

    println!("Environment '{}' created successfully", env.name);
    println!("  ID: {}", env.id);
    println!("  Type: {}", env.env_type);
    if let Some(url) = &env.url {
        println!("  URL: {}", url);
    }
    if let Some(provider) = &env.provider {
        println!("  Provider: {}", provider);
    }
    println!("  Requires approval: {}", env.requires_approval);
    println!("  Config keys: {}", env.config.len());
    println!("  Secrets: {} (encrypted)", env.secrets.len());

    Ok(())
}

async fn handle_env_show(
    db: &Database,
    name: &str,
    show_secrets: bool,
    format: &str,
) -> Result<()> {
    let env = db.get_environment_by_name(name).await?;

    match format {
        "json" => {
            if show_secrets {
                println!("{}", serde_json::to_string_pretty(&env)?);
            } else {
                // Create a version without secrets
                let mut env_no_secrets = env.clone();
                env_no_secrets.secrets = std::collections::HashMap::new();
                println!("{}", serde_json::to_string_pretty(&env_no_secrets)?);
            }
        }
        _ => {
            // Table format
            println!("Environment: {}", env.name);
            println!("{:-<60}", "");
            println!("  ID: {}", env.id);
            println!("  Type: {}", env.env_type);
            if let Some(url) = &env.url {
                println!("  URL: {}", url);
            }
            if let Some(provider) = &env.provider {
                println!("  Provider: {}", provider);
            }
            println!("  Requires approval: {}", env.requires_approval);
            println!("  Created: {}", env.created_at.format("%Y-%m-%d %H:%M:%S"));

            if !env.config.is_empty() {
                println!("\n  Configuration:");
                for (key, value) in &env.config {
                    println!("    {}: {}", key, value);
                }
            }

            if !env.secrets.is_empty() {
                println!("\n  Secrets:");
                for key in env.secrets.keys() {
                    if show_secrets {
                        println!("    {}: {}", key, env.secrets.get(key).unwrap());
                    } else {
                        println!("    {}: ********", key);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_env_delete(db: &Database, name: &str, yes: bool) -> Result<()> {
    // Check if environment exists
    let _ = db.get_environment_by_name(name).await?;

    if !yes {
        println!("Are you sure you want to delete environment '{}'? [y/N]", name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    if db.delete_environment(name).await? {
        println!("Environment '{}' deleted", name);
    } else {
        println!("Environment '{}' not found", name);
    }

    Ok(())
}

async fn handle_deploy_validate(
    db: &Database,
    env: &str,
    version: Option<&str>,
    format: &str,
) -> Result<()> {
    let validator = PreDeployValidator::with_db(Arc::new(db.clone()));
    let validation = validator.validate(env, version).await?;

    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&validation)?);
        return Ok(());
    }

    // Table format
    println!("Pre-Deployment Validation for '{}'", env);
    if let Some(v) = &validation.version {
        println!("Version: {}", v);
    }
    println!("Validated at: {}", validation.validated_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();

    println!("Overall Status: {}", match validation.overall_status {
        ValidationStatus::Passed => "PASSED",
        ValidationStatus::Failed => "FAILED",
        ValidationStatus::Warning => "WARNING",
        ValidationStatus::Skipped => "SKIPPED",
    });
    println!();

    println!("Validation Checks:");
    println!("{:<30} {:<10} {}", "Check", "Status", "Message");
    println!("{}", "-".repeat(80));

    for check in &validation.checks {
        let status_symbol = match check.status {
            ValidationStatus::Passed => "✓ PASS",
            ValidationStatus::Failed => "✗ FAIL",
            ValidationStatus::Warning => "⚠ WARN",
            ValidationStatus::Skipped => "- SKIP",
        };
        println!("{:<30} {:<10} {}", check.name, status_symbol, check.message);
    }

    println!();

    if !validation.is_valid() {
        println!("Validation FAILED. The following checks failed:");
        for check in validation.failed_checks() {
            println!("  - {}: {}", check.name, check.message);
        }
        std::process::exit(1);
    }

    println!("All checks passed. Environment is ready for deployment.");
    Ok(())
}

