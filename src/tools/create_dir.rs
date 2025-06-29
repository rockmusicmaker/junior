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
pub struct CreateDirParams {
    pub path: String,
}

pub struct CreateDir;

#[async_trait]
impl Tool for CreateDir {
    fn name(&self) -> &'static str {
        "create_dir"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description:
                    "Create a directory at the given path, including any parent directories."
                        .to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([(
                        "path".to_string(),
                        JsonSchemaField {
                            field_type: "string".to_string(),
                            description: Some("The path of the directory to create.".to_string()),
                            enum_values: None,
                        },
                    )]),
                    required: vec!["path".to_string()],
                },
            },
        }
    }

    async fn call(&self, args: Value) -> Result<String> {
        let params: CreateDirParams = serde_json::from_value(args)?;

        fs::create_dir_all(&params.path).await?;

        Ok(format!("Directory created at {}", params.path))
    }
}
