#!/usr/bin/env bash
#
# install.sh - Install/Uninstall Simple Multi-Agent Orchestrator
#
# Usage:
#   ./install.sh              Install to target directory
#   ./install.sh uninstall    Remove installation
#   ./install.sh backup       Backup existing installation
#   ./install.sh restore      Restore from backup
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_INSTALL_DIR="$HOME/.local/share/orchestrate"
DEFAULT_BIN_DIR="$HOME/.local/bin"
BACKUP_DIR="$HOME/.local/share/orchestrate-backup"

INSTALL_DIR="${ORCHESTRATE_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
BIN_DIR="${ORCHESTRATE_BIN_DIR:-$DEFAULT_BIN_DIR}"

# Files to install
FILES=(
    "orchestrate"
    "README.md"
    ".gitignore"
)

AGENT_FILES=(
    ".claude/agents/bmad-orchestrator.md"
    ".claude/agents/bmad-planner.md"
    ".claude/agents/story-developer.md"
    ".claude/agents/pr-shepherd.md"
    ".claude/agents/code-reviewer.md"
    ".claude/agents/issue-fixer.md"
    ".claude/agents/explorer.md"
)

# ==================== Functions ====================

check_requirements() {
    local missing=()

    command -v git &>/dev/null || missing+=("git")
    command -v claude &>/dev/null || missing+=("claude (Claude Code CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing requirements: ${missing[*]}"
        echo ""
        echo "Install Claude Code CLI: https://docs.anthropic.com/en/docs/claude-code"
        return 1
    fi

    return 0
}

backup() {
    if [[ ! -d "$INSTALL_DIR" ]]; then
        log_warn "Nothing to backup - $INSTALL_DIR does not exist"
        return 0
    fi

    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_path="$BACKUP_DIR/$timestamp"

    mkdir -p "$backup_path"

    log_info "Backing up to: $backup_path"

    # Backup installation
    cp -r "$INSTALL_DIR"/* "$backup_path/" 2>/dev/null || true

    # Backup symlink target
    if [[ -L "$BIN_DIR/orchestrate" ]]; then
        echo "$(readlink "$BIN_DIR/orchestrate")" > "$backup_path/.symlink"
    fi

    log_info "Backup complete: $backup_path"
    echo "$backup_path"
}

restore() {
    local backup_path="${1:-}"

    if [[ -z "$backup_path" ]]; then
        # Find latest backup
        if [[ ! -d "$BACKUP_DIR" ]]; then
            log_error "No backups found in $BACKUP_DIR"
            return 1
        fi

        backup_path=$(ls -1d "$BACKUP_DIR"/*/ 2>/dev/null | tail -1)

        if [[ -z "$backup_path" ]]; then
            log_error "No backups found"
            return 1
        fi
    fi

    log_info "Restoring from: $backup_path"

    # Uninstall current
    uninstall --quiet

    # Restore files
    mkdir -p "$INSTALL_DIR"
    cp -r "$backup_path"/* "$INSTALL_DIR/" 2>/dev/null || true

    # Restore symlink
    if [[ -f "$backup_path/.symlink" ]]; then
        mkdir -p "$BIN_DIR"
        ln -sf "$INSTALL_DIR/orchestrate" "$BIN_DIR/orchestrate"
    fi

    log_info "Restore complete"
}

uninstall() {
    local quiet="${1:-}"

    [[ -z "$quiet" ]] && log_info "Uninstalling from: $INSTALL_DIR"

    # Remove symlink
    if [[ -L "$BIN_DIR/orchestrate" ]]; then
        rm "$BIN_DIR/orchestrate"
        [[ -z "$quiet" ]] && log_info "Removed: $BIN_DIR/orchestrate"
    fi

    # Remove installation directory
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
        [[ -z "$quiet" ]] && log_info "Removed: $INSTALL_DIR"
    fi

    [[ -z "$quiet" ]] && log_info "Uninstall complete"
}

install() {
    log_info "Installing to: $INSTALL_DIR"

    # Check requirements
    check_requirements || return 1

    # Backup existing if present
    if [[ -d "$INSTALL_DIR" ]]; then
        log_info "Existing installation found, backing up..."
        backup
    fi

    # Create directories
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$INSTALL_DIR/.claude/agents"
    mkdir -p "$BIN_DIR"

    # Copy files
    for file in "${FILES[@]}"; do
        if [[ -f "$SCRIPT_DIR/$file" ]]; then
            cp "$SCRIPT_DIR/$file" "$INSTALL_DIR/$file"
            log_info "Installed: $file"
        fi
    done

    # Copy agent files
    for file in "${AGENT_FILES[@]}"; do
        if [[ -f "$SCRIPT_DIR/$file" ]]; then
            cp "$SCRIPT_DIR/$file" "$INSTALL_DIR/$file"
            log_info "Installed: $file"
        fi
    done

    # Make executable
    chmod +x "$INSTALL_DIR/orchestrate"

    # Create symlink
    ln -sf "$INSTALL_DIR/orchestrate" "$BIN_DIR/orchestrate"
    log_info "Created symlink: $BIN_DIR/orchestrate"

    # Check if BIN_DIR is in PATH
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        log_warn "$BIN_DIR is not in PATH"
        echo ""
        echo "Add to your shell profile:"
        echo "  export PATH=\"\$PATH:$BIN_DIR\""
        echo ""
    fi

    log_info "Installation complete!"
    echo ""
    echo "Usage:"
    echo "  orchestrate help"
    echo "  orchestrate develop \"Add feature\""
    echo "  orchestrate bmad epic-1"
}

install_local() {
    local target="${1:-.}"

    log_info "Installing to local project: $target"

    # Create directories
    mkdir -p "$target/.claude/agents"

    # Copy agent files only
    for file in "${AGENT_FILES[@]}"; do
        if [[ -f "$SCRIPT_DIR/$file" ]]; then
            cp "$SCRIPT_DIR/$file" "$target/$file"
            log_info "Installed: $file"
        fi
    done

    # Copy orchestrate
    cp "$SCRIPT_DIR/orchestrate" "$target/orchestrate"
    chmod +x "$target/orchestrate"
    log_info "Installed: orchestrate"

    log_info "Local installation complete!"
    echo ""
    echo "Usage:"
    echo "  cd $target"
    echo "  ./orchestrate help"
}

list_backups() {
    if [[ ! -d "$BACKUP_DIR" ]]; then
        log_info "No backups found"
        return 0
    fi

    echo "Backups in $BACKUP_DIR:"
    ls -1d "$BACKUP_DIR"/*/ 2>/dev/null | while read -r dir; do
        echo "  $(basename "$dir")"
    done
}

show_help() {
    cat <<EOF
Simple Multi-Agent Orchestrator Installer

Usage: $0 <command> [args]

Commands:
  install              Install to ~/.local/share/orchestrate
  install-local [dir]  Install to local project directory
  uninstall            Remove installation
  backup               Backup current installation
  restore [backup]     Restore from backup (latest if not specified)
  list-backups         List available backups
  help                 Show this help

Environment Variables:
  ORCHESTRATE_INSTALL_DIR   Installation directory (default: ~/.local/share/orchestrate)
  ORCHESTRATE_BIN_DIR       Binary directory (default: ~/.local/bin)

Examples:
  $0 install                    # Install globally
  $0 install-local ./my-project # Install to project
  $0 backup                     # Backup current
  $0 uninstall                  # Remove
  $0 restore                    # Restore latest backup
EOF
}

# ==================== Main ====================

cmd="${1:-install}"
shift 2>/dev/null || true

case "$cmd" in
    install)
        install
        ;;
    install-local|local)
        install_local "${1:-.}"
        ;;
    uninstall|remove)
        uninstall
        ;;
    backup)
        backup
        ;;
    restore)
        restore "$@"
        ;;
    list-backups|backups)
        list_backups
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        log_error "Unknown command: $cmd"
        show_help
        exit 1
        ;;
esac
