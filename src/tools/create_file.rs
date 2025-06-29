use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::tools::{
    JsonSchemaField, JsonSchemaObject, Tool, ToolDefinition, ToolFunction, ToolType,
};

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct CreateFileParams {
    pub path: String,
    pub contents: Option<String>,
}

pub struct CreateFile;

#[async_trait]
impl Tool for CreateFile {
    fn name(&self) -> &'static str {
        "create_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description: "Create a file at a given path relative to the current working directory and optionally write contents to it."
                    .to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "path".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some(
                                    "The full path where the file should be created".to_string(),
                                ),
                                enum_values: None,
                            },
                        ),
                        (
                            "contents".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some(
                                    "Optional text content to write to the file".to_string(),
                                ),
                                enum_values: None,
                            },
                        ),
                    ]),
                    required: vec!["path".to_string()],
                },
            },
        }
    }

    async fn call(&self, args: Value) -> Result<String> {
        let params: CreateFileParams = serde_json::from_value(args)?;

        let mut file = File::create(&params.path).await?;
        if let Some(contents) = &params.contents {
            file.write_all(contents.as_bytes()).await?;
        }

        Ok(format!("File created at {}", params.path))
    }
}
