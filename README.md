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

- Raspberry Pi running a 64-bit OS (Raspberry Pi OS, Ubuntu, etc.)
- [Claude Code CLI](https://claude.ai/code) installed and authenticated on the Pi
- `curl` (pre-installed on Raspberry Pi OS)

---

## Step 1 — Create a Telegram bot

1. Open Telegram and search for **@BotFather**
2. Send `/newbot`
3. Follow the prompts — pick a name and username (username must end in `bot`, e.g. `my_claude_bot`)
4. BotFather gives you a **bot token** like `123456789:ABCdef...` — save it

### Get your Telegram user ID

1. Search for **@userinfobot** on Telegram
2. Send `/start`
3. It replies with your numeric user ID — save it

---

## Step 2 — Install

Run the installer on your Pi. It downloads the binary, walks you through config, and sets up the systemd service so the bot starts automatically on boot.

```bash
curl -fsSL https://raw.githubusercontent.com/nusendra/claude-code-telegram/main/install.sh | bash
```

The installer will ask for:
- Your bot token (from @BotFather)
- Your Telegram user ID (from @userinfobot)
- The full path to the `claude` binary (`which claude` shows it)
- Working directory for Claude (default: your home directory)
- Timeout in seconds (default: 300)

That's it. The bot is running.

---

## Verify it's running

```bash
sudo systemctl status claude-telegram
```

Send your bot a message on Telegram — it should respond.

View live logs:

```bash
journalctl -u claude-telegram -f
```

---

## Commands

| Command | Description |
|---|---|
| `/start` or `/help` | Show available commands |
| `/new` | Clear the current session — Claude starts fresh with no memory of previous messages |
| `/status` | Show the current session ID, working directory, and active model |
| `/cd <path>` | Change the working directory. Accepts absolute or relative paths |
| `/model` | Show the current model |
| `/model <name>` | Switch model (e.g. `opus`, `sonnet`, `haiku`, or a full model ID like `claude-opus-4-7`) |
| `/model default` | Clear the model override and use the Claude CLI default |

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

## Updating

Download and run the update script — it fetches the latest release and restarts the service. Your config is preserved.

```bash
wget https://raw.githubusercontent.com/nusendra/claude-code-telegram/main/update.sh -O ~/update-claude-telegram.sh
chmod +x ~/update-claude-telegram.sh
~/update-claude-telegram.sh
```

After that, future updates are just:

```bash
~/update-claude-telegram.sh
```

---

## Useful service commands

```bash
sudo systemctl status claude-telegram    # check status
sudo systemctl restart claude-telegram   # restart
sudo systemctl stop claude-telegram      # stop
sudo systemctl start claude-telegram     # start
journalctl -u claude-telegram -f         # live logs
```

---

## Configuration

The config file lives at `~/.config/claude-telegram/.env`. Edit it directly if you need to change anything after install, then restart the service.

```env
TELEGRAM_BOT_TOKEN=123456789:ABCdef...
ALLOWED_USER_ID=123456789
CLAUDE_BIN=/home/pi/.npm-global/bin/claude
WORKING_DIR=/home/pi
CLAUDE_TIMEOUT=300
```

| Variable | Required | Default | Description |
|---|---|---|---|
| `TELEGRAM_BOT_TOKEN` | Yes | — | Token from @BotFather |
| `ALLOWED_USER_ID` | Yes | — | Your Telegram user ID. Only this user can use the bot |
| `CLAUDE_BIN` | No | `claude` | Full path to the claude binary. Required when running as a systemd service |
| `WORKING_DIR` | No | `$HOME` | Directory Claude operates in. Can be changed at runtime with `/cd` |
| `CLAUDE_TIMEOUT` | No | `300` | Seconds before Claude is force-killed if it doesn't respond |

---

## Troubleshooting

### Bot doesn't respond to messages

- Confirm `ALLOWED_USER_ID` matches your actual user ID (check with @userinfobot)
- Make sure you're messaging your own bot

### "Conflict" errors in logs

Another process is polling the same bot token. Telegram only allows one active consumer per token. Stop the other process, or create a fresh bot via @BotFather.

### Claude times out or never responds

- Test Claude directly on the Pi: `claude -p "hello"` should work in your terminal
- Increase `CLAUDE_TIMEOUT` if your Pi is under load
- Check logs: `journalctl -u claude-telegram -f`

### Service starts but Claude can't be found

systemd runs with a minimal PATH that often misses npm and local bin directories. Set `CLAUDE_BIN` to the full path:

```bash
which claude   # use this output as CLAUDE_BIN in your .env
```

Then restart: `sudo systemctl restart claude-telegram`

### Bot responds but text looks garbled

Long responses are automatically split at Telegram's 4096-character limit. If MarkdownV2 formatting fails, the bot falls back to plain text automatically.

---

## Building from source

### On the Pi

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

git clone https://github.com/nusendra/claude-code-telegram.git
cd claude-code-telegram
cargo build --release
```

The binary is at `target/release/claude-telegram`.

### Cross-compile from your laptop

```bash
cargo install cross
cross build --release --target aarch64-unknown-linux-musl
# binary: target/aarch64-unknown-linux-musl/release/claude-telegram
```

---

## Security notes

- **Single-user only.** The `ALLOWED_USER_ID` check means only you can use the bot. All other messages are silently ignored.
- **`--dangerously-skip-permissions` is intentional.** There's no way to approve Claude's permission prompts through Telegram. The security boundary is your Telegram user ID check, not Claude's per-tool prompts.
- **Telegram chats are not end-to-end encrypted.** Don't send passwords, API keys, or secrets through the bot.
- **Keep your bot token private.** Anyone with it can impersonate your bot. The `.env` file is excluded from version control.

---

## Architecture

```
Telegram (long-polling)
    │
    ▼
teloxide dispatcher
    │
    ├── /new, /status, /cd, /model, /help  ← command handlers
    │
    └── plain text  ──────────────► tokio::process::Command
                                         │
                                         │  claude -p "..." --output-format json
                                         │  --dangerously-skip-permissions
                                         │  [--resume <session_id>]
                                         │  [--model <model>]
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
