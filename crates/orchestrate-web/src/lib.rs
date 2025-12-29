//! Orchestrate Web - Web interface
//!
//! This crate provides the web interface:
//! - REST API
//! - WebSocket for real-time updates
//! - HTML UI for agent management
//! - Chat interface

pub mod api;
pub mod schedule_executor;
pub mod ui;
pub mod websocket;

pub use api::create_router;
pub use schedule_executor::{MissedSchedulePolicy, ScheduleExecutor, ScheduleExecutorConfig};
pub use ui::create_ui_router;
