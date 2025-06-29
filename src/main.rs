use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use confy;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shellexpand;
use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
mod tools;
use tools::tool_definitions;

use crate::tools::{ToolDefinition, tools_registry};

#[derive(Default, Debug, Deserialize, Serialize)]
struct Config {
    api_key: String,
    model: String,
    endpoint: String,
    history_directory_path: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: Role,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Vec<ToolDefinition>,
    tool_choice: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Serialize)]
struct ChatSessionLog {
    model: String,
    tools: Vec<ToolDefinition>,
    messages: Vec<ChatMessage>,
}

fn load_config() -> Result<Configuration> {
    let config_path = dirs::home_dir()
        .ok_or(anyhow!("Failed to find home directory"))?
        .join(".junior.toml");
    let config: Config = confy::load_path(config_path)?;
    let history_path =
        PathBuf::from(shellexpand::full(&config.history_directory_path)?.to_string());
    Ok(Configuration {
        api_key: config.api_key,
        endpoint: config.endpoint,
        log_file: create_session_file(&history_path)?,
        model: config.model,
    })
}

fn create_session_file(history_path: &PathBuf) -> Result<PathBuf> {
    fs::create_dir_all(&history_path)?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    Ok(history_path.join(format!("session-{}.json", timestamp)))
}

async fn send_to_llm(
    messages: &[ChatMessage],
    model: &str,
    endpoint: &str,
    api_key: &str,
    tool_definitions: &[ToolDefinition],
) -> Result<ChatMessage> {
    let client = Client::new();

    let request_body = ChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        tools: tool_definitions.to_vec(),
        tool_choice: "auto".to_string(),
    };

    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        return Err(anyhow!("Request failed with status: {} - {}", status, text));
    }

    let response_json: ChatResponse = response.json().await?;
    let message = response_json
        .choices
        .get(0)
        .ok_or(anyhow!("No response from model"))?
        .message
        .clone();

    Ok(message)
}

fn save_log(log: &ChatSessionLog, path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(log)?;
    fs::write(path, json)?;
    Ok(())
}

async fn send_message(
    message: String,
    log: &mut ChatSessionLog,
    options: &Configuration,
    tool_definitions: &[ToolDefinition],
) -> Result<ChatMessage> {
    let user_message = ChatMessage {
        role: Role::User,
        content: Some(message),
        tool_calls: None,
    };
    log.messages.push(user_message.clone());
    save_log(log, &options.log_file)?;

    let response = send_to_llm(
        &log.messages,
        &options.model,
        &options.endpoint,
        &options.api_key,
        tool_definitions,
    )
    .await?;

    if let Some(content) = &response.content {
        let assistant_message = ChatMessage {
            role: Role::Assistant,
            content: Some(content.clone()),
            tool_calls: None,
        };
        log.messages.push(assistant_message);
        save_log(log, &options.log_file)?;
    }

    if let Some(tool_calls) = &response.tool_calls {
        let tool_call_message = ChatMessage {
            role: Role::Assistant,
            content: None,
            tool_calls: Some(tool_calls.clone()),
        };
        log.messages.push(tool_call_message);
        save_log(log, &options.log_file)?;
    }

    Ok(response)
}

struct Configuration {
    model: String,
    log_file: PathBuf,
    api_key: String,
    endpoint: String,
}

fn initialize_log(
    system_prompt: String,
    model: String,
    tools: &[ToolDefinition],
    context: Option<String>,
) -> Result<ChatSessionLog> {
    let mut history = Vec::new();

    let system_prompt = ChatMessage {
        role: Role::System,
        content: Some(system_prompt),
        tool_calls: None,
    };
    history.push(system_prompt);

    if let Some(ctx) = context {
        history.push(ChatMessage {
            role: Role::User,
            content: Some(format!("Let's take a look at this together:\n\n{}", ctx)),
            tool_calls: None,
        })
    }

    Ok(ChatSessionLog {
        model,
        tools: tools.to_vec(),
        messages: history,
    })
}

fn sanitize_path_string(path_str: &str) -> String {
    let path = Path::new(path_str);

    if path.is_absolute() {
        path_str.to_string()
    } else if path_str.starts_with("./") || path_str.starts_with("../") {
        path_str.to_string()
    } else {
        format!("./{}", path_str)
    }
}

fn sanitize_and_resolve_path(path_str: &str) -> Result<PathBuf> {
    let current_dir = env::current_dir()?;
    let full_path = current_dir.join(path_str);
    let canonical_cwd = current_dir.canonicalize()?;
    let normalized = full_path.components().collect::<PathBuf>();
    if !normalized.starts_with(&canonical_cwd) {
        return Err(anyhow!(
            "Unsafe path: '{}' is outside of working directory '{}'",
            normalized.display(),
            canonical_cwd.display()
        ));
    }

    Ok(normalized)
}

async fn execute_tool_call(tool_call: &ToolCall) -> Result<()> {
    let mut args: Value = serde_json::from_str(&tool_call.function.arguments)?;

    println!(
        "[Tool Call] {} with args: {}",
        tool_call.function.name, args
    );

    if let Some(path_str) = args.get("path").and_then(|v| v.as_str()) {
        let path_str = sanitize_path_string(path_str);
        let safe_path = sanitize_and_resolve_path(&path_str)?;

        if let Some(obj) = args.as_object_mut() {
            obj.insert(
                "path".to_string(),
                Value::String(safe_path.to_string_lossy().to_string()),
            );
        }
    }

    let tool = tools_registry()
        .into_iter()
        .find(|t| t.name() == tool_call.function.name)
        .ok_or_else(|| anyhow!("Unknown tool function: {}", tool_call.function.name))?;

    let output = tool.call(args).await?;
    println!("[Tool Output] {}", output);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("junior")
        .version("0.1.0")
        .author("Hunter Horby")
        .about("A CLI interface for LLMs")
        .arg(
            Arg::new("prompt")
                .help("The prompt to send to the LLM")
                .index(1),
        )
        .arg(
            Arg::new("file")
                .long("file")
                .short('f')
                .value_name("FILE")
                .help("Path to a file whose contents will be appended to the prompt"),
        )
        .get_matches();

    let config = load_config()?;

    let prompt = matches
        .get_one::<String>("prompt")
        .map(|s| s.as_str())
        .unwrap_or("");
    let system_prompt = include_str!("system_prompt.md").to_string();
    let tool_definitions = tool_definitions();

    let mut additional_context: Option<String> = None;
    if let Some(file_path) = matches.get_one::<String>("file") {
        let mut contents = String::new();
        File::open(file_path)?.read_to_string(&mut contents)?;
        additional_context = Some(contents);
    }
    let mut log = initialize_log(
        system_prompt.clone(),
        config.model.clone(),
        &tool_definitions,
        additional_context,
    )
    .unwrap();

    let response = send_message(prompt.to_string(), &mut log, &config, &tool_definitions).await?;

    if let Some(content) = &response.content {
        println!("{}", content);
    }

    if let Some(tool_calls) = &response.tool_calls {
        for tool_call in tool_calls {
            if let Err(e) = execute_tool_call(tool_call).await {
                eprintln!("❌ Error executing tool call: {}", e);
                break;
            }
        }
    }

    Ok(())
}
