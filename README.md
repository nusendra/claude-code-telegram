# claude-code-telegram

A single-user Telegram bot that wraps the [Claude Code](https://claude.ai/code) CLI. Send messages from your phone, get Claude responses back — with full session memory, working directory control, and a live typing indicator.

Built in Rust. Runs on a Raspberry Pi 5 using ~3 MB RAM.

---

## How it works

```
You (Telegram) → Bot → claude CLI → Response back to you
```

Every message you send is forwarded to `claude -p "<your message>"`. The returned `session_id` is kept in memory so Claude remembers the full conversation. `/new` resets it. `/cd` lets you switch the working directory so Claude operates in the right folder.

---

## Prerequisites

### On your phone / Telegram
- A Telegram account

### On the Raspberry Pi
- Raspberry Pi OS (64-bit recommended)
- [Claude Code CLI](https://claude.ai/code) installed and authenticated
- `curl` (pre-installed on Raspberry Pi OS)

### On your laptop (optional, for building yourself)
- [Rust toolchain](https://rustup.rs) 1.82 or newer

---

## Step 1 — Create a Telegram bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot`
3. Follow the prompts — pick a name and a username (must end in `bot`, e.g. `my_claude_bot`)
4. BotFather gives you a **bot token** that looks like `123456789:ABCdef...` — save it

### Get your Telegram user ID

1. Search for **@userinfobot** on Telegram
2. Send `/start`
3. It replies with your numeric user ID — save it

---

## Step 2 — Install the binary on the Pi

### Option A — Download from GitHub Releases (recommended)

```bash
curl -L https://github.com/nusendra/claude-code-telegram/releases/latest/download/claude-telegram-aarch64 \
  -o ~/claude-telegram
chmod +x ~/claude-telegram
```

### Option B — Build on the Pi

```bash
# Install Rust if you haven't
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build (first build takes 5–10 min)
git clone https://github.com/nusendra/claude-code-telegram.git
cd claude-code-telegram
cargo build --release
cp target/release/claude-telegram ~/claude-telegram
```

### Option C — Cross-compile from your laptop

```bash
# Add the ARM64 target
rustup target add aarch64-unknown-linux-gnu

# Install the cross-linker (macOS)
brew install FiloSottile/musl-cross/musl-cross

# Or on Linux/WSL
sudo apt-get install gcc-aarch64-linux-gnu

# Build
CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
  cargo build --release --target aarch64-unknown-linux-gnu

# Copy to Pi
scp target/aarch64-unknown-linux-gnu/release/claude-telegram pi@raspberrypi.local:~/claude-telegram
```

---

## Step 3 — Configure the bot

Create the config directory and file on the Pi:

```bash
mkdir -p ~/.config/claude-telegram
nano ~/.config/claude-telegram/.env
```

Paste this template and fill in your values:

```env
# Required
TELEGRAM_BOT_TOKEN=123456789:ABCdef...
ALLOWED_USER_ID=123456789

# Optional — defaults shown
WORKING_DIR=/home/pi
CLAUDE_BIN=claude
CLAUDE_TIMEOUT=300
```

### Configuration reference

| Variable | Required | Default | Description |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | Yes | — | Token from @BotFather |
| `ALLOWED_USER_ID` | Yes | — | Your Telegram user ID. Only this user can talk to the bot |
| `WORKING_DIR` | No | `$HOME` | Directory Claude operates in. Can be changed at runtime with `/cd` |
| `CLAUDE_BIN` | No | `claude` | Full path to the claude binary. Use this if systemd can't find it |
| `CLAUDE_TIMEOUT` | No | `300` | Seconds before Claude is killed if it doesn't respond |

### Finding the Claude binary path

```bash
which claude
```

Common locations:
- `/home/pi/.npm-global/bin/claude` (npm install)
- `/home/pi/.local/bin/claude`
- `/usr/local/bin/claude`

If you plan to run the bot as a systemd service, set `CLAUDE_BIN` to the full path — systemd uses a stripped-down PATH that often misses npm and local bin directories.

---

## Step 4 — Run the bot

### Quick test (manual)

```bash
~/claude-telegram
```

Open Telegram, send your bot a message. You should see a response. Press `Ctrl+C` to stop.

Check logs with `RUST_LOG` for more detail:

```bash
RUST_LOG=claude_code_telegram=debug ~/claude-telegram
```

### Keep it running with tmux

```bash
tmux new -s claude-bot
~/claude-telegram
# Detach: Ctrl+B then D
# Reattach later: tmux attach -t claude-bot
```

### Run as a systemd service (auto-start on boot)

Edit the service file to replace `YOUR_USERNAME` with your Pi username (usually `pi`):

```bash
sed -i 's/YOUR_USERNAME/pi/g' ~/claude-code-telegram/claude-telegram.service
```

Or open it and edit manually:

```bash
nano ~/claude-code-telegram/claude-telegram.service
```

Install and enable:

```bash
sudo cp ~/claude-code-telegram/claude-telegram.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable claude-telegram
sudo systemctl start claude-telegram
```

Check it's running:

```bash
sudo systemctl status claude-telegram
```

View live logs:

```bash
journalctl -u claude-telegram -f
```

---

## Commands

| Command | Description |
|---|---|
| `/start` or `/help` | Show available commands |
| `/new` | Clear the current session. Claude starts fresh with no memory of previous messages |
| `/status` | Show the current session ID and working directory |
| `/cd <path>` | Change the working directory. Accepts absolute or relative paths |

### Example session

```
You:   /cd /home/pi/my-project
Bot:   Dir: /home/pi/my-project

You:   What files are in this directory?
Bot:   (typing...)
Bot:   Here are the files in /home/pi/my-project: ...

You:   Create a Python script that reads all .txt files
Bot:   (typing...)
Bot:   I've created read_files.py with the following content: ...

You:   /new
Bot:   Session cleared. Starting fresh.
```

---

## Updating the binary

When a new release is published, update the binary on the Pi with one command:

```bash
curl -L https://github.com/nusendra/claude-code-telegram/releases/latest/download/claude-telegram-aarch64 \
  -o ~/claude-telegram && chmod +x ~/claude-telegram

# Restart the service
sudo systemctl restart claude-telegram
```

---

## Troubleshooting

### Bot doesn't respond to messages

- Check that `ALLOWED_USER_ID` matches your actual Telegram user ID (get it from @userinfobot)
- Make sure you're messaging your own bot, not someone else's

### "Conflict" errors in logs

Another process is polling the same bot token. Telegram only allows one active consumer per token. Stop the other process, or create a new bot via @BotFather.

### Claude times out or never responds

- Run `~/claude-telegram` manually (not as a service) and send a message — watch the terminal output
- Make sure Claude is properly authenticated: `claude -p "hello"` should work in your terminal
- Try increasing `CLAUDE_TIMEOUT` if your Pi is slow

### Service starts but Claude can't be found

systemd runs with a minimal PATH. Set `CLAUDE_BIN` to the full absolute path in your `.env`:

```bash
which claude   # copy this output into CLAUDE_BIN
```

### Bot responds but output looks garbled

Long responses are automatically split into chunks at 4096 characters (Telegram's limit). If formatting looks wrong, the bot falls back from MarkdownV2 to plain text automatically.

---

## Security notes

- **Single-user only.** The `ALLOWED_USER_ID` check means only you can interact with the bot. All other messages are silently ignored.
- **`--dangerously-skip-permissions` is intentional.** There's no way to approve Claude's permission prompts from a Telegram chat. This flag is safe here because the security boundary is your Telegram user ID, not Claude's per-tool prompts.
- **Telegram chats are not end-to-end encrypted.** Don't send passwords, API keys, or secrets through the bot.
- **The bot token is sensitive.** Anyone with it can impersonate your bot. Keep `.env` out of version control (it's in `.gitignore`).

---

## Architecture

```
Telegram (long-polling)
    │
    ▼
teloxide dispatcher
    │
    ├── /new, /status, /cd, /help  ← command handlers
    │
    └── plain text  ──────────────► tokio::process::Command
                                         │
                                         │  claude -p "..." --output-format json
                                         │  --dangerously-skip-permissions
                                         │  [--resume <session_id>]
                                         │
                                         ▼
                                    parse JSON output
                                    strip ANSI codes
                                    update session_id in Arc<Mutex<BotState>>
                                         │
                                         ▼
                                    send reply (chunked if >4096 chars)
```

**Stack:** `teloxide` · `tokio` · `serde_json` · `dotenvy` · `regex` · `tracing` · `dirs-next`

---

## License

MIT
