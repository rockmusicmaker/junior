use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use confy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shellexpand;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::io::AsyncWriteExt;

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

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: Role,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
enum ActionType {
    CreateFile,
    WriteFile,
    AppendFile,
    ReadFile,
    DeleteFile,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Action {
    action_type: ActionType,
    path: String,
    content: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ActionSet {
    explanation: String,
    actions: Vec<Action>,
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
) -> Result<String> {
    let client = Client::new();

    let request_body = ChatRequest {
        model: model.to_string(),
        messages: messages.to_vec(),
        stream: false,
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
    let reply = response_json
        .choices
        .get(0)
        .ok_or(anyhow!("No response from model"))?
        .message
        .content
        .clone();

    Ok(reply)
}

fn save_messages(history: &[ChatMessage], path: &PathBuf) -> Result<()> {
    let json = serde_json::to_string_pretty(history)?;
    fs::write(path, json)?;
    Ok(())
}

async fn send_message(
    message: String,
    history: &mut Vec<ChatMessage>,
    options: &Configuration,
) -> Result<String> {
    history.push(ChatMessage {
        role: Role::User,
        content: message,
    });
    save_messages(history, &options.log_file)?;
    let response =
        send_to_llm(history, &options.model, &options.endpoint, &options.api_key).await?;
    history.push(ChatMessage {
        role: Role::Assistant,
        content: response.clone(),
    });
    save_messages(history, &options.log_file)?;
    Ok(response)
}

struct Configuration {
    model: String,
    log_file: PathBuf,
    api_key: String,
    endpoint: String,
}

fn initialize_history(system_prompt: String, context: Option<String>) -> Result<Vec<ChatMessage>> {
    let mut history = Vec::new();

    let system_prompt = ChatMessage {
        role: Role::System,
        content: system_prompt,
    };
    history.push(system_prompt);

    if let Some(ctx) = context {
        history.push(ChatMessage {
            role: Role::User,
            content: format!("Let's take a look at this together:\n\n{}", ctx),
        })
    }

    Ok(history)
}

fn parse_llm_output(output: &String) -> Result<ActionSet> {
    let code_block_re = Regex::new(r"(?s)```(?:json)?\s*(\{.*?\})\s*```").unwrap();
    if let Some(captures) = code_block_re.captures(output) {
        let json_str = captures.get(1).unwrap().as_str();
        return serde_json::from_str::<ActionSet>(json_str)
            .map_err(|e| anyhow!("Failed to parse ActionSet JSON from code block: {}", e));
    }

    if let Some(start) = output.find('{') {
        let mut brace_count = 0;
        for (i, ch) in output[start..].char_indices() {
            match ch {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        let json_candidate = &output[start..start + i + 1];
                        return serde_json::from_str::<ActionSet>(json_candidate).map_err(|e| {
                            anyhow!("Failed to parse ActionSet JSON from loose text: {}", e)
                        });
                    }
                }
                _ => {}
            }
        }
    }

    Err(anyhow!("Could not find a valid JSON object in LLM output"))
}

fn is_path_safe(path: &str) -> bool {
    if !path.contains('/') {
        return true;
    }

    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(_) => return false,
    };

    let resolved_path = match Path::new(path).canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };

    resolved_path.starts_with(&current_dir)
}

async fn create_file(path: &str) -> Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all("".as_bytes())?;
    Ok(())
}

fn write_file(path: &str, content: &String) -> Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

async fn append_file(path: &str, content: &String) -> Result<()> {
    use tokio::fs::OpenOptions;

    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .await?;

    file.write_all(content.as_bytes()).await?;
    Ok(())
}

fn read_file(path: &String) -> Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

fn delete_file(path: &String) -> Result<()> {
    fs::remove_file(path)?;
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
                .index(1)
                .required(true),
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

    let mut additional_context: Option<String> = None;
    if let Some(file_path) = matches.get_one::<String>("file") {
        let mut contents = String::new();
        File::open(file_path)?.read_to_string(&mut contents)?;
        additional_context = Some(contents);
    }
    let mut history = initialize_history(system_prompt, additional_context).unwrap();

    let response = send_message(prompt.to_string(), &mut history, &config)
        .await
        .unwrap();
    let action_set = parse_llm_output(&response).unwrap();

    println!("{}", action_set.explanation);
    for action in action_set.actions {
        println!("[Action] {:?} on {}", action.action_type, action.path);
        if !is_path_safe(&action.path) {
            eprintln!(
                "❌ Error: Unsafe file path outside current directory: {}",
                action.path
            );
            break;
        }

        match action.action_type {
            ActionType::CreateFile => {
                create_file(&action.path).await?;
            }
            ActionType::WriteFile => {
                if let Some(content) = action.content {
                    write_file(&action.path, &content)?;
                } else {
                    eprintln!(
                        "⚠️  Error: Action {:?} requires non-empty content, but none was provided for path: {}",
                        action.action_type, action.path
                    );
                }
            }
            ActionType::AppendFile => {
                if let Some(content) = action.content {
                    append_file(&action.path, &content).await?;
                } else {
                    eprintln!(
                        "⚠️  Error: Action {:?} requires non-empty content, but none was provided for path: {}",
                        action.action_type, action.path
                    );
                }
            }
            ActionType::ReadFile => {
                read_file(&action.path)?;
            }
            ActionType::DeleteFile => {
                delete_file(&action.path)?;
            }
        }
    }

    Ok(())
}
