use std::path::PathBuf;

pub struct BotState {
    pub session_id: Option<String>,
    pub working_dir: PathBuf,
    pub model: Option<String>,
}

impl BotState {
    pub fn new(working_dir: PathBuf) -> Self {
        Self {
            session_id: None,
            working_dir,
            model: None,
        }
    }
}
