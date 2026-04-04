use anyhow::Result;
use chrono::Utc;
use std::path::PathBuf;
use std::sync::Arc;
use theasus_core::engine::QueryEngine;
use theasus_language_model::{ContentBlock as LlmContentBlock, Message as LlmMessage};
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
            system_prompt: Some("You are Bodhi, an AI terminal assistant.".to_string()),
            max_tool_calls: 10,
            max_iterations: 10,
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
                    .splitn(2, ' ')
                    .nth(1)
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

        if let Some(client) = &self.llm_manager.client {
            let tools = self.tool_registry.to_llm_tools();
            let mut iterations = 0;
            let max_iterations = self.query_engine.config.max_iterations;

            loop {
                iterations += 1;
                if iterations > max_iterations {
                    println!("\n[Max iterations reached]");
                    break;
                }

                let llm_messages: Vec<LlmMessage> = self
                    .query_engine
                    .get_messages()
                    .iter()
                    .map(convert_to_llm_message)
                    .collect();

                let client = self.llm_manager.client.clone();
                let model = self.llm_manager.settings.model.clone();
                let messages = llm_messages;
                let tools_to_use = if tools.is_empty() {
                    None
                } else {
                    Some(tools.clone())
                };

                let result = rt.block_on(async move {
                    if let Some(c) = client {
                        c.complete(theasus_language_model::CompletionRequest {
                            model,
                            messages,
                            max_tokens: Some(4096),
                            temperature: Some(0.7),
                            system: Some("You are Bodhi, an AI terminal assistant. You have access to tools to help with user queries.".to_string()),
                            tools: tools_to_use,
                            stream: false,
                        })
                        .await
                    } else {
                        Err(anyhow::anyhow!("No client"))
                    }
                });

                match result {
                    Ok(response) => {
                        let message = response.message;
                        let usage = response.usage;

                        match message {
                            theasus_language_model::Message::Assistant(assistant_msg) => {
                                let text: String = assistant_msg
                                    .content
                                    .iter()
                                    .filter_map(|block| {
                                        if let theasus_language_model::ContentBlock::Text { text } =
                                            block
                                        {
                                            Some(text.clone())
                                        } else {
                                            None
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n");

                                if !text.is_empty() {
                                    println!("{}", text);
                                }

                                self.query_engine.add_assistant_message(&text);

                                if !assistant_msg.tool_calls.is_empty() {
                                    println!(
                                        "\n[Executing {} tool(s)...]",
                                        assistant_msg.tool_calls.len()
                                    );

                                    for tool_call in &assistant_msg.tool_calls {
                                        let tool_name = &tool_call.name;
                                        let tool_input = tool_call.input.clone();
                                        let tool_id = &tool_call.id;

                                        println!("  → {}: {:?}", tool_name, tool_input);

                                        if let Some(tool) = self.tool_registry.get(tool_name) {
                                            let context = theasus_tools::ToolContext {
                                                cwd: self.cwd.clone(),
                                                session_id: self.session_id,
                                                user_id: None,
                                            };

                                            let tool_result = rt.block_on(async {
                                                tool.execute(tool_input, &context).await
                                            });

                                            match tool_result {
                                                Ok(result) => {
                                                    let result_text = if result.success {
                                                        result.output
                                                    } else {
                                                        format!(
                                                            "Error: {}",
                                                            result.error.unwrap_or_default()
                                                        )
                                                    };

                                                    println!(
                                                        "  ← {}",
                                                        result_text
                                                            .chars()
                                                            .take(100)
                                                            .collect::<String>()
                                                    );

                                                    self.query_engine
                                                        .add_tool_result(tool_id, &result_text);
                                                }
                                                Err(e) => {
                                                    let error_text =
                                                        format!("Tool execution error: {}", e);
                                                    println!("  ← Error: {}", error_text);
                                                    self.query_engine
                                                        .add_tool_result(tool_id, &error_text);
                                                }
                                            }
                                        } else {
                                            let error_text =
                                                format!("Tool not found: {}", tool_name);
                                            println!("  ← Error: {}", error_text);
                                            self.query_engine.add_tool_result(tool_id, &error_text);
                                        }
                                    }
                                } else {
                                    println!("\n[Tokens: {}]", usage.total_tokens);
                                    break;
                                }
                            }
                            _ => {
                                println!("\n[No assistant response]");
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break;
                    }
                }
            }
        } else {
            println!("LLM not configured. Run: bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o");
        }

        Ok(())
    }
}
