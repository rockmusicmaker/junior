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
pub struct WriteFileParams {
    pub path: String,
    pub contents: String,
}

pub struct WriteFile;

#[async_trait]
impl Tool for WriteFile {
    fn name(&self) -> &'static str {
        "write_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description: "Write contents to a file at the specified path, overwriting if the file exists."
                    .to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([
                        (
                            "path".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some("The path to the file.".to_string()),
                                enum_values: None,
                            },
                        ),
                        (
                            "contents".to_string(),
                            JsonSchemaField {
                                field_type: "string".to_string(),
                                description: Some("Contents to write into the file.".to_string()),
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
        let params: WriteFileParams = serde_json::from_value(args)?;

        let mut file = File::create(&params.path).await?;
        file.write_all(params.contents.as_bytes()).await?;

        Ok(format!("Wrote to file at {}", params.path))
    }
}
