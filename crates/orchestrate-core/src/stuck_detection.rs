//! Stuck Agent Detection
//!
//! Epic 016: Autonomous Epic Processing - Story 6
//!
//! Detects agents that are stuck, making no progress, or hitting limits.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Types of stuck agent situations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StuckType {
    /// Agent approaching max_turns limit
    TurnLimit,
    /// No meaningful output in last N turns
    NoProgress,
    /// CI check not updating for extended period
    CiTimeout,
    /// PR review taking too long (async Copilot reviews)
    ReviewDelay,
    /// PR has merge conflicts
    MergeConflict,
    /// API rate limit encountered
    RateLimit,
    /// Approaching context token limit
    ContextLimit,
    /// Agent in error loop
    ErrorLoop,
}

impl StuckType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TurnLimit => "turn_limit",
            Self::NoProgress => "no_progress",
            Self::CiTimeout => "ci_timeout",
            Self::ReviewDelay => "review_delay",
            Self::MergeConflict => "merge_conflict",
            Self::RateLimit => "rate_limit",
            Self::ContextLimit => "context_limit",
            Self::ErrorLoop => "error_loop",
        }
    }
}

impl std::str::FromStr for StuckType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "turn_limit" => Ok(Self::TurnLimit),
            "no_progress" => Ok(Self::NoProgress),
            "ci_timeout" => Ok(Self::CiTimeout),
            "review_delay" => Ok(Self::ReviewDelay),
            "merge_conflict" => Ok(Self::MergeConflict),
            "rate_limit" => Ok(Self::RateLimit),
            "context_limit" => Ok(Self::ContextLimit),
            "error_loop" => Ok(Self::ErrorLoop),
            _ => Err(crate::Error::Other(format!("Invalid stuck type: {}", s))),
        }
    }
}

impl std::fmt::Display for StuckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Severity of a stuck detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StuckSeverity {
    /// Minor issue, can likely self-resolve
    Low,
    /// Moderate issue, needs monitoring
    Medium,
    /// Serious issue, may need intervention
    High,
    /// Critical issue, immediate action required
    Critical,
}

impl StuckSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

impl std::str::FromStr for StuckSeverity {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            _ => Err(crate::Error::Other(format!("Invalid severity: {}", s))),
        }
    }
}

impl std::fmt::Display for StuckSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A stuck agent detection event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckDetection {
    pub id: i64,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub detection_type: StuckType,
    pub severity: StuckSeverity,
    pub details: serde_json::Value,
    pub resolved: bool,
    pub resolution_action: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl StuckDetection {
    pub fn new(agent_id: impl Into<String>, detection_type: StuckType, severity: StuckSeverity) -> Self {
        Self {
            id: 0,
            agent_id: agent_id.into(),
            session_id: None,
            detection_type,
            severity,
            details: serde_json::json!({}),
            resolved: false,
            resolution_action: None,
            detected_at: Utc::now(),
            resolved_at: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    pub fn resolve(&mut self, action: impl Into<String>) {
        self.resolved = true;
        self.resolution_action = Some(action.into());
        self.resolved_at = Some(Utc::now());
    }
}

/// Progress metrics for an agent
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentProgress {
    /// Current turn number
    pub turn_count: u32,
    /// Maximum allowed turns
    pub max_turns: u32,
    /// Current token count
    pub token_count: u64,
    /// Maximum tokens allowed
    pub max_tokens: u64,
    /// Last time meaningful output was produced
    pub last_meaningful_output: Option<DateTime<Utc>>,
    /// Number of errors in recent turns
    pub recent_error_count: u32,
    /// Last CI check update time
    pub last_ci_update: Option<DateTime<Utc>>,
    /// Last PR review update time
    pub last_review_update: Option<DateTime<Utc>>,
    /// Whether PR has merge conflicts
    pub has_merge_conflicts: bool,
    /// Recent rate limit encountered
    pub rate_limited_until: Option<DateTime<Utc>>,
}

impl AgentProgress {
    pub fn new(max_turns: u32, max_tokens: u64) -> Self {
        Self {
            max_turns,
            max_tokens,
            ..Default::default()
        }
    }

    /// Get percentage of turns used
    pub fn turn_percentage(&self) -> f64 {
        if self.max_turns == 0 {
            return 0.0;
        }
        (self.turn_count as f64 / self.max_turns as f64) * 100.0
    }

    /// Get percentage of tokens used
    pub fn token_percentage(&self) -> f64 {
        if self.max_tokens == 0 {
            return 0.0;
        }
        (self.token_count as f64 / self.max_tokens as f64) * 100.0
    }
}

/// Configuration for stuck detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckDetectionConfig {
    /// Alert when turn count reaches this percentage of max_turns
    pub turn_warning_threshold: f64,
    /// Alert when token count reaches this percentage of max_tokens
    pub token_warning_threshold: f64,
    /// Number of turns without meaningful output to consider stuck
    pub no_progress_turn_threshold: u32,
    /// Minutes without CI update to consider stuck
    pub ci_timeout_minutes: u32,
    /// Minutes without review update to consider stuck
    pub review_delay_minutes: u32,
    /// Number of errors in sequence to consider stuck
    pub error_loop_threshold: u32,
}

impl Default for StuckDetectionConfig {
    fn default() -> Self {
        Self {
            turn_warning_threshold: 80.0,
            token_warning_threshold: 85.0,
            no_progress_turn_threshold: 5,
            ci_timeout_minutes: 30,
            review_delay_minutes: 60,
            error_loop_threshold: 3,
        }
    }
}

/// Stuck agent detector
#[derive(Debug, Clone)]
pub struct StuckDetector {
    config: StuckDetectionConfig,
}

impl StuckDetector {
    pub fn new() -> Self {
        Self {
            config: StuckDetectionConfig::default(),
        }
    }

    pub fn with_config(config: StuckDetectionConfig) -> Self {
        Self { config }
    }

    /// Check agent progress and return any stuck detections
    pub fn check(&self, agent_id: &str, progress: &AgentProgress) -> Vec<StuckDetection> {
        let mut detections = Vec::new();

        // Check turn limit
        if let Some(detection) = self.check_turn_limit(agent_id, progress) {
            detections.push(detection);
        }

        // Check token/context limit
        if let Some(detection) = self.check_context_limit(agent_id, progress) {
            detections.push(detection);
        }

        // Check for no progress
        if let Some(detection) = self.check_no_progress(agent_id, progress) {
            detections.push(detection);
        }

        // Check CI timeout
        if let Some(detection) = self.check_ci_timeout(agent_id, progress) {
            detections.push(detection);
        }

        // Check review delay
        if let Some(detection) = self.check_review_delay(agent_id, progress) {
            detections.push(detection);
        }

        // Check merge conflicts
        if let Some(detection) = self.check_merge_conflict(agent_id, progress) {
            detections.push(detection);
        }

        // Check rate limit
        if let Some(detection) = self.check_rate_limit(agent_id, progress) {
            detections.push(detection);
        }

        // Check error loop
        if let Some(detection) = self.check_error_loop(agent_id, progress) {
            detections.push(detection);
        }

        detections
    }

    fn check_turn_limit(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        let percentage = progress.turn_percentage();
        if percentage >= self.config.turn_warning_threshold {
            let severity = if percentage >= 95.0 {
                StuckSeverity::Critical
            } else if percentage >= 90.0 {
                StuckSeverity::High
            } else {
                StuckSeverity::Medium
            };

            Some(
                StuckDetection::new(agent_id, StuckType::TurnLimit, severity).with_details(
                    serde_json::json!({
                        "turn_count": progress.turn_count,
                        "max_turns": progress.max_turns,
                        "percentage": percentage,
                    }),
                ),
            )
        } else {
            None
        }
    }

    fn check_context_limit(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        let percentage = progress.token_percentage();
        if percentage >= self.config.token_warning_threshold {
            let severity = if percentage >= 95.0 {
                StuckSeverity::Critical
            } else if percentage >= 90.0 {
                StuckSeverity::High
            } else {
                StuckSeverity::Medium
            };

            Some(
                StuckDetection::new(agent_id, StuckType::ContextLimit, severity).with_details(
                    serde_json::json!({
                        "token_count": progress.token_count,
                        "max_tokens": progress.max_tokens,
                        "percentage": percentage,
                    }),
                ),
            )
        } else {
            None
        }
    }

    fn check_no_progress(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if let Some(last_output) = progress.last_meaningful_output {
            let now = Utc::now();
            let turns_since_progress = progress.turn_count; // Simplified - in real impl, track turns since last output

            // Check if enough turns have passed without meaningful output
            // Using time as a proxy (each turn is ~1 minute average)
            let minutes_since = (now - last_output).num_minutes() as u32;
            if minutes_since >= self.config.no_progress_turn_threshold {
                let severity = if minutes_since >= self.config.no_progress_turn_threshold * 3 {
                    StuckSeverity::High
                } else {
                    StuckSeverity::Medium
                };

                return Some(
                    StuckDetection::new(agent_id, StuckType::NoProgress, severity).with_details(
                        serde_json::json!({
                            "minutes_since_progress": minutes_since,
                            "threshold_minutes": self.config.no_progress_turn_threshold,
                            "last_output": last_output.to_rfc3339(),
                        }),
                    ),
                );
            }
        }
        None
    }

    fn check_ci_timeout(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if let Some(last_update) = progress.last_ci_update {
            let minutes_since = (Utc::now() - last_update).num_minutes() as u32;
            if minutes_since >= self.config.ci_timeout_minutes {
                let severity = if minutes_since >= self.config.ci_timeout_minutes * 2 {
                    StuckSeverity::High
                } else {
                    StuckSeverity::Medium
                };

                return Some(
                    StuckDetection::new(agent_id, StuckType::CiTimeout, severity).with_details(
                        serde_json::json!({
                            "minutes_since_update": minutes_since,
                            "timeout_threshold": self.config.ci_timeout_minutes,
                            "last_update": last_update.to_rfc3339(),
                        }),
                    ),
                );
            }
        }
        None
    }

    fn check_review_delay(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if let Some(last_update) = progress.last_review_update {
            let minutes_since = (Utc::now() - last_update).num_minutes() as u32;
            if minutes_since >= self.config.review_delay_minutes {
                let severity = if minutes_since >= self.config.review_delay_minutes * 2 {
                    StuckSeverity::High
                } else {
                    StuckSeverity::Medium
                };

                return Some(
                    StuckDetection::new(agent_id, StuckType::ReviewDelay, severity).with_details(
                        serde_json::json!({
                            "minutes_since_update": minutes_since,
                            "timeout_threshold": self.config.review_delay_minutes,
                            "last_update": last_update.to_rfc3339(),
                        }),
                    ),
                );
            }
        }
        None
    }

    fn check_merge_conflict(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if progress.has_merge_conflicts {
            Some(
                StuckDetection::new(agent_id, StuckType::MergeConflict, StuckSeverity::High)
                    .with_details(serde_json::json!({
                        "has_conflicts": true,
                    })),
            )
        } else {
            None
        }
    }

    fn check_rate_limit(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if let Some(until) = progress.rate_limited_until {
            if until > Utc::now() {
                let wait_minutes = (until - Utc::now()).num_minutes();
                let severity = if wait_minutes > 30 {
                    StuckSeverity::High
                } else if wait_minutes > 10 {
                    StuckSeverity::Medium
                } else {
                    StuckSeverity::Low
                };

                return Some(
                    StuckDetection::new(agent_id, StuckType::RateLimit, severity).with_details(
                        serde_json::json!({
                            "rate_limited_until": until.to_rfc3339(),
                            "wait_minutes": wait_minutes,
                        }),
                    ),
                );
            }
        }
        None
    }

    fn check_error_loop(&self, agent_id: &str, progress: &AgentProgress) -> Option<StuckDetection> {
        if progress.recent_error_count >= self.config.error_loop_threshold {
            let severity = if progress.recent_error_count >= self.config.error_loop_threshold * 2 {
                StuckSeverity::Critical
            } else {
                StuckSeverity::High
            };

            Some(
                StuckDetection::new(agent_id, StuckType::ErrorLoop, severity).with_details(
                    serde_json::json!({
                        "error_count": progress.recent_error_count,
                        "threshold": self.config.error_loop_threshold,
                    }),
                ),
            )
        } else {
            None
        }
    }
}

impl Default for StuckDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Work evaluation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationStatus {
    /// Agent is healthy and making progress
    Healthy,
    /// Agent has warnings but can continue
    Warning,
    /// Agent is stuck and needs intervention
    Stuck,
    /// Agent has failed
    Failed,
    /// Work is complete
    Complete,
}

impl EvaluationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Warning => "warning",
            Self::Stuck => "stuck",
            Self::Failed => "failed",
            Self::Complete => "complete",
        }
    }
}

impl std::str::FromStr for EvaluationStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "healthy" => Ok(Self::Healthy),
            "warning" => Ok(Self::Warning),
            "stuck" => Ok(Self::Stuck),
            "failed" => Ok(Self::Failed),
            "complete" => Ok(Self::Complete),
            _ => Err(crate::Error::Other(format!("Invalid evaluation status: {}", s))),
        }
    }
}

impl std::fmt::Display for EvaluationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type of work evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationType {
    /// Progress check
    Progress,
    /// Completion check
    Completion,
    /// Stuck agent check
    StuckCheck,
    /// Review outcome check
    ReviewOutcome,
    /// CI status check
    CiStatus,
}

impl EvaluationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Progress => "progress",
            Self::Completion => "completion",
            Self::StuckCheck => "stuck_check",
            Self::ReviewOutcome => "review_outcome",
            Self::CiStatus => "ci_status",
        }
    }
}

impl std::str::FromStr for EvaluationType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "progress" => Ok(Self::Progress),
            "completion" => Ok(Self::Completion),
            "stuck_check" => Ok(Self::StuckCheck),
            "review_outcome" => Ok(Self::ReviewOutcome),
            "ci_status" => Ok(Self::CiStatus),
            _ => Err(crate::Error::Other(format!("Invalid evaluation type: {}", s))),
        }
    }
}

impl std::fmt::Display for EvaluationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A work evaluation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkEvaluation {
    pub id: i64,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub story_id: Option<String>,
    pub evaluation_type: EvaluationType,
    pub status: EvaluationStatus,
    pub details: serde_json::Value,
    pub turn_count: Option<u32>,
    pub max_turns: Option<u32>,
    pub token_count: Option<u64>,
    pub max_tokens: Option<u64>,
    pub duration_secs: Option<u64>,
    pub created_at: DateTime<Utc>,
}

impl WorkEvaluation {
    pub fn new(
        agent_id: impl Into<String>,
        evaluation_type: EvaluationType,
        status: EvaluationStatus,
    ) -> Self {
        Self {
            id: 0,
            agent_id: agent_id.into(),
            session_id: None,
            story_id: None,
            evaluation_type,
            status,
            details: serde_json::json!({}),
            turn_count: None,
            max_turns: None,
            token_count: None,
            max_tokens: None,
            duration_secs: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_story(mut self, story_id: impl Into<String>) -> Self {
        self.story_id = Some(story_id.into());
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    pub fn with_progress(mut self, turn_count: u32, max_turns: u32) -> Self {
        self.turn_count = Some(turn_count);
        self.max_turns = Some(max_turns);
        self
    }

    pub fn with_tokens(mut self, token_count: u64, max_tokens: u64) -> Self {
        self.token_count = Some(token_count);
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_duration(mut self, duration_secs: u64) -> Self {
        self.duration_secs = Some(duration_secs);
        self
    }
}

/// Rate limit backoff calculator
#[derive(Debug, Clone)]
pub struct RateLimitBackoff {
    /// Base delay in seconds
    pub base_delay: u64,
    /// Maximum delay in seconds
    pub max_delay: u64,
    /// Current retry count
    pub retry_count: u32,
}

impl Default for RateLimitBackoff {
    fn default() -> Self {
        Self {
            base_delay: 5,
            max_delay: 300, // 5 minutes
            retry_count: 0,
        }
    }
}

impl RateLimitBackoff {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the next backoff delay in seconds using exponential backoff
    pub fn next_delay(&mut self) -> u64 {
        let delay = self.base_delay * 2u64.pow(self.retry_count);
        self.retry_count += 1;
        delay.min(self.max_delay)
    }

    /// Reset the backoff counter
    pub fn reset(&mut self) {
        self.retry_count = 0;
    }

    /// Get the time until which we should wait
    pub fn rate_limited_until(&mut self) -> DateTime<Utc> {
        let delay_secs = self.next_delay() as i64;
        Utc::now() + Duration::seconds(delay_secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stuck_type_roundtrip() {
        let types = [
            StuckType::TurnLimit,
            StuckType::NoProgress,
            StuckType::CiTimeout,
            StuckType::ReviewDelay,
            StuckType::MergeConflict,
            StuckType::RateLimit,
            StuckType::ContextLimit,
            StuckType::ErrorLoop,
        ];

        for t in types {
            let s = t.as_str();
            let parsed: StuckType = s.parse().unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_stuck_severity_ordering() {
        assert!(StuckSeverity::Low < StuckSeverity::Medium);
        assert!(StuckSeverity::Medium < StuckSeverity::High);
        assert!(StuckSeverity::High < StuckSeverity::Critical);
    }

    #[test]
    fn test_stuck_detection_new() {
        let detection = StuckDetection::new("agent-1", StuckType::TurnLimit, StuckSeverity::High);
        assert_eq!(detection.agent_id, "agent-1");
        assert_eq!(detection.detection_type, StuckType::TurnLimit);
        assert_eq!(detection.severity, StuckSeverity::High);
        assert!(!detection.resolved);
    }

    #[test]
    fn test_stuck_detection_with_session() {
        let detection = StuckDetection::new("agent-1", StuckType::CiTimeout, StuckSeverity::Medium)
            .with_session("session-123");
        assert_eq!(detection.session_id, Some("session-123".to_string()));
    }

    #[test]
    fn test_stuck_detection_resolve() {
        let mut detection =
            StuckDetection::new("agent-1", StuckType::MergeConflict, StuckSeverity::High);
        detection.resolve("rebased branch");
        assert!(detection.resolved);
        assert_eq!(detection.resolution_action, Some("rebased branch".to_string()));
        assert!(detection.resolved_at.is_some());
    }

    #[test]
    fn test_agent_progress_percentages() {
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 80;
        progress.token_count = 85000;

        assert_eq!(progress.turn_percentage(), 80.0);
        assert_eq!(progress.token_percentage(), 85.0);
    }

    #[test]
    fn test_agent_progress_zero_max() {
        let progress = AgentProgress::new(0, 0);
        assert_eq!(progress.turn_percentage(), 0.0);
        assert_eq!(progress.token_percentage(), 0.0);
    }

    #[test]
    fn test_detector_turn_limit_warning() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 80; // 80%

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::TurnLimit);
        assert_eq!(detections[0].severity, StuckSeverity::Medium);
    }

    #[test]
    fn test_detector_turn_limit_high() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 92; // 92%

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, StuckSeverity::High);
    }

    #[test]
    fn test_detector_turn_limit_critical() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 96; // 96%

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, StuckSeverity::Critical);
    }

    #[test]
    fn test_detector_no_warning_below_threshold() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 70; // 70%, below 80% threshold

        let detections = detector.check("agent-1", &progress);
        assert!(detections.is_empty());
    }

    #[test]
    fn test_detector_context_limit() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.token_count = 90000; // 90%

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::ContextLimit);
    }

    #[test]
    fn test_detector_merge_conflict() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.has_merge_conflicts = true;

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::MergeConflict);
        assert_eq!(detections[0].severity, StuckSeverity::High);
    }

    #[test]
    fn test_detector_error_loop() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.recent_error_count = 3;

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::ErrorLoop);
        assert_eq!(detections[0].severity, StuckSeverity::High);
    }

    #[test]
    fn test_detector_error_loop_critical() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.recent_error_count = 6; // Double the threshold

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].severity, StuckSeverity::Critical);
    }

    #[test]
    fn test_detector_rate_limit() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.rate_limited_until = Some(Utc::now() + Duration::minutes(15));

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::RateLimit);
        assert_eq!(detections[0].severity, StuckSeverity::Medium);
    }

    #[test]
    fn test_detector_rate_limit_expired() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.rate_limited_until = Some(Utc::now() - Duration::minutes(5)); // Expired

        let detections = detector.check("agent-1", &progress);
        assert!(detections.is_empty());
    }

    #[test]
    fn test_detector_multiple_issues() {
        let detector = StuckDetector::new();
        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 85;
        progress.has_merge_conflicts = true;
        progress.recent_error_count = 4;

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 3);

        let types: Vec<_> = detections.iter().map(|d| d.detection_type).collect();
        assert!(types.contains(&StuckType::TurnLimit));
        assert!(types.contains(&StuckType::MergeConflict));
        assert!(types.contains(&StuckType::ErrorLoop));
    }

    #[test]
    fn test_custom_config() {
        let config = StuckDetectionConfig {
            turn_warning_threshold: 70.0,
            ..Default::default()
        };
        let detector = StuckDetector::with_config(config);

        let mut progress = AgentProgress::new(100, 100000);
        progress.turn_count = 75; // Would not trigger with default 80% threshold

        let detections = detector.check("agent-1", &progress);
        assert_eq!(detections.len(), 1);
        assert_eq!(detections[0].detection_type, StuckType::TurnLimit);
    }

    #[test]
    fn test_work_evaluation_new() {
        let eval = WorkEvaluation::new("agent-1", EvaluationType::Progress, EvaluationStatus::Healthy);
        assert_eq!(eval.agent_id, "agent-1");
        assert_eq!(eval.evaluation_type, EvaluationType::Progress);
        assert_eq!(eval.status, EvaluationStatus::Healthy);
    }

    #[test]
    fn test_work_evaluation_with_progress() {
        let eval = WorkEvaluation::new("agent-1", EvaluationType::Progress, EvaluationStatus::Healthy)
            .with_progress(50, 100)
            .with_tokens(50000, 100000)
            .with_duration(300);

        assert_eq!(eval.turn_count, Some(50));
        assert_eq!(eval.max_turns, Some(100));
        assert_eq!(eval.token_count, Some(50000));
        assert_eq!(eval.max_tokens, Some(100000));
        assert_eq!(eval.duration_secs, Some(300));
    }

    #[test]
    fn test_evaluation_type_roundtrip() {
        let types = [
            EvaluationType::Progress,
            EvaluationType::Completion,
            EvaluationType::StuckCheck,
            EvaluationType::ReviewOutcome,
            EvaluationType::CiStatus,
        ];

        for t in types {
            let s = t.as_str();
            let parsed: EvaluationType = s.parse().unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_evaluation_status_roundtrip() {
        let statuses = [
            EvaluationStatus::Healthy,
            EvaluationStatus::Warning,
            EvaluationStatus::Stuck,
            EvaluationStatus::Failed,
            EvaluationStatus::Complete,
        ];

        for s in statuses {
            let str = s.as_str();
            let parsed: EvaluationStatus = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_rate_limit_backoff_exponential() {
        let mut backoff = RateLimitBackoff::new();

        // First retry: 5 seconds
        assert_eq!(backoff.next_delay(), 5);
        // Second retry: 10 seconds
        assert_eq!(backoff.next_delay(), 10);
        // Third retry: 20 seconds
        assert_eq!(backoff.next_delay(), 20);
        // Fourth retry: 40 seconds
        assert_eq!(backoff.next_delay(), 40);
    }

    #[test]
    fn test_rate_limit_backoff_max() {
        let mut backoff = RateLimitBackoff::new();

        // Keep incrementing until we hit max
        for _ in 0..10 {
            backoff.next_delay();
        }

        // Should be capped at max_delay
        assert_eq!(backoff.next_delay(), 300);
    }

    #[test]
    fn test_rate_limit_backoff_reset() {
        let mut backoff = RateLimitBackoff::new();
        backoff.next_delay();
        backoff.next_delay();
        backoff.reset();

        // Should be back to base delay
        assert_eq!(backoff.next_delay(), 5);
    }

    #[test]
    fn test_rate_limited_until() {
        let mut backoff = RateLimitBackoff::new();
        let until = backoff.rate_limited_until();
        let now = Utc::now();

        // Should be in the future
        assert!(until > now);
        // Should be about 5 seconds from now (base delay)
        let diff = (until - now).num_seconds();
        assert!(diff >= 4 && diff <= 6);
    }
}
