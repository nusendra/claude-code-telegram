#!/bin/bash
set -e

INSTALL_PATH="/usr/local/bin/claude-telegram"
REPO="nusendra/claude-code-telegram"

echo "Fetching latest release..."
LATEST=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
echo "Latest: $LATEST"

echo "Stopping service..."
sudo systemctl stop claude-telegram

echo "Downloading..."
TMPFILE="/tmp/claude-telegram-new"
sudo wget -q "https://github.com/$REPO/releases/download/$LATEST/claude-telegram-aarch64" -O "$TMPFILE"
sudo chmod +x "$TMPFILE"
sudo mv "$TMPFILE" "$INSTALL_PATH"

echo "Restarting service..."
sudo systemctl restart claude-telegram
sudo systemctl status claude-telegram --no-pager
