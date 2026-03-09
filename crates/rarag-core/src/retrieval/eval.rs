use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalTaskFixture {
    pub task_id: String,
    pub revision: String,
    pub query_mode: String,
    pub query_text: String,
    pub symbol_path: Option<String>,
    pub ideal: Vec<String>,
    pub acceptable: Vec<String>,
    pub distractors: Vec<String>,
}

pub fn load_eval_task_fixtures(path: &Path) -> Result<Vec<EvalTaskFixture>, String> {
    let body = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read eval fixture {}: {err}", path.display()))?;
    serde_json::from_str(&body)
        .map_err(|err| format!("failed to parse eval fixture {}: {err}", path.display()))
}
