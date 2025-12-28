//! Orchestrate Web - Web interface
//!
//! This crate provides the web interface:
//! - REST API
//! - WebSocket for real-time updates
//! - Chat interface

pub mod api;
pub mod websocket;

pub use api::create_router;
