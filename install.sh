#!/usr/bin/env bash
set -e

REPO="nusendra/claude-code-telegram"
BINARY_NAME="claude-telegram-aarch64"
INSTALL_PATH="/usr/local/bin/claude-telegram"
SERVICE_NAME="claude-telegram"
CONFIG_DIR="$HOME/.config/claude-telegram"
CONFIG_FILE="$CONFIG_DIR/.env"
SERVICE_FILE="/etc/systemd/system/$SERVICE_NAME.service"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${GREEN}[+]${NC} $1"; }
warn()    { echo -e "${YELLOW}[!]${NC} $1"; }
error()   { echo -e "${RED}[x]${NC} $1"; exit 1; }
ask()     { echo -e "${YELLOW}[?]${NC} $1"; }

echo ""
echo "  Claude Code Telegram Bot — Installer"
echo "  ======================================"
echo ""

# ── Prerequisites ──────────────────────────────────────────────────────────────

command -v curl  >/dev/null || error "curl is required. Install it with: sudo apt-get install curl"
command -v systemctl >/dev/null || error "systemd is required."

# ── Download binary ────────────────────────────────────────────────────────────

info "Downloading latest binary from GitHub..."
TMP=$(mktemp)
curl -fsSL "https://github.com/$REPO/releases/latest/download/$BINARY_NAME" -o "$TMP"
chmod +x "$TMP"
sudo mv "$TMP" "$INSTALL_PATH"
info "Binary installed to $INSTALL_PATH"

# ── Config ─────────────────────────────────────────────────────────────────────

mkdir -p "$CONFIG_DIR"

if [ -f "$CONFIG_FILE" ]; then
    warn "Config already exists at $CONFIG_FILE"
    ask "Overwrite it? [y/N]"
    read -r OVERWRITE </dev/tty
    if [[ ! "$OVERWRITE" =~ ^[Yy]$ ]]; then
        info "Keeping existing config."
        SKIP_CONFIG=true
    fi
fi

if [ -z "$SKIP_CONFIG" ]; then
    echo ""
    info "Setting up config at $CONFIG_FILE"
    echo "  (Get your bot token from @BotFather on Telegram)"
    echo "  (Get your user ID from @userinfobot on Telegram)"
    echo ""

    ask "Telegram bot token:"
    read -r BOT_TOKEN </dev/tty
    [ -z "$BOT_TOKEN" ] && error "Bot token cannot be empty."

    ask "Your Telegram user ID (numbers only):"
    read -r USER_ID </dev/tty
    [[ "$USER_ID" =~ ^[0-9]+$ ]] || error "User ID must be a number."

    CLAUDE_DEFAULT=$(command -v claude 2>/dev/null || echo "")
    ask "Full path to claude binary [${CLAUDE_DEFAULT:-not found, enter manually}]:"
    read -r CLAUDE_BIN </dev/tty
    CLAUDE_BIN="${CLAUDE_BIN:-$CLAUDE_DEFAULT}"
    [ -z "$CLAUDE_BIN" ] && error "claude binary path cannot be empty. Run 'which claude' to find it."
    [ -x "$CLAUDE_BIN" ] || warn "'$CLAUDE_BIN' does not exist or is not executable. Make sure it's correct."

    ask "Working directory for Claude [default: $HOME]:"
    read -r WORKING_DIR </dev/tty
    WORKING_DIR="${WORKING_DIR:-$HOME}"

    ask "Claude timeout in seconds [default: 300]:"
    read -r TIMEOUT </dev/tty
    TIMEOUT="${TIMEOUT:-300}"

    cat > "$CONFIG_FILE" <<EOF
TELEGRAM_BOT_TOKEN=$BOT_TOKEN
ALLOWED_USER_ID=$USER_ID
CLAUDE_BIN=$CLAUDE_BIN
WORKING_DIR=$WORKING_DIR
CLAUDE_TIMEOUT=$TIMEOUT
EOF

    chmod 600 "$CONFIG_FILE"
    info "Config saved."
fi

# ── Systemd service ────────────────────────────────────────────────────────────

info "Installing systemd service..."
sudo tee "$SERVICE_FILE" > /dev/null <<EOF
[Unit]
Description=Claude Code Telegram Bot
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$USER
ExecStart=$INSTALL_PATH
Restart=on-failure
RestartSec=5
EnvironmentFile=$CONFIG_FILE

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable "$SERVICE_NAME"
sudo systemctl restart "$SERVICE_NAME"

# ── Done ───────────────────────────────────────────────────────────────────────

echo ""
info "Installation complete!"
echo ""
echo "  Useful commands:"
echo "    sudo systemctl status $SERVICE_NAME     — check status"
echo "    journalctl -u $SERVICE_NAME -f          — live logs"
echo "    sudo systemctl restart $SERVICE_NAME    — restart"
echo "    sudo systemctl stop $SERVICE_NAME       — stop"
echo ""
echo "  To update the binary later, just re-run this script."
echo ""
