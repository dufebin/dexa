#!/usr/bin/env bash
# Install wechat-agent skills to ~/.claude/skills/
# Usage: ./skills/install.sh [--target <dir>]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="${1:-}"

if [[ "$TARGET" == "--target" ]]; then
    TARGET="$2"
fi

if [[ -z "$TARGET" ]]; then
    # Default: ~/.claude/skills on macOS/Linux, %APPDATA%\.claude\skills on Windows
    if [[ -n "${APPDATA:-}" ]]; then
        TARGET="$APPDATA/.claude/skills"
    else
        TARGET="$HOME/.claude/skills"
    fi
fi

echo "Installing wechat-agent skills to: $TARGET"

for skill_dir in "$SCRIPT_DIR"/*/; do
    skill_name="$(basename "$skill_dir")"
    src="$skill_dir/SKILL.md"
    dst_dir="$TARGET/$skill_name"

    if [[ ! -f "$src" ]]; then
        continue
    fi

    mkdir -p "$dst_dir"

    # Don't overwrite a previously generated wechat-self (personalized)
    if [[ "$skill_name" == "wechat-self" && -f "$dst_dir/SKILL.md" ]]; then
        echo "  Skipping wechat-self (personalized version exists — run 'wx-agent distill self' to refresh)"
        continue
    fi

    cp "$src" "$dst_dir/SKILL.md"
    echo "  Installed: $skill_name"
done

echo ""
echo "Done. Next steps:"
echo "  1. wx-agent distill contact <name>  — distill key contacts"
echo "  2. wx-agent distill self            — generate personalized /wechat-self"
echo "  3. wx-agent watch                   — start the auto-reply daemon"
