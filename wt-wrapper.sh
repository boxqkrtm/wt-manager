#!/bin/bash
# wt wrapper function for automatic directory changing
# Add this to your ~/.zshrc or ~/.bashrc:
#
#   source /path/to/wt-manager/wt-wrapper.sh
#

trash_or_rm() {
    if command -v trash >/dev/null 2>&1; then
        trash "$@"
    else
        rm -f "$@"
    fi
}

wt() {
    local wt_bin="${HOME}/.cargo/bin/wt"
    
    # If binary doesn't exist in ~/.cargo/bin, try current directory
    if [[ ! -f "$wt_bin" ]]; then
        local script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        wt_bin="$script_dir/target/release/wt"
    fi
    
    if [[ ! -f "$wt_bin" ]]; then
        echo "Error: wt binary not found. Run ./install.sh first."
        return 1
    fi
    
    # Create temporary file for output
    local tmp_output=$(mktemp)
    
    # Run wt and capture output
    "$wt_bin" "$@" | tee "$tmp_output"
    local exit_code=${PIPESTATUS[0]}
    
    # Look for "cd " command in output
    local cd_line=$(grep "^  cd " "$tmp_output" | head -n1)
    
    if [[ -n "$cd_line" ]]; then
        # Extract directory path
        local target_dir=$(echo "$cd_line" | sed 's/^  cd //')
        
        # Change to directory
        if [[ -d "$target_dir" ]]; then
            cd "$target_dir" || {
                trash_or_rm "$tmp_output"
                return 1
            }
            echo ""
            echo "✓ Changed to: $(pwd)"
        fi
    fi
    
    trash_or_rm "$tmp_output"
    return $exit_code
}

# For zsh compatibility
if [[ -n "$ZSH_VERSION" ]]; then
    # Function already defined above
    :
fi
