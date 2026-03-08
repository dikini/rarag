use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "rag_reindex".to_string(),
            description: "Index a Rust workspace snapshot via the daemon".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "repo_root": { "type": "string" },
                    "worktree_root": { "type": "string" },
                    "git_sha": { "type": "string" },
                    "workspace_root": { "type": "string" }
                },
                "required": ["repo_root", "worktree_root", "git_sha", "workspace_root"]
            }),
        },
        ToolDefinition {
            name: "rag_query".to_string(),
            description: "Retrieve repository context for a task-oriented query".to_string(),
            input_schema: query_schema(true),
        },
        ToolDefinition {
            name: "rag_symbol_context".to_string(),
            description: "Retrieve focused symbol context for repository assistance".to_string(),
            input_schema: query_schema(false),
        },
        ToolDefinition {
            name: "rag_examples".to_string(),
            description: "Find relevant examples and tests for a symbol".to_string(),
            input_schema: query_schema(false),
        },
        ToolDefinition {
            name: "rag_blast_radius".to_string(),
            description: "Compute bounded blast radius around a symbol or query".to_string(),
            input_schema: query_schema(false),
        },
        ToolDefinition {
            name: "rag_index_status".to_string(),
            description: "Resolve current daemon snapshot state".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "snapshot_id": { "type": "string" },
                    "worktree_root": { "type": "string" }
                }
            }),
        },
    ]
}

fn query_schema(mode_required: bool) -> Value {
    let mut schema = json!({
        "type": "object",
        "properties": {
            "snapshot_id": { "type": "string" },
            "worktree_root": { "type": "string" },
            "mode": { "type": "string" },
            "text": { "type": "string" },
            "symbol_path": { "type": "string" },
            "limit": { "type": "integer" },
            "changed_paths": {
                "type": "array",
                "items": { "type": "string" }
            }
        },
        "required": ["text"]
    });

    if mode_required
        && let Some(required) = schema.get_mut("required").and_then(Value::as_array_mut)
    {
        required.push(Value::String("mode".to_string()));
    }

    schema
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpRequest {
    ListTools,
    CallTool { name: String, arguments: Value },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum McpResponse {
    Tools { tools: Vec<ToolDefinition> },
    CallResult { result: Value },
    Error { message: String },
}
