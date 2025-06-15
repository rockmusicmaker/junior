use anyhow::{Result, anyhow};
use clap::{Arg, Command};
use confy;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;

#[derive(Default, Debug, Deserialize, Serialize)]
struct Config {
    api_key: String,
    model: String,
    endpoint: String,
    stream: bool,
}
fn load_config() -> Result<Config> {
    let config_path = dirs::home_dir()
        .ok_or(anyhow!("Failed to find home directory"))?
        .join(".junior.toml");
    let config: Config = confy::load_path(config_path)?;
    Ok(config)
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
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

async fn send_prompt(config: &Config, messages: &[ChatMessage]) -> Result<String> {
    let client = Client::new();

    let request_body = ChatRequest {
        model: config.model.clone(),
        messages: messages.to_vec(),
        stream: config.stream,
    };

    let response = client
        .post(&config.endpoint)
        .bearer_auth(&config.api_key)
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

fn write_history_to_file(history: &[ChatMessage]) -> Result<()> {
    let history_path = dirs::home_dir()
        .ok_or(anyhow!("Failed to find home directory"))?
        .join(".junior-history.json");
    let json = serde_json::to_string_pretty(history)?;
    fs::write(history_path, json)?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("junior")
        .version("0.1.0")
        .author("Your Name")
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
    let mut prompt: String = matches.get_one("prompt").cloned().unwrap_or_default();

    let mut additional_context = String::new();

    if let Some(file_path) = matches.get_one::<String>("file") {
        let mut file = File::open(file_path)?;
        file.read_to_string(&mut additional_context)?;
    }

    let mut history: Vec<ChatMessage> = Vec::new();

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

                    history.push(ChatMessage {
                        role: "user".to_string(),
                        content: format!("{}\n{}", input, additional_context),
                    });

                    let response = send_prompt(&config, &history).await?;
                    println!("{}", response);
                    history.push(ChatMessage {
                        role: "assistant".to_string(),
                        content: response,
                    });
                    write_history_to_file(&history)?;
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("\n👋 lol bye.");
                    break;
                }
            }
        }
    } else {
        prompt.push_str(&additional_context);

        history.push(ChatMessage {
            role: "user".to_string(),
            content: prompt.clone(),
        });

        let response = send_prompt(&config, &history).await?;

        println!("{}", response);

        history.push(ChatMessage {
            role: "assistant".to_string(),
            content: response,
        });
    }

    Ok(())
}
