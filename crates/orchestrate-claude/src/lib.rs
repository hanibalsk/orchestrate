//! Orchestrate Claude - Claude Agent SDK integration
//!
//! This crate provides integration with the Claude API:
//! - API client
//! - Loop functionality
//! - Tool execution
//! - Session management

pub mod client;
pub mod loop_runner;
pub mod tools;

pub use client::ClaudeClient;
pub use loop_runner::AgentLoop;
