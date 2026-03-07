use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "rag_reindex".to_string(),
            description: "Index a Rust workspace snapshot via the daemon".to_string(),
        },
        ToolDefinition {
            name: "rag_query".to_string(),
            description: "Retrieve repository context for a task-oriented query".to_string(),
        },
        ToolDefinition {
            name: "rag_symbol_context".to_string(),
            description: "Retrieve focused symbol context for repository assistance".to_string(),
        },
        ToolDefinition {
            name: "rag_examples".to_string(),
            description: "Find relevant examples and tests for a symbol".to_string(),
        },
        ToolDefinition {
            name: "rag_blast_radius".to_string(),
            description: "Compute bounded blast radius around a symbol or query".to_string(),
        },
        ToolDefinition {
            name: "rag_index_status".to_string(),
            description: "Resolve current daemon snapshot state".to_string(),
        },
    ]
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
