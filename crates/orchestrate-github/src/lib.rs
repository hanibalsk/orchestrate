//! Orchestrate GitHub - GitHub API integration
//!
//! This crate provides GitHub integration:
//! - PR management
//! - Review handling
//! - CI check monitoring

pub mod client;
pub mod pr;
pub mod review;

pub use client::GitHubClient;
