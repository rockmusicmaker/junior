use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

use crate::tools::{
    JsonSchemaField, JsonSchemaObject, Tool, ToolDefinition, ToolFunction, ToolType,
};

use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct AppendFileParams {
    pub path: String,
    pub contents: String,
}

pub struct AppendFile;

#[async_trait]
impl Tool for AppendFile {
    fn name(&self) -> &'static str {
        "append_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description: "Append text to a file at the given path.".to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "path".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some(
                                    "The path to the file to which content should be appended."
                                        .to_string(),
                                ),
                                enum_values: None,
                            },
                        ),
                        (
                            "contents".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some("The content to append to the file.".to_string()),
                                enum_values: None,
                            },
                        ),
                    ]),
                    required: vec!["path".to_string(), "contents".to_string()],
                },
            },
        }
    }

    async fn call(&self, args: Value) -> Result<String> {
        let params: AppendFileParams = serde_json::from_value(args)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&params.path)
            .await?;

        file.write_all(params.contents.as_bytes()).await?;

        Ok(format!("Appended to file at {}", params.path))
    }
}
