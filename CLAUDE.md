# claude-code-telegram (Rust)

A single-user Telegram bot wrapper for Claude Code, running on a Raspberry Pi 5.

## What it does

Forwards Telegram messages to the local `claude` CLI and sends responses back. Each message calls:

```
claude -p "<prompt>" --output-format json --dangerously-skip-permissions
```

The returned `session_id` is kept in memory and passed back via `--resume <session_id>` on the next message so Claude remembers the conversation. `/new` clears the session.

## Why Rust

Replaces an earlier Python prototype. The Python version worked but used ~50MB RAM at idle, needed Python + venv + pip on the Pi, and took ~2s to start. The Rust version is a single self-contained binary using ~3MB RAM with instant startup.

## Architecture

```
Telegram → teloxide (long-polling) → tokio::process spawns claude CLI → reply
                                              ↓
                                    Arc<Mutex<BotState>> tracks session_id
```

## Stack

- `teloxide` 0.13 — Telegram bot framework
- `tokio` (full features) — async runtime
- `tokio::process::Command` — async subprocess for the claude CLI
- `serde` + `serde_json` — parse Claude's JSON output
- `dotenvy` — load config from `~/.config/claude-telegram/.env`
- `regex` — strip ANSI escape codes from claude output
- `Arc<Mutex<BotState>>` — shared mutable session state across async tasks
- `dirs-next` — find the user's home directory cross-platform

## Features implemented

- `/start`, `/help` — welcome message
- `/new` — clear `session_id` for a fresh conversation
- `/status` — show current session ID and working dir
- Whitelist by `ALLOWED_USER_ID` (single-user bot)
- Live "typing..." indicator via `tokio::spawn` + `handle.abort()` pattern
- Message chunking for Telegram's 4096-char limit (prefers newline boundaries)
- ANSI escape code stripping from claude output
- MarkdownV2 send with plain-text fallback on parse error
- 300s timeout via `tokio::time::timeout()`
- Loads config from `~/.config/claude-telegram/.env`, falls back to local `.env`

## Configuration

Lives at `~/.config/claude-telegram/.env`:

| Variable | Required | Default | Notes |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | yes | — | From @BotFather |
| `ALLOWED_USER_ID` | yes | — | u64, only this user can chat with the bot |
| `WORKING_DIR` | no | `$HOME` | cwd for the claude subprocess |
| `CLAUDE_BIN` | no | `claude` | Full path needed if not on PATH |
| `CLAUDE_TIMEOUT` | no | `300` | Seconds before killing claude |

## Build & deploy

- **Build on Pi**: `cargo build --release` (5-10 min first time)
- **Cross-compile from laptop**: target `aarch64-unknown-linux-gnu`, scp the binary to Pi
- **Run**: tmux session for personal use, or systemd service for auto-start on boot (template included)

## Repo structure

```
claude-code-telegram/
├── src/
│   └── main.rs               # ~300 lines, everything in one file
├── Cargo.toml
├── .env.example
├── .gitignore
├── claude-telegram.service   # systemd unit (replace YOUR_USERNAME)
├── LICENSE                   # MIT
├── CLAUDE.md                 # this file
└── README.md
```

## Known issues and design decisions

### 1. Conflict errors are expected if another client polls the same bot

Telegram only allows one consumer per bot token. If another tool (n8n, the official Anthropic Telegram plugin, Hermes, etc.) polls the same token, both clients will alternate getting kicked. The fix is to use a dedicated bot for this project, or revoke the token and start fresh.

The bot suppresses the noisy Conflict stack traces in logs because they don't indicate a real failure.

### 2. The claude binary path is environment-dependent

Common locations:
- `/home/user/.npm-global/bin/claude` (npm install)
- `/usr/local/bin/claude`
- `/home/user/.local/bin/claude`

`which claude` finds it. systemd uses a stripped-down PATH that often doesn't include npm/local bin directories, so setting `CLAUDE_BIN` explicitly in `.env` is the reliable fix.

### 3. `--dangerously-skip-permissions` is intentional

The bot runs Claude with this flag because there's no way to approve permission prompts from Telegram chat. Claude can read/write files and run shell commands without prompting. This is acceptable because:

- The `ALLOWED_USER_ID` whitelist enforces single-user access
- The Pi is a personal device
- The trade-off is "ask once via .env config" vs "block forever on permission prompts"

The security boundary is the Telegram user ID check, not Claude's per-tool prompts.

### 4. Rust toolchain version

The project needs Rust 1.82+ because modern teloxide pulls in crates requiring `edition2024`. The apt-packaged Rust on Debian/Ubuntu is too old — install via rustup.

### 5. Telegram bot chats are not E2E encrypted

Documented in the README. No passwords, API keys, or secrets should go through the bot.

## Current state

Working end-to-end. Bot responds to messages, maintains session, handles /new and /status. Production-ready for personal use. Open improvements listed below.

## Possible next tasks

- Review code quality and suggest improvements
- Add a `/cd <path>` command to switch working directories at runtime
- Add voice message support (Whisper transcription → forward to Claude)
- Replace `--dangerously-skip-permissions` with explicit `--allowed-tools` list
- Add structured logging with `tracing` instead of `pretty_env_logger`
- Refactor `src/main.rs` into multiple modules (config, handlers, claude, state)
- Write integration tests with a mock Telegram server
- Set up GitHub Actions for cross-compiling release binaries
- Add multi-user support with per-user session tracking
- Add streaming output (use `--output-format stream-json`) for faster perceived response
- Support file uploads from Telegram → write to working dir before passing to Claude
