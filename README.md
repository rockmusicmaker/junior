# Junior

A CLI interface for Large Language Models (LLMs) that can understand prompts and automatically perform file operations based on the AI's responses.

## Features

- **LLM Integration**: Send prompts to any OpenAI-compatible API endpoint
- **File Operations**: AI can create, read, write, append, and delete files
- **Session History**: Automatically saves conversation history with timestamps
- **File Context**: Include file contents in your prompts for AI analysis
- **Safety Controls**: Restricts file operations to the current directory
- **Structured Actions**: AI responses are parsed for specific file actions

## Installation

### Prerequisites

- Rust (latest stable version)
- Cargo

### Building from Source

```bash
cargo build --release
```

The binary will be available at `target/release/junior`.

## Configuration

Junior uses a TOML configuration file located at `~/.junior.toml`:

```toml
api_key = "your-api-key-here"
model = "gpt-4"
endpoint = "https://api.openai.com/v1/chat/completions"
history_directory_path = "~/junior-history"
```

### Configuration Fields

- `api_key`: Your API key for the LLM service
- `model`: The model name to use (e.g., "gpt-4", "gpt-3.5-turbo")
- `endpoint`: The API endpoint URL
- `history_directory_path`: Directory where conversation histories are saved

## Usage

### Basic Usage

```bash
junior "Create a simple Python hello world script"
```

### Include File Context

```bash
junior "Analyze this code and suggest improvements" --file main.rs
```

### Short Form

```bash
junior "Fix the bug in this function" -f src/lib.rs
```

## How It Works

1. **Prompt Processing**: Junior sends your prompt to the configured LLM
2. **AI Response**: The AI analyzes your request and responds with both an explanation and structured actions
3. **Action Parsing**: Junior parses the AI response for JSON-formatted file operations
4. **Safe Execution**: Actions are validated and executed within the current directory only

## AI Response Format

The AI is expected to respond with explanations and actions in this JSON format:

```json
{
  "explanation": "I'll create a Python hello world script for you.",
  "actions": [
    {
      "action_type": "write_file",
      "path": "hello.py",
      "content": "print('Hello, World!')\n"
    }
  ]
}
```

### Supported Action Types

- `create_file`: Create an empty file
- `write_file`: Write content to a file (overwrites existing)
- `append_file`: Append content to a file
- `read_file`: Read file contents
- `delete_file`: Delete a file

## Session History

Each run creates a session file in your configured history directory with the format:
```
session-<timestamp>.json
```

These files contain the complete conversation history including:
- System prompts
- User messages
- AI responses

## Safety Features

- **Path Restriction**: All file operations are restricted to the current working directory
- **Path Validation**: Prevents directory traversal attacks (e.g., `../../../etc/passwd`)
- **Error Handling**: Graceful error handling for network issues, file operations, and JSON parsing

## Examples

### Create a New Project Structure

```bash
junior "Set up a basic Node.js project with package.json, index.js, and README.md"
```

### Code Review and Fixes

```bash
junior "Review this code for bugs and fix them" --file buggy-script.py
```

### Documentation Generation

```bash
junior "Generate comprehensive documentation for this module" --file src/main.rs
```

## Dependencies

- `anyhow`: Error handling
- `clap`: Command-line argument parsing
- `confy`: Configuration file management
- `regex`: Regular expression support
- `reqwest`: HTTP client for API calls
- `serde`: Serialization/deserialization
- `shellexpand`: Shell-style path expansion
- `tokio`: Async runtime
