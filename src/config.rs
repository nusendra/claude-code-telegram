use std::path::PathBuf;

pub struct Config {
    pub telegram_bot_token: String,
    pub allowed_user_id: u64,
    pub working_dir: PathBuf,
    pub claude_bin: String,
    pub claude_timeout: u64,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        if let Some(home) = dirs_next::home_dir() {
            let path = home.join(".config").join("claude-telegram").join(".env");
            if path.exists() {
                dotenvy::from_path(path).ok();
            }
        }
        dotenvy::dotenv().ok();

        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))?;
        let allowed_user_id = std::env::var("ALLOWED_USER_ID")
            .map_err(|_| anyhow::anyhow!("ALLOWED_USER_ID not set"))?
            .parse::<u64>()
            .map_err(|_| anyhow::anyhow!("ALLOWED_USER_ID must be a u64"))?;
        let working_dir = std::env::var("WORKING_DIR")
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/")));
        let claude_bin =
            std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".to_string());
        let claude_timeout = std::env::var("CLAUDE_TIMEOUT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300u64);

        Ok(Config {
            telegram_bot_token,
            allowed_user_id,
            working_dir,
            claude_bin,
            claude_timeout,
        })
    }
}
