#!/bin/bash

# Installation script for wt-manager

set -e

echo "Installing wt-manager with cargo..."
cargo install --path .

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WRAPPER_PATH="$SCRIPT_DIR/wt-wrapper.sh"
ZSHRC="$HOME/.zshrc"

# Add wrapper to .zshrc if not already present
if [[ -f "$ZSHRC" ]]; then
    if ! grep -q "wt-wrapper.sh" "$ZSHRC"; then
        echo ""
        echo "Adding wt wrapper to ~/.zshrc..."
        echo "" >> "$ZSHRC"
        echo "# wt-manager: Auto-cd to worktree" >> "$ZSHRC"
        echo "source $WRAPPER_PATH" >> "$ZSHRC"
        echo "✓ Wrapper added to ~/.zshrc"
    else
        echo "✓ Wrapper already in ~/.zshrc"
    fi
else
    echo "⚠ ~/.zshrc not found. You can manually add:"
    echo "  source $WRAPPER_PATH"
fi

echo ""
echo "✓ Installation complete!"
echo ""
echo "🔄 현재 셸에서 wrapper를 활성화하려면:"
echo "  source ~/.zshrc"
echo ""
echo "또는 새 터미널을 열면 자동으로 활성화됩니다."
echo ""
echo "사용법:"
echo "  wt              # TUI로 워크트리 검색/생성"
echo "  wt <branch>     # 특정 브랜치 워크트리 생성/이동"
