//! Release Management Service
//!
//! This module provides release management capabilities:
//! - Semantic version bumping
//! - Changelog generation from commits
//! - Release branch creation
//! - GitHub release creation with assets
//! - Release tagging

use crate::{Database, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Version bump type for semantic versioning
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BumpType {
    Major,
    Minor,
    Patch,
}

impl std::fmt::Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Major => write!(f, "major"),
            Self::Minor => write!(f, "minor"),
            Self::Patch => write!(f, "patch"),
        }
    }
}

impl std::str::FromStr for BumpType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "patch" => Ok(Self::Patch),
            _ => Err(Error::Other(format!("Invalid bump type: {}", s))),
        }
    }
}

/// Semantic version
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build: Option<String>,
}

impl Version {
    /// Parse version from string (e.g., "1.2.3", "1.2.3-beta.1", "1.2.3+build.123")
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        let (version_part, build) = match parts.as_slice() {
            [v] => (*v, None),
            [v, b] => (*v, Some((*b).to_string())),
            _ => return Err(Error::Other(format!("Invalid version format: {}", s))),
        };

        let parts: Vec<&str> = version_part.split('-').collect();
        let (core_version, pre_release) = match parts.as_slice() {
            [v] => (*v, None),
            [v, p] => (*v, Some((*p).to_string())),
            _ => return Err(Error::Other(format!("Invalid version format: {}", s))),
        };

        let parts: Vec<&str> = core_version.split('.').collect();
        if parts.len() != 3 {
            return Err(Error::Other(format!("Invalid version format: {}", s)));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| Error::Other(format!("Invalid major version: {}", parts[0])))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| Error::Other(format!("Invalid minor version: {}", parts[1])))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| Error::Other(format!("Invalid patch version: {}", parts[2])))?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build,
        })
    }

    /// Bump version according to type
    pub fn bump(&self, bump_type: &BumpType) -> Self {
        match bump_type {
            BumpType::Major => Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
                pre_release: None,
                build: None,
            },
            BumpType::Minor => Self {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
                pre_release: None,
                build: None,
            },
            BumpType::Patch => Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
                pre_release: None,
                build: None,
            },
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(build) = &self.build {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

/// Commit type for changelog categorization
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitType {
    Feature,
    Fix,
    Change,
    Breaking,
    Docs,
    Chore,
    Other,
}

impl CommitType {
    /// Parse commit type from conventional commit message
    pub fn from_message(message: &str) -> Self {
        let lower = message.to_lowercase();

        // Check for breaking changes first (before checking other types)
        if lower.contains("breaking change") || lower.contains("!:") {
            return Self::Breaking;
        }

        if lower.starts_with("feat") || lower.starts_with("feature") {
            Self::Feature
        } else if lower.starts_with("fix") {
            Self::Fix
        } else if lower.starts_with("refactor") || lower.starts_with("perf") {
            Self::Change
        } else if lower.starts_with("docs") {
            Self::Docs
        } else if lower.starts_with("chore") || lower.starts_with("build") || lower.starts_with("ci") {
            Self::Chore
        } else {
            Self::Other
        }
    }

    /// Get changelog section name
    pub fn section_name(&self) -> &str {
        match self {
            Self::Feature => "Added",
            Self::Fix => "Fixed",
            Self::Change => "Changed",
            Self::Breaking => "Breaking Changes",
            Self::Docs => "Documentation",
            Self::Chore => "Maintenance",
            Self::Other => "Other",
        }
    }
}

/// Git commit information
#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: DateTime<Utc>,
}

/// Changelog entry
#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub commit_type: CommitType,
    pub description: String,
    pub pr_number: Option<u32>,
}

/// Changelog for a release
#[derive(Debug, Clone)]
pub struct Changelog {
    pub version: Version,
    pub date: DateTime<Utc>,
    pub entries: Vec<ChangelogEntry>,
}

impl Changelog {
    /// Generate markdown for changelog
    pub fn to_markdown(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("## [{}] - {}\n\n", self.version, self.date.format("%Y-%m-%d")));

        // Group entries by type
        let mut by_type: HashMap<&str, Vec<&ChangelogEntry>> = HashMap::new();
        for entry in &self.entries {
            by_type
                .entry(entry.commit_type.section_name())
                .or_default()
                .push(entry);
        }

        // Order sections
        let section_order = [
            "Breaking Changes",
            "Added",
            "Changed",
            "Fixed",
            "Documentation",
            "Maintenance",
            "Other",
        ];

        for section in section_order {
            if let Some(entries) = by_type.get(section) {
                if !entries.is_empty() {
                    output.push_str(&format!("### {}\n", section));
                    for entry in entries {
                        output.push_str("- ");
                        output.push_str(&entry.description);
                        if let Some(pr) = entry.pr_number {
                            output.push_str(&format!(" (#{pr})"));
                        }
                        output.push('\n');
                    }
                    output.push('\n');
                }
            }
        }

        output
    }
}

/// Release preparation result
#[derive(Debug, Clone)]
pub struct ReleasePreparation {
    pub new_version: Version,
    pub branch_name: String,
    pub changelog: Changelog,
}

/// GitHub release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub path: String,
    pub content_type: String,
}

/// GitHub release request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRequest {
    pub version: Version,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<ReleaseAsset>,
}

/// Release management service
pub struct ReleaseManager {
    db: Database,
}

impl ReleaseManager {
    /// Create new release manager
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Get current version from Cargo.toml
    pub async fn get_current_version(&self, cargo_toml_path: &Path) -> Result<Version> {
        let content = tokio::fs::read_to_string(cargo_toml_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read Cargo.toml: {}", e)))?;

        let toml: toml::Value = toml::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse Cargo.toml: {}", e)))?;

        let version_str = toml
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("version"))
            .or_else(|| toml.get("package").and_then(|p| p.get("version")))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("Version not found in Cargo.toml".to_string()))?;

        Version::parse(version_str)
    }

    /// Bump version in Cargo.toml
    pub async fn bump_version(&self, cargo_toml_path: &Path, new_version: &Version) -> Result<()> {
        let content = tokio::fs::read_to_string(cargo_toml_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read Cargo.toml: {}", e)))?;

        let mut toml: toml::Value = toml::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse Cargo.toml: {}", e)))?;

        // Update version in workspace.package.version or package.version
        if let Some(workspace) = toml.get_mut("workspace") {
            if let Some(package) = workspace.get_mut("package") {
                if let Some(version) = package.get_mut("version") {
                    *version = toml::Value::String(new_version.to_string());
                }
            }
        } else if let Some(package) = toml.get_mut("package") {
            if let Some(version) = package.get_mut("version") {
                *version = toml::Value::String(new_version.to_string());
            }
        }

        let new_content = toml::to_string(&toml)
            .map_err(|e| Error::Other(format!("Failed to serialize Cargo.toml: {}", e)))?;

        tokio::fs::write(cargo_toml_path, new_content)
            .await
            .map_err(|e| Error::Other(format!("Failed to write Cargo.toml: {}", e)))?;

        Ok(())
    }

    /// Bump version in package.json
    pub async fn bump_package_json_version(&self, package_json_path: &Path, new_version: &Version) -> Result<()> {
        let content = tokio::fs::read_to_string(package_json_path)
            .await
            .map_err(|e| Error::Other(format!("Failed to read package.json: {}", e)))?;

        let mut json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| Error::Other(format!("Failed to parse package.json: {}", e)))?;

        if let Some(obj) = json.as_object_mut() {
            obj.insert("version".to_string(), serde_json::Value::String(new_version.to_string()));
        }

        let new_content = serde_json::to_string_pretty(&json)
            .map_err(|e| Error::Other(format!("Failed to serialize package.json: {}", e)))?;

        tokio::fs::write(package_json_path, new_content)
            .await
            .map_err(|e| Error::Other(format!("Failed to write package.json: {}", e)))?;

        Ok(())
    }

    /// Parse git log to extract commits
    pub async fn get_commits_since(&self, since_ref: &str) -> Result<Vec<Commit>> {
        let output = tokio::process::Command::new("git")
            .args(["log", &format!("{}..HEAD", since_ref), "--format=%H|||%s|||%an|||%aI"])
            .output()
            .await
            .map_err(|e| Error::Other(format!("Failed to run git log: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Other(format!(
                "git log failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut commits = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split("|||").collect();
            if parts.len() != 4 {
                continue;
            }

            commits.push(Commit {
                hash: parts[0].to_string(),
                message: parts[1].to_string(),
                author: parts[2].to_string(),
                date: DateTime::parse_from_rfc3339(parts[3])
                    .map_err(|e| Error::Other(format!("Failed to parse date: {}", e)))?
                    .with_timezone(&Utc),
            });
        }

        Ok(commits)
    }

    /// Generate changelog from commits
    pub fn generate_changelog(&self, commits: &[Commit], version: &Version) -> Changelog {
        let mut entries = Vec::new();

        for commit in commits {
            let commit_type = CommitType::from_message(&commit.message);

            // Skip chore commits in changelog
            if matches!(commit_type, CommitType::Chore) {
                continue;
            }

            // Extract PR number from commit message (e.g., "#123" or "(#123)")
            let pr_number = self.extract_pr_number(&commit.message);

            // Clean up commit message for changelog
            let description = self.clean_commit_message(&commit.message);

            entries.push(ChangelogEntry {
                commit_type,
                description,
                pr_number,
            });
        }

        Changelog {
            version: version.clone(),
            date: Utc::now(),
            entries,
        }
    }

    /// Extract PR number from commit message
    fn extract_pr_number(&self, message: &str) -> Option<u32> {
        let re = regex::Regex::new(r"#(\d+)").ok()?;
        re.captures(message)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Clean commit message for changelog
    fn clean_commit_message(&self, message: &str) -> String {
        // Remove conventional commit prefix (feat:, fix:, etc.)
        let cleaned = regex::Regex::new(r"^(feat|fix|docs|chore|refactor|perf|test|build|ci|style)(\(.+?\))?!?:\s*")
            .ok()
            .and_then(|re| Some(re.replace(message, "").to_string()))
            .unwrap_or_else(|| message.to_string());

        // Remove PR number from message (we'll add it separately)
        let cleaned = regex::Regex::new(r"\s*\(#\d+\)|\s*#\d+")
            .ok()
            .and_then(|re| Some(re.replace_all(&cleaned, "").to_string()))
            .unwrap_or(cleaned);

        // Capitalize first letter
        let mut chars = cleaned.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }

    /// Prepare release: bump version, create branch, generate changelog
    pub async fn prepare_release(&self, bump_type: BumpType, cargo_toml_path: &Path) -> Result<ReleasePreparation> {
        // Get current version
        let current_version = self.get_current_version(cargo_toml_path).await?;

        // Calculate new version
        let new_version = current_version.bump(&bump_type);

        // Get commits since last tag
        let last_tag = format!("v{}", current_version);
        let commits = self.get_commits_since(&last_tag).await?;

        // Generate changelog
        let changelog = self.generate_changelog(&commits, &new_version);

        // Create release branch name
        let branch_name = format!("release/v{}", new_version);

        Ok(ReleasePreparation {
            new_version,
            branch_name,
            changelog,
        })
    }

    /// Create release branch
    pub async fn create_release_branch(&self, branch_name: &str) -> Result<()> {
        let output = tokio::process::Command::new("git")
            .args(["checkout", "-b", branch_name])
            .output()
            .await
            .map_err(|e| Error::Other(format!("Failed to create branch: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Other(format!(
                "git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Update CHANGELOG.md file
    pub async fn update_changelog_file(&self, changelog_path: &Path, changelog: &Changelog) -> Result<()> {
        let new_entry = changelog.to_markdown();

        // Read existing changelog if it exists
        let existing = if changelog_path.exists() {
            tokio::fs::read_to_string(changelog_path)
                .await
                .unwrap_or_default()
        } else {
            String::from("# Changelog\n\nAll notable changes to this project will be documented in this file.\n\n")
        };

        // Insert new entry after the header
        let updated = if let Some(pos) = existing.find("\n\n") {
            let (header, rest) = existing.split_at(pos + 2);
            format!("{}{}{}", header, new_entry, rest)
        } else {
            format!("{}\n\n{}", existing, new_entry)
        };

        tokio::fs::write(changelog_path, updated)
            .await
            .map_err(|e| Error::Other(format!("Failed to write CHANGELOG.md: {}", e)))?;

        Ok(())
    }

    /// Create git tag for release
    pub async fn create_release_tag(&self, version: &Version, message: &str) -> Result<()> {
        let tag_name = format!("v{}", version);

        let output = tokio::process::Command::new("git")
            .args(["tag", "-a", &tag_name, "-m", message])
            .output()
            .await
            .map_err(|e| Error::Other(format!("Failed to create tag: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Other(format!(
                "git tag failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }

    /// Push tag to remote
    pub async fn push_tag(&self, version: &Version) -> Result<()> {
        let tag_name = format!("v{}", version);

        let output = tokio::process::Command::new("git")
            .args(["push", "origin", &tag_name])
            .output()
            .await
            .map_err(|e| Error::Other(format!("Failed to push tag: {}", e)))?;

        if !output.status.success() {
            return Err(Error::Other(format!(
                "git push failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build, None);
    }

    #[test]
    fn test_version_parse_with_prerelease() {
        let v = Version::parse("1.2.3-beta.1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, Some("beta.1".to_string()));
        assert_eq!(v.build, None);
    }

    #[test]
    fn test_version_parse_with_build() {
        let v = Version::parse("1.2.3+build.123").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, None);
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_parse_with_prerelease_and_build() {
        let v = Version::parse("1.2.3-beta.1+build.123").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert_eq!(v.pre_release, Some("beta.1".to_string()));
        assert_eq!(v.build, Some("build.123".to_string()));
    }

    #[test]
    fn test_version_parse_invalid() {
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.2.3.4").is_err());
        assert!(Version::parse("a.b.c").is_err());
    }

    #[test]
    fn test_version_to_string() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: None,
            build: None,
        };
        assert_eq!(v.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_to_string_with_prerelease() {
        let v = Version {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: Some("beta.1".to_string()),
            build: None,
        };
        assert_eq!(v.to_string(), "1.2.3-beta.1");
    }

    #[test]
    fn test_version_bump_major() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(&BumpType::Major);
        assert_eq!(bumped.to_string(), "2.0.0");
    }

    #[test]
    fn test_version_bump_minor() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(&BumpType::Minor);
        assert_eq!(bumped.to_string(), "1.3.0");
    }

    #[test]
    fn test_version_bump_patch() {
        let v = Version::parse("1.2.3").unwrap();
        let bumped = v.bump(&BumpType::Patch);
        assert_eq!(bumped.to_string(), "1.2.4");
    }

    #[test]
    fn test_version_bump_clears_prerelease() {
        let v = Version::parse("1.2.3-beta.1").unwrap();
        let bumped = v.bump(&BumpType::Patch);
        assert_eq!(bumped.to_string(), "1.2.4");
        assert_eq!(bumped.pre_release, None);
    }

    #[test]
    fn test_bump_type_from_str() {
        assert_eq!("major".parse::<BumpType>().unwrap(), BumpType::Major);
        assert_eq!("minor".parse::<BumpType>().unwrap(), BumpType::Minor);
        assert_eq!("patch".parse::<BumpType>().unwrap(), BumpType::Patch);
        assert_eq!("MAJOR".parse::<BumpType>().unwrap(), BumpType::Major);
        assert!("invalid".parse::<BumpType>().is_err());
    }

    #[test]
    fn test_commit_type_from_message() {
        assert_eq!(CommitType::from_message("feat: add new feature"), CommitType::Feature);
        assert_eq!(CommitType::from_message("feature: add new feature"), CommitType::Feature);
        assert_eq!(CommitType::from_message("fix: fix bug"), CommitType::Fix);
        assert_eq!(CommitType::from_message("refactor: improve code"), CommitType::Change);
        assert_eq!(CommitType::from_message("perf: optimize performance"), CommitType::Change);
        assert_eq!(CommitType::from_message("feat!: breaking change"), CommitType::Breaking);
        assert_eq!(CommitType::from_message("docs: update readme"), CommitType::Docs);
        assert_eq!(CommitType::from_message("chore: update deps"), CommitType::Chore);
        assert_eq!(CommitType::from_message("random message"), CommitType::Other);
    }

    #[test]
    fn test_commit_type_section_name() {
        assert_eq!(CommitType::Feature.section_name(), "Added");
        assert_eq!(CommitType::Fix.section_name(), "Fixed");
        assert_eq!(CommitType::Change.section_name(), "Changed");
        assert_eq!(CommitType::Breaking.section_name(), "Breaking Changes");
    }

    #[tokio::test]
    async fn test_release_manager_extract_pr_number() {
        let db = Database::in_memory().await.unwrap();
        let manager = ReleaseManager::new(db);

        assert_eq!(manager.extract_pr_number("feat: add feature (#123)"), Some(123));
        assert_eq!(manager.extract_pr_number("fix: fix bug #456"), Some(456));
        assert_eq!(manager.extract_pr_number("feat: no PR number"), None);
    }

    #[tokio::test]
    async fn test_release_manager_clean_commit_message() {
        let db = Database::in_memory().await.unwrap();
        let manager = ReleaseManager::new(db);

        assert_eq!(
            manager.clean_commit_message("feat: add new feature"),
            "Add new feature"
        );
        assert_eq!(
            manager.clean_commit_message("feat(scope): add new feature"),
            "Add new feature"
        );
        assert_eq!(
            manager.clean_commit_message("fix!: breaking fix"),
            "Breaking fix"
        );
        assert_eq!(
            manager.clean_commit_message("random message"),
            "Random message"
        );
    }

    #[tokio::test]
    async fn test_changelog_to_markdown() {
        let changelog = Changelog {
            version: Version::parse("1.2.0").unwrap(),
            date: DateTime::parse_from_rfc3339("2024-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            entries: vec![
                ChangelogEntry {
                    commit_type: CommitType::Feature,
                    description: "GitHub webhook triggers (Epic 002)".to_string(),
                    pr_number: None,
                },
                ChangelogEntry {
                    commit_type: CommitType::Feature,
                    description: "Scheduled agent execution (Epic 003)".to_string(),
                    pr_number: None,
                },
                ChangelogEntry {
                    commit_type: CommitType::Fix,
                    description: "Agent timeout handling".to_string(),
                    pr_number: Some(123),
                },
                ChangelogEntry {
                    commit_type: CommitType::Change,
                    description: "Improved PR shepherd performance".to_string(),
                    pr_number: None,
                },
            ],
        };

        let markdown = changelog.to_markdown();

        assert!(markdown.contains("## [1.2.0] - 2024-01-15"));
        assert!(markdown.contains("### Added"));
        assert!(markdown.contains("- GitHub webhook triggers (Epic 002)"));
        assert!(markdown.contains("- Scheduled agent execution (Epic 003)"));
        assert!(markdown.contains("### Fixed"));
        assert!(markdown.contains("- Agent timeout handling (#123)"));
        assert!(markdown.contains("### Changed"));
        assert!(markdown.contains("- Improved PR shepherd performance"));
    }

    #[tokio::test]
    async fn test_generate_changelog() {
        let db = Database::in_memory().await.unwrap();
        let manager = ReleaseManager::new(db);

        let commits = vec![
            Commit {
                hash: "abc123".to_string(),
                message: "feat: add webhook triggers (#100)".to_string(),
                author: "John Doe".to_string(),
                date: Utc::now(),
            },
            Commit {
                hash: "def456".to_string(),
                message: "fix: resolve timeout issue (#101)".to_string(),
                author: "Jane Smith".to_string(),
                date: Utc::now(),
            },
            Commit {
                hash: "ghi789".to_string(),
                message: "chore: update dependencies".to_string(),
                author: "Bot".to_string(),
                date: Utc::now(),
            },
        ];

        let version = Version::parse("1.2.0").unwrap();
        let changelog = manager.generate_changelog(&commits, &version);

        // Chore commits should be excluded
        assert_eq!(changelog.entries.len(), 2);

        assert_eq!(changelog.entries[0].commit_type, CommitType::Feature);
        assert_eq!(changelog.entries[0].description, "Add webhook triggers");
        assert_eq!(changelog.entries[0].pr_number, Some(100));

        assert_eq!(changelog.entries[1].commit_type, CommitType::Fix);
        assert_eq!(changelog.entries[1].description, "Resolve timeout issue");
        assert_eq!(changelog.entries[1].pr_number, Some(101));
    }
}
