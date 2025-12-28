//! Orchestrate Claude - Claude Agent SDK integration
//!
//! This crate provides integration with the Claude API:
//! - API client with prompt caching support
//! - Token estimation and context management
//! - Message windowing and summarization
//! - Loop functionality with optimizations
//! - Tool execution
//! - Session management

pub mod client;
pub mod loop_runner;
pub mod token;
pub mod tools;

pub use client::{ClaudeClient, ClaudeCliClient};
pub use loop_runner::AgentLoop;
pub use token::{ContextManager, TokenConfig, TokenEstimator};
