//! Security Gate for Pipelines
//!
//! Provides security policy enforcement for CI/CD pipelines:
//! - Configurable security policy
//! - Block on critical/high vulnerabilities
//! - Allow override with justification
//! - Track security exceptions

use crate::security::{SecurityException, SecurityPolicy, SecurityScan, Severity};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Security gate decision
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GateDecision {
    /// Allow the pipeline to proceed
    Allow,
    /// Block the pipeline
    Block { reasons: Vec<String> },
    /// Allow with override
    AllowWithOverride { exception_id: String, reason: String },
}

impl GateDecision {
    pub fn is_blocked(&self) -> bool {
        matches!(self, GateDecision::Block { .. })
    }

    pub fn is_allowed(&self) -> bool {
        matches!(self, GateDecision::Allow | GateDecision::AllowWithOverride { .. })
    }
}

/// Security gate result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateResult {
    pub scan_id: String,
    pub decision: GateDecision,
    pub evaluated_at: DateTime<Utc>,
    pub policy_id: String,
    pub active_exceptions: Vec<String>,
    pub details: GateDetails,
}

impl GateResult {
    pub fn new(scan_id: impl Into<String>, policy_id: impl Into<String>, decision: GateDecision) -> Self {
        Self {
            scan_id: scan_id.into(),
            decision,
            evaluated_at: Utc::now(),
            policy_id: policy_id.into(),
            active_exceptions: Vec::new(),
            details: GateDetails::default(),
        }
    }

    pub fn with_exception(mut self, exception_id: impl Into<String>) -> Self {
        self.active_exceptions.push(exception_id.into());
        self
    }

    pub fn with_details(mut self, details: GateDetails) -> Self {
        self.details = details;
        self
    }
}

/// Details about gate evaluation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateDetails {
    pub total_vulnerabilities: usize,
    pub blocking_vulnerabilities: usize,
    pub excepted_vulnerabilities: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub secrets_count: usize,
}

/// Security gate evaluator
pub struct SecurityGate {
    policy: SecurityPolicy,
    exceptions: Vec<SecurityException>,
}

impl SecurityGate {
    /// Create a new security gate with policy
    pub fn new(policy: SecurityPolicy) -> Self {
        Self {
            policy,
            exceptions: Vec::new(),
        }
    }

    /// Add a security exception
    pub fn add_exception(&mut self, exception: SecurityException) {
        self.exceptions.push(exception);
    }

    /// Set all exceptions
    pub fn with_exceptions(mut self, exceptions: Vec<SecurityException>) -> Self {
        self.exceptions = exceptions;
        self
    }

    /// Get the policy
    pub fn policy(&self) -> &SecurityPolicy {
        &self.policy
    }

    /// Evaluate a security scan against the gate
    pub fn evaluate(&self, scan: &SecurityScan) -> GateResult {
        let mut details = GateDetails {
            total_vulnerabilities: scan.vulnerabilities.len(),
            critical_count: scan.summary.critical_count,
            high_count: scan.summary.high_count,
            medium_count: scan.summary.medium_count,
            low_count: scan.summary.low_count,
            secrets_count: scan.secrets.len(),
            blocking_vulnerabilities: 0,
            excepted_vulnerabilities: 0,
        };

        let mut blocking_reasons = Vec::new();
        let mut active_exceptions = Vec::new();

        // Check vulnerabilities
        for vuln in &scan.vulnerabilities {
            // Check if this vulnerability has an active exception
            if let Some(exception) = self.find_active_exception(&vuln.id) {
                details.excepted_vulnerabilities += 1;
                active_exceptions.push(exception.id.clone());
                continue;
            }

            // Check if this vulnerability should block
            if self.policy.should_block(&vuln.severity) {
                details.blocking_vulnerabilities += 1;
                blocking_reasons.push(format!(
                    "{} vulnerability: {} ({})",
                    vuln.severity,
                    vuln.title,
                    vuln.cve_id.as_ref().unwrap_or(&vuln.id)
                ));
            }
        }

        // Check secrets
        if self.policy.block_on_secrets && !scan.secrets.is_empty() {
            blocking_reasons.push(format!("{} secrets detected", scan.secrets.len()));
        }

        // Make decision
        let decision = if blocking_reasons.is_empty() {
            GateDecision::Allow
        } else {
            GateDecision::Block {
                reasons: blocking_reasons,
            }
        };

        let mut result = GateResult::new(&scan.id, &self.policy.id, decision).with_details(details);
        result.active_exceptions = active_exceptions;
        result
    }

    /// Find an active exception for a vulnerability
    fn find_active_exception(&self, vulnerability_id: &str) -> Option<&SecurityException> {
        self.exceptions
            .iter()
            .find(|e| e.vulnerability_id == vulnerability_id && e.is_active && !e.is_expired())
    }

    /// Request an override for a blocked scan
    pub fn request_override(
        &mut self,
        scan: &SecurityScan,
        vulnerability_id: impl Into<String>,
        reason: impl Into<String>,
        approved_by: impl Into<String>,
        duration_days: u32,
    ) -> Result<SecurityException, String> {
        // Verify the policy allows exceptions
        if !self.policy.allow_exceptions {
            return Err("Policy does not allow security exceptions".to_string());
        }

        // Verify duration is within limits
        if let Some(max_days) = self.policy.max_exception_days {
            if duration_days > max_days {
                return Err(format!(
                    "Exception duration ({} days) exceeds policy maximum ({} days)",
                    duration_days, max_days
                ));
            }
        }

        // Verify vulnerability exists in scan
        let vuln_id = vulnerability_id.into();
        let vuln = scan
            .vulnerabilities
            .iter()
            .find(|v| v.id == vuln_id)
            .ok_or_else(|| format!("Vulnerability {} not found in scan", vuln_id))?;

        // Create exception
        let exception = SecurityException::new(
            vuln_id,
            reason,
            approved_by,
            duration_days,
        );

        self.exceptions.push(exception.clone());

        Ok(exception)
    }

    /// Evaluate with automatic exception application
    pub fn evaluate_with_override(
        &mut self,
        scan: &SecurityScan,
        override_reason: Option<String>,
        approved_by: Option<String>,
    ) -> GateResult {
        let result = self.evaluate(scan);

        // If blocked and override is requested
        if result.decision.is_blocked() {
            if let (Some(reason), Some(approver)) = (override_reason, approved_by) {
                if self.policy.allow_exceptions {
                    // Create exception for the first blocking vulnerability
                    if let Some(vuln) = scan.vulnerabilities.iter().find(|v| self.policy.should_block(&v.severity)) {
                        if let Ok(exception) = self.request_override(
                            scan,
                            &vuln.id,
                            &reason,
                            approver,
                            self.policy.max_exception_days.unwrap_or(30),
                        ) {
                            return GateResult::new(&scan.id, &self.policy.id, GateDecision::AllowWithOverride {
                                exception_id: exception.id.clone(),
                                reason,
                            })
                            .with_exception(exception.id)
                            .with_details(result.details);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get all active exceptions
    pub fn active_exceptions(&self) -> Vec<&SecurityException> {
        self.exceptions
            .iter()
            .filter(|e| e.is_active && !e.is_expired())
            .collect()
    }

    /// Revoke an exception
    pub fn revoke_exception(&mut self, exception_id: &str) -> Result<(), String> {
        let exception = self
            .exceptions
            .iter_mut()
            .find(|e| e.id == exception_id)
            .ok_or_else(|| format!("Exception {} not found", exception_id))?;

        exception.is_active = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{ScanType, SecurityScan, Vulnerability};

    #[test]
    fn test_gate_decision_methods() {
        let allow = GateDecision::Allow;
        assert!(!allow.is_blocked());
        assert!(allow.is_allowed());

        let block = GateDecision::Block {
            reasons: vec!["Critical vulnerability".to_string()],
        };
        assert!(block.is_blocked());
        assert!(!block.is_allowed());

        let override_allow = GateDecision::AllowWithOverride {
            exception_id: "exc-1".to_string(),
            reason: "Business justification".to_string(),
        };
        assert!(!override_allow.is_blocked());
        assert!(override_allow.is_allowed());
    }

    #[test]
    fn test_gate_allows_safe_scan() {
        let policy = SecurityPolicy::default();
        let gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("pkg", "1.0.0", Severity::Low),
        );
        scan.complete();

        let result = gate.evaluate(&scan);
        assert_eq!(result.decision, GateDecision::Allow);
        assert_eq!(result.details.total_vulnerabilities, 1);
        assert_eq!(result.details.blocking_vulnerabilities, 0);
    }

    #[test]
    fn test_gate_blocks_critical_vulnerability() {
        let policy = SecurityPolicy::default();
        let gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical)
                .with_cve("CVE-2024-0001"),
        );
        scan.complete();

        let result = gate.evaluate(&scan);
        assert!(result.decision.is_blocked());
        assert_eq!(result.details.blocking_vulnerabilities, 1);

        if let GateDecision::Block { reasons } = result.decision {
            assert_eq!(reasons.len(), 1);
            assert!(reasons[0].contains("CRITICAL"));
        } else {
            panic!("Expected Block decision");
        }
    }

    #[test]
    fn test_gate_blocks_high_vulnerability() {
        let policy = SecurityPolicy::default();
        let gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::High),
        );
        scan.complete();

        let result = gate.evaluate(&scan);
        assert!(result.decision.is_blocked());
    }

    #[test]
    fn test_gate_blocks_secrets() {
        let mut policy = SecurityPolicy::default();
        policy.block_on_secrets = true;
        let gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Secrets], "test");
        scan.add_secret(crate::security::DetectedSecret::new(
            crate::security::SecretType::AwsAccessKey,
            ".env",
            10,
            "AKIA***",
        ));
        scan.complete();

        let result = gate.evaluate(&scan);
        assert!(result.decision.is_blocked());

        if let GateDecision::Block { reasons } = result.decision {
            assert!(reasons[0].contains("secrets"));
        }
    }

    #[test]
    fn test_gate_with_active_exception() {
        let policy = SecurityPolicy::default();
        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        let vuln = Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical)
            .with_cve("CVE-2024-0001");
        let vuln_id = vuln.id.clone();
        scan.add_vulnerability(vuln);
        scan.complete();

        let exception = SecurityException::new(
            &vuln_id,
            "False positive",
            "security-team",
            30,
        );

        let gate = SecurityGate::new(policy).with_exceptions(vec![exception.clone()]);

        let result = gate.evaluate(&scan);
        assert_eq!(result.decision, GateDecision::Allow);
        assert_eq!(result.details.excepted_vulnerabilities, 1);
        assert_eq!(result.active_exceptions, vec![exception.id]);
    }

    #[test]
    fn test_request_override_success() {
        let mut policy = SecurityPolicy::default();
        policy.allow_exceptions = true;
        policy.max_exception_days = Some(30);

        let mut gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        let vuln = Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical);
        let vuln_id = vuln.id.clone();
        scan.add_vulnerability(vuln);
        scan.complete();

        let result = gate.request_override(
            &scan,
            &vuln_id,
            "Business critical dependency, fix in progress",
            "manager@example.com",
            7,
        );

        assert!(result.is_ok());
        let exception = result.unwrap();
        assert_eq!(exception.vulnerability_id, vuln_id);
        assert_eq!(exception.approved_by, "manager@example.com");
    }

    #[test]
    fn test_request_override_policy_disallows() {
        let mut policy = SecurityPolicy::default();
        policy.allow_exceptions = false;

        let mut gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        let vuln = Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical);
        let vuln_id = vuln.id.clone();
        scan.add_vulnerability(vuln);
        scan.complete();

        let result = gate.request_override(
            &scan,
            &vuln_id,
            "Override request",
            "manager@example.com",
            7,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not allow"));
    }

    #[test]
    fn test_request_override_exceeds_duration() {
        let mut policy = SecurityPolicy::default();
        policy.allow_exceptions = true;
        policy.max_exception_days = Some(7);

        let mut gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        let vuln = Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical);
        let vuln_id = vuln.id.clone();
        scan.add_vulnerability(vuln);
        scan.complete();

        let result = gate.request_override(
            &scan,
            &vuln_id,
            "Override request",
            "manager@example.com",
            30, // Exceeds max of 7
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds"));
    }

    #[test]
    fn test_evaluate_with_override() {
        let mut policy = SecurityPolicy::default();
        policy.allow_exceptions = true;

        let mut gate = SecurityGate::new(policy);

        let mut scan = SecurityScan::new(vec![ScanType::Full], "test");
        scan.add_vulnerability(
            Vulnerability::dependency("vuln-pkg", "1.0.0", Severity::Critical),
        );
        scan.complete();

        let result = gate.evaluate_with_override(
            &scan,
            Some("Business justification".to_string()),
            Some("manager@example.com".to_string()),
        );

        assert!(result.decision.is_allowed());
        if let GateDecision::AllowWithOverride { reason, .. } = result.decision {
            assert_eq!(reason, "Business justification");
        } else {
            panic!("Expected AllowWithOverride decision");
        }
    }

    #[test]
    fn test_revoke_exception() {
        let policy = SecurityPolicy::default();
        let exception = SecurityException::new(
            "vuln-1",
            "Test exception",
            "test",
            30,
        );
        let exception_id = exception.id.clone();

        let mut gate = SecurityGate::new(policy).with_exceptions(vec![exception]);

        assert_eq!(gate.active_exceptions().len(), 1);

        gate.revoke_exception(&exception_id).unwrap();

        assert_eq!(gate.active_exceptions().len(), 0);
    }

    #[test]
    fn test_expired_exception_not_active() {
        let policy = SecurityPolicy::default();
        let mut exception = SecurityException::new(
            "vuln-1",
            "Test exception",
            "test",
            1,
        );
        // Force expiration by setting expires_at in the past
        exception.expires_at = Utc::now() - chrono::Duration::days(1);

        let gate = SecurityGate::new(policy).with_exceptions(vec![exception]);

        assert_eq!(gate.active_exceptions().len(), 0);
    }
}
