#!/usr/bin/env bash
#
# Version bump script for orchestrate
#
# Usage:
#   ./scripts/bump-version.sh [major|minor|patch] [--dry-run]
#
# Examples:
#   ./scripts/bump-version.sh patch          # 0.1.0 -> 0.1.1
#   ./scripts/bump-version.sh minor          # 0.1.0 -> 0.2.0
#   ./scripts/bump-version.sh major          # 0.1.0 -> 1.0.0
#   ./scripts/bump-version.sh patch --dry-run # Show what would happen

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CARGO_TOML="$ROOT_DIR/Cargo.toml"
CHANGELOG="$ROOT_DIR/CHANGELOG.md"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse arguments
BUMP_TYPE="${1:-}"
DRY_RUN=false

for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
    esac
done

# Validate bump type
if [[ ! "$BUMP_TYPE" =~ ^(major|minor|patch)$ ]]; then
    echo -e "${RED}Error: Invalid bump type '$BUMP_TYPE'${NC}"
    echo "Usage: $0 [major|minor|patch] [--dry-run]"
    exit 1
fi

# Get current version from workspace Cargo.toml
get_current_version() {
    grep -E '^version = "' "$CARGO_TOML" | head -1 | sed 's/version = "\([^"]*\)"/\1/'
}

# Parse semver components
parse_version() {
    local version="$1"
    echo "$version" | sed 's/\./ /g'
}

# Calculate new version
bump_version() {
    local current="$1"
    local type="$2"

    read -r major minor patch <<< "$(parse_version "$current")"

    case "$type" in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
    esac

    echo "${major}.${minor}.${patch}"
}

# Get commits since last tag or initial commit
get_commits_since_last_tag() {
    local last_tag
    last_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")

    if [[ -n "$last_tag" ]]; then
        git log "${last_tag}..HEAD" --pretty=format:"%h %s" --no-merges
    else
        git log --pretty=format:"%h %s" --no-merges
    fi
}

# Categorize commits by conventional commit type
categorize_commits() {
    local commits="$1"

    local features=""
    local fixes=""
    local docs=""
    local refactor=""
    local other=""

    while IFS= read -r line; do
        [[ -z "$line" ]] && continue

        local hash="${line%% *}"
        local message="${line#* }"

        if [[ "$message" =~ ^feat(\(.+\))?:\ (.+) ]]; then
            features+="- ${BASH_REMATCH[2]} (\`$hash\`)"$'\n'
        elif [[ "$message" =~ ^fix(\(.+\))?:\ (.+) ]]; then
            fixes+="- ${BASH_REMATCH[2]} (\`$hash\`)"$'\n'
        elif [[ "$message" =~ ^docs(\(.+\))?:\ (.+) ]]; then
            docs+="- ${BASH_REMATCH[2]} (\`$hash\`)"$'\n'
        elif [[ "$message" =~ ^refactor(\(.+\))?:\ (.+) ]]; then
            refactor+="- ${BASH_REMATCH[2]} (\`$hash\`)"$'\n'
        else
            other+="- $message (\`$hash\`)"$'\n'
        fi
    done <<< "$commits"

    # Output categorized sections
    local output=""

    if [[ -n "$features" ]]; then
        output+="### Features"$'\n\n'"$features"$'\n'
    fi
    if [[ -n "$fixes" ]]; then
        output+="### Bug Fixes"$'\n\n'"$fixes"$'\n'
    fi
    if [[ -n "$docs" ]]; then
        output+="### Documentation"$'\n\n'"$docs"$'\n'
    fi
    if [[ -n "$refactor" ]]; then
        output+="### Refactoring"$'\n\n'"$refactor"$'\n'
    fi
    if [[ -n "$other" ]]; then
        output+="### Other Changes"$'\n\n'"$other"$'\n'
    fi

    echo "$output"
}

# Generate changelog entry
generate_changelog_entry() {
    local version="$1"
    local date
    date=$(date +%Y-%m-%d)

    local commits
    commits=$(get_commits_since_last_tag)

    if [[ -z "$commits" ]]; then
        echo -e "${YELLOW}Warning: No commits found since last tag${NC}"
        return 1
    fi

    local categorized
    categorized=$(categorize_commits "$commits")

    echo "## [$version] - $date"
    echo ""
    echo "$categorized"
}

# Update version in Cargo.toml files
update_cargo_versions() {
    local old_version="$1"
    local new_version="$2"

    # Update workspace version
    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' "s/^version = \"$old_version\"/version = \"$new_version\"/" "$CARGO_TOML"
    else
        sed -i "s/^version = \"$old_version\"/version = \"$new_version\"/" "$CARGO_TOML"
    fi

    echo -e "${GREEN}Updated $CARGO_TOML${NC}"
}

# Update changelog file
update_changelog() {
    local entry="$1"

    if [[ -f "$CHANGELOG" ]]; then
        # Insert new entry after the header
        local header
        header=$(head -n 2 "$CHANGELOG")
        local rest
        rest=$(tail -n +3 "$CHANGELOG")

        {
            echo "$header"
            echo ""
            echo "$entry"
            echo "$rest"
        } > "$CHANGELOG.tmp"
        mv "$CHANGELOG.tmp" "$CHANGELOG"
    else
        # Create new changelog
        {
            echo "# Changelog"
            echo ""
            echo "All notable changes to this project will be documented in this file."
            echo ""
            echo "The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),"
            echo "and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)."
            echo ""
            echo "$entry"
        } > "$CHANGELOG"
    fi

    echo -e "${GREEN}Updated $CHANGELOG${NC}"
}

# Main execution
main() {
    cd "$ROOT_DIR"

    # Ensure we're in a git repository
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        echo -e "${RED}Error: Not a git repository${NC}"
        exit 1
    fi

    # Check for uncommitted changes
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
        echo -e "${YELLOW}Warning: You have uncommitted changes${NC}"
    fi

    # Get versions
    local current_version
    current_version=$(get_current_version)
    local new_version
    new_version=$(bump_version "$current_version" "$BUMP_TYPE")

    echo -e "${BLUE}Current version:${NC} $current_version"
    echo -e "${BLUE}New version:${NC}     $new_version"
    echo -e "${BLUE}Bump type:${NC}       $BUMP_TYPE"
    echo ""

    # Generate changelog entry
    local changelog_entry
    changelog_entry=$(generate_changelog_entry "$new_version")

    if [[ -z "$changelog_entry" ]]; then
        echo -e "${RED}Error: Could not generate changelog entry${NC}"
        exit 1
    fi

    echo -e "${BLUE}Changelog entry:${NC}"
    echo "----------------------------------------"
    echo "$changelog_entry"
    echo "----------------------------------------"
    echo ""

    if [[ "$DRY_RUN" == true ]]; then
        echo -e "${YELLOW}Dry run - no changes made${NC}"
        exit 0
    fi

    # Confirm with user
    read -p "Proceed with version bump? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo -e "${YELLOW}Aborted${NC}"
        exit 0
    fi

    # Perform updates
    echo ""
    echo -e "${BLUE}Updating files...${NC}"
    update_cargo_versions "$current_version" "$new_version"
    update_changelog "$changelog_entry"

    # Create git commit and tag
    echo ""
    echo -e "${BLUE}Creating git commit and tag...${NC}"
    git add "$CARGO_TOML" "$CHANGELOG"
    git commit -m "chore: bump version to $new_version"
    git tag -a "v$new_version" -m "Release v$new_version"

    echo ""
    echo -e "${GREEN}Version bumped to $new_version${NC}"
    echo ""
    echo "Next steps:"
    echo "  git push origin $(git branch --show-current)"
    echo "  git push origin v$new_version"
}

main "$@"
