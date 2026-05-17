#!/bin/bash
set -e

INSTALL_PATH="/usr/local/bin/claude-telegram"
REPO="nusendra/claude-code-telegram"

echo "Fetching latest release..."
LATEST=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
echo "Latest: $LATEST"

echo "Downloading..."
sudo wget -q "https://github.com/$REPO/releases/download/$LATEST/claude-telegram-aarch64" -O "$INSTALL_PATH"
sudo chmod +x "$INSTALL_PATH"

echo "Restarting service..."
sudo systemctl restart claude-telegram
sudo systemctl status claude-telegram --no-pager
