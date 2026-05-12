mod claude;
mod config;
mod handlers;
mod state;

use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::Mutex;
use tracing::info;

use config::Config;
use handlers::*;
use state::BotState;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Cmd {
    #[command(description = "Show help")]
    Start,
    #[command(description = "Show help")]
    Help,
    #[command(description = "Clear session")]
    New,
    #[command(description = "Show session and working dir")]
    Status,
    #[command(description = "Change working directory")]
    Cd(String),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "claude_code_telegram=info".into()),
        )
        .init();

    let config = Arc::new(Config::load()?);
    info!(
        allowed_user = config.allowed_user_id,
        claude_bin = %config.claude_bin,
        working_dir = %config.working_dir.display(),
        "Bot starting"
    );

    let bot = Bot::new(&config.telegram_bot_token);
    let state = Arc::new(Mutex::new(BotState::new(config.working_dir.clone())));

    let allowed_id = config.allowed_user_id;

    let handler = Update::filter_message()
        .filter(move |msg: Message| {
            msg.from
                .as_ref()
                .map(|u| u.id.0 == allowed_id)
                .unwrap_or(false)
        })
        .branch(
            dptree::entry()
                .filter_command::<Cmd>()
                .endpoint(
                    |bot: Bot, msg: Message, cmd: Cmd, state: Arc<Mutex<BotState>>, _config: Arc<Config>| async move {
                        match cmd {
                            Cmd::Start | Cmd::Help => cmd_help(bot, msg).await,
                            Cmd::New => cmd_new(bot, msg, state).await,
                            Cmd::Status => cmd_status(bot, msg, state).await,
                            Cmd::Cd(arg) => cmd_cd(bot, msg, state, arg).await,
                        }
                    },
                ),
        )
        .branch(dptree::endpoint(on_message));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
