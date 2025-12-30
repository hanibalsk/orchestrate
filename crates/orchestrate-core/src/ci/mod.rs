//! CI/CD Integration Implementation
//!
//! This module provides concrete implementations for CI/CD platform integrations.

pub mod client;
pub mod github_actions;
pub mod gitlab_ci;
pub mod circleci;
pub mod result_parser;
pub mod failure_handler;

pub use client::{CiClient, CiClientTrait};
pub use github_actions::GitHubActionsClient;
pub use gitlab_ci::GitLabCiClient;
pub use circleci::CircleCiClient;
pub use result_parser::{LogParser, TestResultParser};
pub use failure_handler::{FailureHandler, FailureResponse};
