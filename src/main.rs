use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use confy;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use shellexpand;
use std::fs::{self, File};
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
    model: &String,
    endpoint: &String,
    api_key: &String,
) -> Result<String> {
    let client = Client::new();

    let request_body = ChatRequest {
        model: model.clone(),
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
                .required(false),
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

    let prompt: String = matches.get_one("prompt").cloned().unwrap_or_default();
    let system_prompt = include_str!("system_prompt.md").to_string();

    let mut additional_context: Option<String> = None;
    if let Some(file_path) = matches.get_one::<String>("file") {
        let mut contents = String::new();
        File::open(file_path)?.read_to_string(&mut contents)?;
        additional_context = Some(contents);
    }
    let mut history = initialize_history(system_prompt, additional_context).unwrap();

    if prompt.is_empty() {
        let mut line_editor = Reedline::create();
        let prompt = DefaultPrompt::new(
            DefaultPromptSegment::Basic("junior".to_string()),
            DefaultPromptSegment::Basic("junior".to_string()),
        );
        loop {
            match line_editor.read_line(&prompt)? {
                Signal::Success(input) => {
                    if input.trim() == "kbye" {
                        println!("\n👋 lol bye.");
                        break;
                    }
                    let response =
                        send_message(format!("{}\n", input), &mut history, &config).await?;
                    println!("{}", response);
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("\n👋 lol bye.");
                    break;
                }
            }
        }
    } else {
        let response = send_message(prompt.clone(), &mut history, &config).await?;
        println!("{}", response);
    }

    Ok(())
}
