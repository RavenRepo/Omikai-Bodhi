use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use theasus_core::engine::QueryEngine;
use theasus_language_model::{Message as LlmMessage, ContentBlock as LlmContentBlock};

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
            content: m.content.iter().map(|c| {
                match c {
                    theasus_core::ContentBlock::Text { text } => LlmContentBlock::Text { text: text.clone() },
                    theasus_core::ContentBlock::Image { url, detail: _ } => LlmContentBlock::Text { text: format!("[Image: {}]", url) },
                    theasus_core::ContentBlock::ToolUse { tool } => LlmContentBlock::Text { text: format!("[Tool: {}]", tool.name) },
                    theasus_core::ContentBlock::ToolResult { tool_use_id: _, content } => LlmContentBlock::Text { text: content.clone() },
                }
            }).collect(),
            timestamp: m.timestamp,
        }),
        CoreMessage::Assistant(m) => LlmMessage::Assistant(theasus_language_model::AssistantMessage {
            id: m.id,
            content: m.content.iter().map(|c| {
                match c {
                    theasus_core::ContentBlock::Text { text } => LlmContentBlock::Text { text: text.clone() },
                    theasus_core::ContentBlock::Image { url, detail: _ } => LlmContentBlock::Text { text: format!("[Image: {}]", url) },
                    theasus_core::ContentBlock::ToolUse { tool } => LlmContentBlock::Text { text: format!("[Tool: {}]", tool.name) },
                    theasus_core::ContentBlock::ToolResult { tool_use_id: _, content } => LlmContentBlock::Text { text: content.clone() },
                }
            }).collect(),
            tool_calls: vec![],
            usage: theasus_language_model::Usage {
                input_tokens: m.usage.input_tokens,
                output_tokens: m.usage.output_tokens,
                total_tokens: m.usage.total_tokens,
            },
            model: m.model.clone(),
            stop_reason: m.stop_reason.clone(),
            timestamp: m.timestamp,
        }),
        CoreMessage::System(m) => LlmMessage::System(theasus_language_model::SystemMessage {
            id: Uuid::new_v4(),
            content: vec![LlmContentBlock::Text { text: m.content.clone() }],
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
                let args = input.trim_start_matches('/').splitn(2, ' ').nth(1).unwrap_or("");
                let context = theasus_commands::CommandContext {
                    cwd: self.cwd.clone(),
                    session_id: self.session_id,
                };
                
                let result = rt.block_on(async {
                    cmd.execute(args, &context).await
                });
                
                match result {
                    Ok(result) => {
                        if result.success {
                            println!("{}", result.output);
                        } else if let Some(error) = result.error {
                            eprintln!("Error: {}", error);
                        }
                        
                        if input.starts_with("/exit") || input.starts_with("/quit") || input.starts_with("/q") {
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
            let llm_messages: Vec<LlmMessage> = self.query_engine.get_messages()
                .iter()
                .map(convert_to_llm_message)
                .collect();
            
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
                        system: Some("You are Bodhi, an AI terminal assistant.".to_string()),
                        tools: None,
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
                            msg.content.iter()
                                .filter_map(|block| {
                                    if let theasus_language_model::ContentBlock::Text { text } = block {
                                        Some(text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
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
