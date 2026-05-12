use std::path::PathBuf;
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::{ChatAction, ParseMode};
use tokio::sync::Mutex;
use tracing::{info, warn};

use crate::claude;
use crate::config::Config;
use crate::state::BotState;

const MAX_LEN: usize = 4096;

pub async fn cmd_help(bot: Bot, msg: Message) -> ResponseResult<()> {
    bot.send_message(
        msg.chat.id,
        "Claude Code Telegram Bot\n\n\
         Send any message to talk to Claude.\n\n\
         /new — clear session (fresh conversation)\n\
         /status — show session ID and working dir\n\
         /cd <path> — change working directory\n\
         /help — show this message",
    )
    .await?;
    Ok(())
}

pub async fn cmd_new(
    bot: Bot,
    msg: Message,
    state: Arc<Mutex<BotState>>,
) -> ResponseResult<()> {
    state.lock().await.session_id = None;
    info!("Session cleared");
    bot.send_message(msg.chat.id, "Session cleared. Starting fresh.").await?;
    Ok(())
}

pub async fn cmd_status(
    bot: Bot,
    msg: Message,
    state: Arc<Mutex<BotState>>,
) -> ResponseResult<()> {
    let s = state.lock().await;
    let session = s.session_id.as_deref().unwrap_or("none");
    let dir = s.working_dir.display();
    bot.send_message(msg.chat.id, format!("Session: {session}\nDir: {dir}"))
        .await?;
    Ok(())
}

pub async fn cmd_cd(
    bot: Bot,
    msg: Message,
    state: Arc<Mutex<BotState>>,
    arg: String,
) -> ResponseResult<()> {
    let arg = arg.trim().to_string();
    if arg.is_empty() {
        bot.send_message(msg.chat.id, "Usage: /cd <path>").await?;
        return Ok(());
    }

    let raw = PathBuf::from(&arg);
    let candidate = if raw.is_absolute() {
        raw
    } else {
        state.lock().await.working_dir.join(&raw)
    };

    match candidate.canonicalize() {
        Ok(canonical) if canonical.is_dir() => {
            state.lock().await.working_dir = canonical.clone();
            info!("Working dir → {}", canonical.display());
            bot.send_message(msg.chat.id, format!("Dir: {}", canonical.display()))
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, format!("Not a directory: {arg}")).await?;
        }
    }
    Ok(())
}

pub async fn on_message(
    bot: Bot,
    msg: Message,
    state: Arc<Mutex<BotState>>,
    config: Arc<Config>,
) -> ResponseResult<()> {
    let text = match msg.text() {
        Some(t) => t.to_string(),
        None => return Ok(()),
    };

    let (session_id, working_dir) = {
        let s = state.lock().await;
        (s.session_id.clone(), s.working_dir.clone())
    };

    info!(session = ?session_id, "Forwarding message to Claude");

    // Typing indicator — cancelled after Claude responds.
    let bot2 = bot.clone();
    let chat_id = msg.chat.id;
    let typing = tokio::spawn(async move {
        loop {
            let _ = bot2.send_chat_action(chat_id, ChatAction::Typing).await;
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
        }
    });

    let result = claude::run(
        &text,
        session_id.as_deref(),
        &working_dir,
        &config.claude_bin,
        config.claude_timeout,
    )
    .await;

    typing.abort();

    match result {
        Ok(output) => {
            if let Some(sid) = output.session_id {
                state.lock().await.session_id = Some(sid);
            }
            send_chunked(&bot, msg.chat.id, &output.text).await?;
        }
        Err(e) => {
            warn!("Claude error: {e:#}");
            bot.send_message(msg.chat.id, format!("Error: {e}")).await?;
        }
    }

    Ok(())
}

async fn send_chunked(bot: &Bot, chat_id: ChatId, text: &str) -> ResponseResult<()> {
    if text.is_empty() {
        bot.send_message(chat_id, "(empty response)").await?;
        return Ok(());
    }

    for chunk in chunks(text, MAX_LEN) {
        match bot
            .send_message(chat_id, &chunk)
            .parse_mode(ParseMode::MarkdownV2)
            .await
        {
            Ok(_) => {}
            Err(_) => {
                bot.send_message(chat_id, &chunk).await?;
            }
        }
    }
    Ok(())
}

fn chunks(text: &str, max: usize) -> Vec<String> {
    if text.len() <= max {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut rest = text;
    while rest.len() > max {
        let split = rest[..max].rfind('\n').unwrap_or(max);
        out.push(rest[..split].to_string());
        rest = rest[split..].trim_start_matches('\n');
    }
    if !rest.is_empty() {
        out.push(rest.to_string());
    }
    out
}
