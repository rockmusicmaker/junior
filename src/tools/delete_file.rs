use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;

use crate::tools::{
    JsonSchemaField, JsonSchemaObject, Tool, ToolDefinition, ToolFunction, ToolType,
};

use std::collections::HashMap;
use trash::delete;

#[derive(Debug, Deserialize)]
pub struct DeleteFileParams {
    pub path: String,
}

pub struct DeleteFile;

#[async_trait]
impl Tool for DeleteFile {
    fn name(&self) -> &'static str {
        "delete_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            r#type: ToolType::Function,
            function: ToolFunction {
                name: self.name().to_string(),
                description: "Move a file to the system trash (instead of deleting permanently)."
                    .to_string(),
                parameters: JsonSchemaObject {
                    schema_type: "object".to_string(),
                    properties: HashMap::from([(
                        "path".to_string(),
                        JsonSchemaField {
                            field_type: "string".to_string(),
                            description: Some(
                                "The path to the file that should be moved to trash.".to_string(),
                            ),
                            enum_values: None,
                        },
                    )]),
                    required: vec!["path".to_string()],
                },
            },
        }
    }

    async fn call(&self, args: Value) -> Result<String> {
        let params: DeleteFileParams = serde_json::from_value(args)?;
        delete(&params.path)?;
        Ok(format!("File '{}' moved to trash.", params.path))
    }
}
