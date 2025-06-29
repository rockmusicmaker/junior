use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::fs;

use crate::tools::{
    JsonSchemaField, JsonSchemaObject, Tool, ToolDefinition, ToolFunction, ToolType,
};

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct MoveFileParams {
    pub from_path: String,
    pub to_path: String,
}

pub struct MoveFile;

#[async_trait]
impl Tool for MoveFile {
    fn name(&self) -> &'static str {
        "move_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description: "Move a file or directory from one path to another.".to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "from_path".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some(
                                    "The original file or directory path.".to_string(),
                                ),
                                enum_values: None,
                            },
                        ),
                        (
                            "to_path".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some("The destination path.".to_string()),
                                enum_values: None,
                            },
                        ),
                    ]),
                    required: vec!["from_path".to_string(), "to_path".to_string()],
                },
            },
        }
    }

    async fn call(&self, args: Value) -> Result<String> {
        let params: MoveFileParams = serde_json::from_value(args)?;

        fs::rename(&params.from_path, &params.to_path).await?;

        Ok(format!(
            "Moved from {} to {}",
            params.from_path, params.to_path
        ))
    }
}
