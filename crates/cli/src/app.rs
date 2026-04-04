use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use theasus_core::engine::QueryEngine;
use theasus_language_model::{
    ContentBlock as LlmContentBlock, Message as LlmMessage, ToolDefinition,
};
use uuid::Uuid;

use crate::LlmManager;

pub struct App {
    pub cwd: PathBuf,
    pub session_id: Uuid,
    pub running: bool,
    pub tool_registry: Arc<theasus_tools::ToolRegistry>,
    pub command_registry: Arc<theasus_commands::CommandRegistry>,
    pub llm_manager: crate::LlmManager,
    pub query_engine: QueryEngine,
    #[allow(dead_code)]
    pub max_tool_iterations: usize,
}

fn convert_to_llm_message(msg: &theasus_core::Message) -> LlmMessage {
    use theasus_core::Message as CoreMessage;
    match msg {
        CoreMessage::User(m) => LlmMessage::User(theasus_language_model::UserMessage {
            id: m.id,
            content: m
                .content
                .iter()
                .map(|c| match c {
                    theasus_core::ContentBlock::Text { text } => {
                        LlmContentBlock::Text { text: text.clone() }
                    }
                    theasus_core::ContentBlock::Image { url, detail: _ } => LlmContentBlock::Text {
                        text: format!("[Image: {}]", url),
                    },
                    theasus_core::ContentBlock::ToolUse { tool } => LlmContentBlock::Text {
                        text: format!("[Tool: {}]", tool.name),
                    },
                    theasus_core::ContentBlock::ToolResult {
                        tool_use_id: _,
                        content,
                    } => LlmContentBlock::Text {
                        text: content.clone(),
                    },
                })
                .collect(),
            timestamp: m.timestamp,
        }),
        CoreMessage::Assistant(m) => {
            LlmMessage::Assistant(theasus_language_model::AssistantMessage {
                id: m.id,
                content: m
                    .content
                    .iter()
                    .map(|c| match c {
                        theasus_core::ContentBlock::Text { text } => {
                            LlmContentBlock::Text { text: text.clone() }
                        }
                        theasus_core::ContentBlock::Image { url, detail: _ } => {
                            LlmContentBlock::Text {
                                text: format!("[Image: {}]", url),
                            }
                        }
                        theasus_core::ContentBlock::ToolUse { tool } => LlmContentBlock::Text {
                            text: format!("[Tool: {}]", tool.name),
                        },
                        theasus_core::ContentBlock::ToolResult {
                            tool_use_id: _,
                            content,
                        } => LlmContentBlock::Text {
                            text: content.clone(),
                        },
                    })
                    .collect(),
                tool_calls: vec![],
                usage: theasus_language_model::Usage {
                    input_tokens: m.usage.input_tokens,
                    output_tokens: m.usage.output_tokens,
                    total_tokens: m.usage.total_tokens,
                },
                model: m.model.clone(),
                stop_reason: m.stop_reason.clone(),
                timestamp: m.timestamp,
            })
        }
        CoreMessage::System(m) => LlmMessage::System(theasus_language_model::SystemMessage {
            id: Uuid::new_v4(),
            content: vec![LlmContentBlock::Text {
                text: m.content.clone(),
            }],
            timestamp: Utc::now(),
        }),
        _ => LlmMessage::User(theasus_language_model::UserMessage {
            id: Uuid::new_v4(),
            content: vec![],
            timestamp: Utc::now(),
        }),
    }
}

impl App {
    pub fn new(cwd: PathBuf) -> Self {
        let llm_manager = LlmManager::new();

        let config = theasus_core::engine::QueryEngineConfig {
            model: llm_manager.settings.model.clone(),
            max_tokens: Some(4096),
            temperature: 0.7,
            system_prompt: Some("You are Bodhi, an AI terminal assistant. You have access to tools to help you answer questions. When you need to use a tool, respond with a tool use block.".to_string()),
        };

        let query_engine = theasus_core::QueryEngine::new(config);

        Self {
            cwd: cwd.clone(),
            session_id: Uuid::new_v4(),
            running: true,
            tool_registry: Arc::new(theasus_tools::ToolRegistry::new()),
            command_registry: Arc::new(theasus_commands::CommandRegistry::new()),
            llm_manager,
            query_engine,
            max_tool_iterations: 5,
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();

        if self.tool_registry.get("bash").is_some() {
            tools.push(ToolDefinition {
                name: "bash".to_string(),
                description: "Execute shell commands and return the output".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The command to execute"
                        }
                    },
                    "required": ["command"]
                }),
            });
        }

        if self.tool_registry.get("file_read").is_some() {
            tools.push(ToolDefinition {
                name: "file_read".to_string(),
                description: "Read files and directories".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file or directory path to read"
                        }
                    },
                    "required": ["path"]
                }),
            });
        }

        if self.tool_registry.get("file_write").is_some() {
            tools.push(ToolDefinition {
                name: "file_write".to_string(),
                description: "Create or overwrite files with content".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The file path to write"
                        },
                        "content": {
                            "type": "string",
                            "description": "The content to write"
                        }
                    },
                    "required": ["path", "content"]
                }),
            });
        }

        if self.tool_registry.get("grep").is_some() {
            tools.push(ToolDefinition {
                name: "grep".to_string(),
                description: "Search for patterns in files".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "The directory or file to search in"
                        }
                    },
                    "required": ["pattern"]
                }),
            });
        }

        if self.tool_registry.get("glob").is_some() {
            tools.push(ToolDefinition {
                name: "glob".to_string(),
                description: "Find files matching a glob pattern".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "The glob pattern (e.g., **/*.rs)"
                        },
                        "path": {
                            "type": "string",
                            "description": "The directory to search in"
                        }
                    },
                    "required": ["pattern"]
                }),
            });
        }

        tools
    }

    #[allow(dead_code)]
    fn execute_tool(
        &self,
        name: &str,
        input: serde_json::Value,
        rt: &tokio::runtime::Runtime,
    ) -> String {
        let context = theasus_tools::ToolContext {
            cwd: self.cwd.clone(),
            session_id: self.session_id,
            user_id: None,
        };

        if let Some(tool) = self.tool_registry.get(name) {
            let result = rt.block_on(tool.execute(input, &context));

            match result {
                Ok(result) => {
                    if result.success {
                        result.output
                    } else {
                        result
                            .error
                            .unwrap_or_else(|| "Tool execution failed".to_string())
                    }
                }
                Err(e) => e.to_string(),
            }
        } else {
            format!("Tool '{}' not found", name)
        }
    }

    pub fn run(&mut self) -> Result<()> {
        let rt = tokio::runtime::Runtime::new()?;

        println!("\nWelcome to Bodhi! Type /help for available commands.\n");

        while self.running {
            print!("> ");
            std::io::Write::flush(&mut std::io::stdout())?;

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            if input.starts_with('/') {
                self.handle_command(input, &rt)?;
            } else {
                self.handle_query(input, &rt)?;
            }
        }

        Ok(())
    }

    fn handle_command(&mut self, input: &str, rt: &tokio::runtime::Runtime) -> Result<()> {
        let cmd = self.command_registry.get(input);

        match cmd {
            Some(cmd) => {
                let args = input
                    .trim_start_matches('/')
                    .split_once(' ')
                    .map(|x| x.1)
                    .unwrap_or("");
                let context = theasus_commands::CommandContext {
                    cwd: self.cwd.clone(),
                    session_id: self.session_id,
                };

                let result = rt.block_on(async { cmd.execute(args, &context).await });

                match result {
                    Ok(result) => {
                        if result.success {
                            println!("{}", result.output);
                        } else if let Some(error) = result.error {
                            eprintln!("Error: {}", error);
                        }

                        if input.starts_with("/exit")
                            || input.starts_with("/quit")
                            || input.starts_with("/q")
                        {
                            self.running = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("Command error: {}", e);
                    }
                }
            }
            None => {
                println!("Unknown command: {}", input);
                println!("Type /help for available commands");
            }
        }

        Ok(())
    }

    fn handle_query(&mut self, input: &str, rt: &tokio::runtime::Runtime) -> Result<()> {
        println!("Processing query: {}", input);

        self.query_engine.add_user_message(input);

        if let Some(_client) = &self.llm_manager.client {
            let llm_messages: Vec<LlmMessage> = self
                .query_engine
                .get_messages()
                .iter()
                .map(convert_to_llm_message)
                .collect();

            let tools = self.get_tool_definitions();

            let client = self.llm_manager.client.clone();
            let model = self.llm_manager.settings.model.clone();
            let messages = llm_messages;

            let result = rt.block_on(async move {
                if let Some(c) = client {
                    c.complete(theasus_language_model::CompletionRequest {
                        model,
                        messages,
                        max_tokens: Some(4096),
                        temperature: Some(0.7),
                        system: Some("You are Bodhi, an AI terminal assistant. You have access to tools to help you answer questions. When you need to use a tool, respond with a tool use block.".to_string()),
                        tools: if tools.is_empty() { None } else { Some(tools.clone()) },
                        stream: false,
                    }).await
                } else {
                    Err(anyhow::anyhow!("No client"))
                }
            });

            match result {
                Ok(response) => {
                    let text = match response.message {
                        theasus_language_model::Message::Assistant(msg) => {
                            let mut content = String::new();
                            for block in &msg.content {
                                if let theasus_language_model::ContentBlock::Text { text } = block {
                                    content.push_str(text);
                                }
                            }
                            content
                        }
                        _ => String::new(),
                    };

                    println!("{}", text);
                    self.query_engine.add_assistant_message(&text);
                    println!("\n[Tokens: {}]", response.usage.total_tokens);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                }
            }
        } else {
            println!("LLM not configured. Run: bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o");
        }

        Ok(())
    }
}
