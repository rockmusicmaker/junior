use async_trait::async_trait;
use std::collections::HashMap;

pub mod create_file;
use create_file::CreateFile;

pub mod delete_file;
use delete_file::DeleteFile;

pub mod append_file;
use append_file::AppendFile;

pub mod create_dir;
use create_dir::CreateDir;

pub mod move_file;
use move_file::MoveFile;

pub mod write_file;
use write_file::WriteFile;

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    Function,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolDefinition {
    pub r#type: ToolType,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: JsonSchemaObject,
}

#[derive(Debug, Serialize, Clone)]
pub struct JsonSchemaObject {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, JsonSchemaField>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct JsonSchemaField {
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn definition(&self) -> ToolDefinition;

    async fn call(&self, args: serde_json::Value) -> anyhow::Result<String>;
}

pub fn tools_registry() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(CreateFile),
        Box::new(DeleteFile),
        Box::new(AppendFile),
        Box::new(CreateDir),
        Box::new(MoveFile),
        Box::new(WriteFile),
    ]
}

pub fn tool_definitions() -> Vec<ToolDefinition> {
    tools_registry()
        .iter()
        .map(|tool| tool.definition())
        .collect()
}
