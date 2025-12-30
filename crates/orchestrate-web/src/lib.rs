//! Orchestrate Web - Web interface
//!
//! This crate provides the web interface:
//! - REST API
//! - WebSocket for real-time updates
//! - HTML UI for agent management
//! - Chat interface
//! - GitHub webhook receiver

pub mod api;
pub mod metrics;
pub mod monitoring;
pub mod schedule_executor;
pub mod event_handlers;
pub mod ui;
pub mod webhook;
pub mod webhook_processor;
pub mod websocket;

pub use api::{create_router, create_router_with_webhook};
pub use metrics::MetricsCollector;
pub use schedule_executor::{MissedSchedulePolicy, ScheduleExecutor, ScheduleExecutorConfig};
pub use ui::create_ui_router;
pub use webhook::{WebhookConfig, WebhookState, github_webhook_handler};
pub use webhook_processor::{WebhookProcessor, WebhookProcessorConfig};
