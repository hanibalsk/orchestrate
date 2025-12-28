#!/usr/bin/env bash
#
# install.sh - Install/Uninstall Orchestrate
#
# Usage:
#   ./install.sh              Install to target directory
#   ./install.sh uninstall    Remove installation
#   ./install.sh backup       Backup existing installation
#   ./install.sh restore      Restore from backup
#   ./install.sh list         List files from manifest
#
# The MANIFEST file defines what gets installed.

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_debug() { [[ "${VERBOSE:-}" == "true" ]] && echo -e "${BLUE}[DEBUG]${NC} $*" || true; }

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_FILE="$SCRIPT_DIR/MANIFEST"
DEFAULT_INSTALL_DIR="$HOME/.local/share/orchestrate"
DEFAULT_BIN_DIR="$HOME/.local/bin"
BACKUP_DIR="$HOME/.local/share/orchestrate-backup"

INSTALL_DIR="${ORCHESTRATE_INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
BIN_DIR="${ORCHESTRATE_BIN_DIR:-$DEFAULT_BIN_DIR}"

# ==================== Manifest Parsing ====================

# Parse manifest and return entries of specified type(s)
# Usage: parse_manifest [type1,type2,...]
# Returns: "type path mode" per line
parse_manifest() {
    local filter="${1:-}"

    if [[ ! -f "$MANIFEST_FILE" ]]; then
        log_error "Manifest file not found: $MANIFEST_FILE"
        return 1
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        # Parse: TYPE PATH [MODE]
        read -r type path mode <<< "$line"

        # Skip if type doesn't match filter
        if [[ -n "$filter" ]] && [[ ! ",$filter," == *",$type,"* ]]; then
            continue
        fi

        # Default modes
        if [[ -z "$mode" ]]; then
            case "$type" in
                bin|script) mode="755" ;;
                *) mode="644" ;;
            esac
        fi

        echo "$type $path $mode"
    done < "$MANIFEST_FILE"
}

# Get all files from manifest
get_manifest_files() {
    parse_manifest "${1:-}" | while read -r type path mode; do
        echo "$path"
    done
}

# ==================== Functions ====================

check_requirements() {
    local missing=()

    command -v git &>/dev/null || missing+=("git")
    command -v claude &>/dev/null || missing+=("claude (Claude Code CLI)")

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_warn "Optional requirements not found: ${missing[*]}"
        echo ""
        echo "Install Claude Code CLI: https://docs.anthropic.com/en/docs/claude-code"
        echo ""
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

    # Save manifest snapshot
    if [[ -f "$MANIFEST_FILE" ]]; then
        cp "$MANIFEST_FILE" "$backup_path/.manifest"
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

    # Remove symlinks for bin types
    parse_manifest "bin,script" | while read -r type path mode; do
        local name=$(basename "$path")
        if [[ -L "$BIN_DIR/$name" ]]; then
            rm "$BIN_DIR/$name"
            [[ -z "$quiet" ]] && log_info "Removed symlink: $BIN_DIR/$name"
        fi
    done

    # Remove installation directory
    if [[ -d "$INSTALL_DIR" ]]; then
        rm -rf "$INSTALL_DIR"
        [[ -z "$quiet" ]] && log_info "Removed: $INSTALL_DIR"
    fi

    [[ -z "$quiet" ]] && log_info "Uninstall complete"
}

# Parse manifest from installed location
parse_installed_manifest() {
    local manifest_file="$1"
    local filter="${2:-}"

    if [[ ! -f "$manifest_file" ]]; then
        return 1
    fi

    while IFS= read -r line || [[ -n "$line" ]]; do
        # Skip comments and empty lines
        [[ "$line" =~ ^[[:space:]]*# ]] && continue
        [[ -z "${line// }" ]] && continue

        # Parse: TYPE PATH [MODE]
        read -r type path mode <<< "$line"

        # Skip if type doesn't match filter
        if [[ -n "$filter" ]] && [[ ! ",$filter," == *",$type,"* ]]; then
            continue
        fi

        echo "$type $path"
    done < "$manifest_file"
}

uninstall_local() {
    local target="${1:-.}"
    local install_dir="$target/.orchestrate"
    local claude_dir="$target/.claude"
    local manifest_file="$install_dir/MANIFEST"

    if [[ ! -f "$manifest_file" ]]; then
        log_error "No installation found at $target (missing $manifest_file)"
        return 1
    fi

    log_info "Uninstalling from: $target"

    # Remove files based on installed manifest
    parse_installed_manifest "$manifest_file" "bin,script,agent,skill,config" | while read -r type path; do
        local dst

        case "$type" in
            bin)
                dst="$install_dir/$(basename "$path")"
                ;;
            script)
                dst="$install_dir/scripts/$(basename "$path")"
                ;;
            agent)
                dst="$claude_dir/agents/$(basename "$path")"
                ;;
            skill)
                dst="$claude_dir/skills/$(basename "$path")"
                ;;
            config)
                dst="$install_dir/$(basename "$path")"
                ;;
            *)
                dst="$install_dir/$path"
                ;;
        esac

        if [[ -e "$dst" ]]; then
            rm "$dst"
            log_info "Removed: $dst"
        fi
    done

    # Remove wrapper script
    if [[ -e "$target/orchestrate" ]]; then
        rm "$target/orchestrate"
        log_info "Removed: $target/orchestrate"
    fi

    # Remove manifest
    rm "$manifest_file"
    log_info "Removed: $manifest_file"

    # Clean up empty directories
    rmdir "$install_dir/scripts" 2>/dev/null || true
    rmdir "$install_dir" 2>/dev/null || true
    rmdir "$claude_dir/agents" 2>/dev/null || true
    rmdir "$claude_dir/skills" 2>/dev/null || true
    rmdir "$claude_dir" 2>/dev/null || true

    log_info "Uninstall complete"
}

install() {
    log_info "Installing to: $INSTALL_DIR"

    # Check requirements
    check_requirements

    # Backup existing if present
    if [[ -d "$INSTALL_DIR" ]]; then
        log_info "Existing installation found, backing up..."
        backup
    fi

    # Create base directories
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$BIN_DIR"

    local installed=0
    local skipped=0

    # Install files from manifest
    parse_manifest | while read -r type path mode; do
        local src="$SCRIPT_DIR/$path"
        local dst="$INSTALL_DIR/$path"

        if [[ ! -e "$src" ]]; then
            log_debug "Skipping (not found): $path"
            ((skipped++)) || true
            continue
        fi

        # Create parent directory
        mkdir -p "$(dirname "$dst")"

        # Copy file
        cp "$src" "$dst"
        chmod "$mode" "$dst"

        log_info "Installed: $path ($type)"
        ((installed++)) || true

        # Create symlink for bin/script types
        if [[ "$type" == "bin" || "$type" == "script" ]]; then
            local name=$(basename "$path")
            ln -sf "$dst" "$BIN_DIR/$name"
            log_debug "Symlinked: $BIN_DIR/$name -> $dst"
        fi
    done

    # Copy manifest itself
    cp "$MANIFEST_FILE" "$INSTALL_DIR/MANIFEST"

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
    local install_dir="$target/.orchestrate"
    local claude_dir="$target/.claude"
    local backup_zip="$install_dir/backup-$(date +%Y%m%d_%H%M%S).zip"
    local files_to_backup=()

    log_info "Installing to local project: $target"

    # Create directory structure
    mkdir -p "$install_dir/scripts"
    mkdir -p "$claude_dir/agents"
    mkdir -p "$claude_dir/skills"

    # First pass: collect files that will be overwritten
    parse_manifest "bin,script,agent,skill,config" | while read -r type path mode; do
        local dst

        case "$type" in
            bin)
                dst="$install_dir/$(basename "$path")"
                ;;
            script)
                dst="$install_dir/scripts/$(basename "$path")"
                ;;
            agent)
                dst="$claude_dir/agents/$(basename "$path")"
                ;;
            skill)
                dst="$claude_dir/skills/$(basename "$path")"
                ;;
            config)
                dst="$install_dir/$(basename "$path")"
                ;;
            *)
                dst="$install_dir/$path"
                ;;
        esac

        if [[ -e "$dst" ]]; then
            echo "$dst"
        fi
    done > /tmp/orchestrate_backup_list.txt

    # Check wrapper script too
    if [[ -e "$target/orchestrate" ]]; then
        echo "$target/orchestrate" >> /tmp/orchestrate_backup_list.txt
    fi

    # Create backup zip if there are files to backup
    if [[ -s /tmp/orchestrate_backup_list.txt ]]; then
        log_info "Backing up existing files to: $backup_zip"

        # Create zip from the list
        (cd "$target" && cat /tmp/orchestrate_backup_list.txt | while read -r file; do
            # Convert absolute path to relative
            rel_path="${file#$target/}"
            if [[ -e "$file" ]]; then
                echo "$rel_path"
            fi
        done | xargs zip -q "$backup_zip" 2>/dev/null) || true

        if [[ -f "$backup_zip" ]]; then
            log_info "Backup created: $backup_zip"
        fi
    fi
    rm -f /tmp/orchestrate_backup_list.txt

    # Install files
    parse_manifest "bin,script,agent,skill,config" | while read -r type path mode; do
        local src="$SCRIPT_DIR/$path"
        local dst

        # Map paths to appropriate directories
        case "$type" in
            bin)
                dst="$install_dir/$(basename "$path")"
                ;;
            script)
                dst="$install_dir/scripts/$(basename "$path")"
                ;;
            agent)
                # Keep in .claude/agents/
                dst="$claude_dir/agents/$(basename "$path")"
                ;;
            skill)
                # Keep in .claude/skills/
                dst="$claude_dir/skills/$(basename "$path")"
                ;;
            config)
                dst="$install_dir/$(basename "$path")"
                ;;
            *)
                dst="$install_dir/$path"
                ;;
        esac

        if [[ ! -e "$src" ]]; then
            log_debug "Skipping (not found): $path"
            continue
        fi

        mkdir -p "$(dirname "$dst")"
        cp "$src" "$dst"
        chmod "$mode" "$dst"

        log_info "Installed: $dst"
    done

    # Copy manifest
    cp "$MANIFEST_FILE" "$install_dir/MANIFEST"

    # Create wrapper script in project root
    cat > "$target/orchestrate" <<'WRAPPER'
#!/usr/bin/env bash
# Orchestrate wrapper - delegates to .orchestrate/orchestrate
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/.orchestrate/orchestrate" "$@"
WRAPPER
    chmod 755 "$target/orchestrate"

    log_info "Local installation complete!"
    echo ""
    echo "Installed to:"
    echo "  $install_dir/        - orchestrate scripts"
    echo "  $claude_dir/agents/  - Claude agents"
    echo "  $claude_dir/skills/  - Claude skills"
    echo ""
    echo "Usage:"
    echo "  cd $target"
    echo "  ./orchestrate help"
}

install_rust() {
    log_info "Building and installing Rust crates..."

    if ! command -v cargo &>/dev/null; then
        log_error "Cargo not found. Install Rust: https://rustup.rs"
        return 1
    fi

    cd "$SCRIPT_DIR"

    # Build release
    log_info "Building release..."
    cargo build --release

    # Install binary
    local bin_path="$SCRIPT_DIR/target/release/orchestrate"
    if [[ -f "$bin_path" ]]; then
        cp "$bin_path" "$BIN_DIR/orchestrate-rs"
        chmod 755 "$BIN_DIR/orchestrate-rs"
        log_info "Installed: $BIN_DIR/orchestrate-rs"
    fi

    log_info "Rust installation complete!"
}

list_manifest() {
    local filter="${1:-}"

    echo "Files in MANIFEST${filter:+ (filter: $filter)}:"
    echo ""

    printf "%-10s %-50s %s\n" "TYPE" "PATH" "MODE"
    printf "%-10s %-50s %s\n" "----" "----" "----"

    parse_manifest "$filter" | while read -r type path mode; do
        local exists=" "
        [[ -e "$SCRIPT_DIR/$path" ]] && exists="*"
        printf "%-10s %-50s %s %s\n" "$type" "$path" "$mode" "$exists"
    done

    echo ""
    echo "* = file exists in source"
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

verify_install() {
    log_info "Verifying installation..."

    local errors=0

    # Check each manifest entry
    parse_manifest | while read -r type path mode; do
        local dst="$INSTALL_DIR/$path"

        if [[ ! -e "$dst" ]]; then
            log_error "Missing: $path"
            ((errors++)) || true
        else
            log_debug "OK: $path"
        fi
    done

    # Check symlinks
    parse_manifest "bin,script" | while read -r type path mode; do
        local name=$(basename "$path")
        if [[ ! -L "$BIN_DIR/$name" ]]; then
            log_error "Missing symlink: $BIN_DIR/$name"
            ((errors++)) || true
        fi
    done

    if [[ $errors -eq 0 ]]; then
        log_info "Verification passed!"
    else
        log_error "Verification failed with $errors errors"
        return 1
    fi
}

show_help() {
    cat <<EOF
Orchestrate Installer

Usage: $0 <command> [args]

Commands:
  install              Install to ~/.local/share/orchestrate
  install-local [dir]  Install to local project directory
  install-rust         Build and install Rust crates
  uninstall            Remove global installation
  uninstall-local [dir] Remove local installation (uses installed manifest)
  backup               Backup current installation
  restore [backup]     Restore from backup (latest if not specified)
  list-backups         List available backups
  list [type]          List files from manifest (optionally filter by type)
  verify               Verify installation integrity
  help                 Show this help

File Types (in MANIFEST):
  bin      Executable binaries (symlinked to BIN_DIR)
  script   Helper scripts (symlinked to BIN_DIR)
  agent    Claude agent definitions
  doc      Documentation files
  config   Configuration templates

Environment Variables:
  ORCHESTRATE_INSTALL_DIR   Installation directory (default: ~/.local/share/orchestrate)
  ORCHESTRATE_BIN_DIR       Binary directory (default: ~/.local/bin)
  VERBOSE                   Enable debug output (set to "true")

Examples:
  $0 install                    # Install globally
  $0 install-local ./my-project # Install to project
  $0 list agent                 # List agent files
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
    install-rust|rust)
        install_rust
        ;;
    uninstall|remove)
        uninstall
        ;;
    uninstall-local|remove-local)
        uninstall_local "${1:-.}"
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
    list|manifest)
        list_manifest "${1:-}"
        ;;
    verify|check)
        verify_install
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
