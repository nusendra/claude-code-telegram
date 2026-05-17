use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

static ANSI_RE: OnceLock<Regex> = OnceLock::new();

fn ansi_re() -> &'static Regex {
    ANSI_RE.get_or_init(|| Regex::new(r"\x1b\[[0-9;]*[mGKHF]").unwrap())
}

#[derive(Deserialize)]
struct ClaudeJson {
    result: Option<String>,
    session_id: Option<String>,
}

pub struct ClaudeOutput {
    pub text: String,
    pub session_id: Option<String>,
}

pub async fn run(
    prompt: &str,
    session_id: Option<&str>,
    working_dir: &PathBuf,
    claude_bin: &str,
    timeout_secs: u64,
    model: Option<&str>,
) -> Result<ClaudeOutput> {
    let mut cmd = Command::new(claude_bin);
    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("json")
        .arg("--dangerously-skip-permissions")
        .current_dir(working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }

    if let Some(m) = model {
        cmd.arg("--model").arg(m);
    }

    let child = cmd.spawn()?;

    let raw = timeout(Duration::from_secs(timeout_secs), child.wait_with_output())
        .await
        .map_err(|_| anyhow::anyhow!("Claude timed out after {timeout_secs}s"))??;

    let stderr = String::from_utf8_lossy(&raw.stderr);
    if !stderr.trim().is_empty() {
        tracing::debug!("claude stderr: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&raw.stdout);
    let parsed: ClaudeJson = serde_json::from_str(stdout.trim())
        .map_err(|e| anyhow::anyhow!("Failed to parse Claude output: {e}\nRaw: {stdout}"))?;

    let text = ansi_re()
        .replace_all(parsed.result.as_deref().unwrap_or(""), "")
        .into_owned();

    Ok(ClaudeOutput {
        text,
        session_id: parsed.session_id,
    })
}
