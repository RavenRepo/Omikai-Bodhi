use anyhow::Result;
use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

mod app;
mod completions;
mod convert;
mod llm;
pub use llm::LlmManager;

use convert::convert_core_to_llm;

#[derive(Parser)]
#[command(name = "bodhi")]
#[command(about = "Bodhi AI Terminal", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Generate shell completion scripts
    #[arg(long, value_enum)]
    completions: Option<Shell>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Start interactive terminal session")]
    Run {
        #[arg(short, long, default_value = ".")]
        cwd: PathBuf,
    },

    #[command(about = "Execute a single query")]
    Query {
        #[arg(required = true)]
        prompt: Vec<String>,

        #[arg(short, long)]
        stream: bool,
    },

    #[command(about = "Show configuration")]
    Config {
        #[arg(short, long, default_value = "false")]
        edit: bool,
    },

    #[command(about = "Show version")]
    Version,

    #[command(about = "List available tools")]
    Tools,

    #[command(about = "List available agents")]
    Agents,

    #[command(about = "List available commands")]
    List,

    #[command(about = "Show session info")]
    Status,

    #[command(about = "Configure LLM provider")]
    ConfigLlm {
        #[arg(short, long)]
        provider: Option<String>,

        #[arg(short, long)]
        api_key: Option<String>,

        #[arg(short, long)]
        model: Option<String>,

        #[arg(short, long)]
        base_url: Option<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    // Handle completions generation
    if let Some(shell) = cli.completions {
        completions::generate_completions(shell);
        return Ok(());
    }

    // Require a command if not generating completions
    let command = cli.command.unwrap_or_else(|| Commands::Run { cwd: PathBuf::from(".") });

    match command {
        Commands::Run { cwd } => {
            tracing::info!("Starting Bodhi terminal in: {:?}", cwd);
            println!("Bodhi AI Terminal v{}", env!("CARGO_PKG_VERSION"));
            println!("Use /help for available commands, /exit to quit");
            println!("Working directory: {:?}", cwd);

            let mut app = app::App::new(cwd);
            app.run()?;
            Ok(())
        }
        Commands::Query { prompt, stream: _ } => {
            let prompt = prompt.join(" ");
            tracing::info!("Executing query: {}", prompt);

            let settings = theasus_settings::Settings::load().unwrap_or_default();

            if settings.api_key.is_none() {
                eprintln!("Error: LLM not configured. Run: bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o");
                return Ok(());
            }

            let llm_manager = LlmManager::new();

            if !llm_manager.is_configured() {
                eprintln!("Error: LLM not configured. Run: bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o");
                return Ok(());
            }

            let tool_registry = Arc::new(theasus_tools::ToolRegistry::new());

            let config = theasus_core::engine::QueryEngineConfig {
                model: settings.model.clone(),
                max_tokens: Some(4096),
                temperature: 0.7,
                system_prompt: Some("You are Bodhi, an AI terminal assistant.".to_string()),
                max_tool_calls: 10,
                max_iterations: 10,
            };

            let mut query_engine = theasus_core::QueryEngine::new(config);
            let session_id = Uuid::new_v4();
            let cwd = std::env::current_dir().unwrap_or_default();

            query_engine.add_user_message(&prompt);

            let rt = tokio::runtime::Runtime::new()?;

            if let Some(client) = &llm_manager.client {
                let tools = tool_registry.to_llm_tools();
                let mut iterations = 0;

                loop {
                    iterations += 1;
                    if iterations > 10 {
                        println!("\n[Max iterations reached]");
                        break;
                    }

                    let llm_messages: Vec<theasus_language_model::Message> =
                        query_engine.get_messages().iter().map(convert_core_to_llm).collect();

                    let result = rt.block_on(async {
                        client.complete(theasus_language_model::CompletionRequest {
                            model: settings.model.clone(),
                            messages: llm_messages,
                            max_tokens: Some(4096),
                            temperature: Some(0.7),
                            system: Some("You are Bodhi, an AI terminal assistant. You have access to tools to help with user queries.".to_string()),
                            tools: if tools.is_empty() { None } else { Some(tools.clone()) },
                            stream: false,
                        }).await
                    });

                    match result {
                        Ok(response) => {
                            let message = response.message;

                            match message {
                                theasus_language_model::Message::Assistant(assistant_msg) => {
                                    let text: String = assistant_msg
                                        .content
                                        .iter()
                                        .filter_map(|block| {
                                            if let theasus_language_model::ContentBlock::Text {
                                                text,
                                            } = block
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

                                    query_engine.add_assistant_message(&text);

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

                                            if let Some(tool) = tool_registry.get(tool_name) {
                                                let context = theasus_tools::ToolContext {
                                                    cwd: cwd.clone(),
                                                    session_id,
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
                                                        query_engine
                                                            .add_tool_result(tool_id, &result_text);
                                                    }
                                                    Err(e) => {
                                                        let error_text =
                                                            format!("Tool execution error: {}", e);
                                                        println!("  ← Error: {}", error_text);
                                                        query_engine
                                                            .add_tool_result(tool_id, &error_text);
                                                    }
                                                }
                                            } else {
                                                let error_text =
                                                    format!("Tool not found: {}", tool_name);
                                                println!("  ← Error: {}", error_text);
                                                query_engine.add_tool_result(tool_id, &error_text);
                                            }
                                        }
                                    } else {
                                        println!("\n[Tokens: {}]", response.usage.total_tokens);
                                        break;
                                    }
                                }
                                _ => {
                                    println!("[No assistant response]");
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
            }

            Ok(())
        }
        Commands::Config { edit } => {
            if edit {
                println!("Config editor not yet implemented");
            } else {
                println!("Configuration:");
                if let Ok(settings) = theasus_settings::Settings::load() {
                    println!("  model: {}", settings.model);
                    println!("  theme: {:?}", settings.theme);
                    println!("  permission_mode: {:?}", settings.permission_mode);
                }
            }
            Ok(())
        }
        Commands::ConfigLlm { provider, api_key, model, base_url } => {
            if provider.is_none() && api_key.is_none() && model.is_none() && base_url.is_none() {
                println!("LLM Configuration:");
                println!("  Use --provider to set provider (openai, anthropic, ollama, custom)");
                println!("  Use --api-key to set API key");
                println!("  Use --model to set model name");
                println!("  Use --base-url to set custom endpoint");
                if let Ok(settings) = theasus_settings::Settings::load() {
                    println!("  Current provider: {}", settings.llm_provider);
                    println!("  Current model: {}", settings.model);
                }
                return Ok(());
            }

            let mut settings = theasus_settings::Settings::load().unwrap_or_default();

            if let Some(provider) = provider {
                settings.llm_provider = provider;
            }
            if let Some(api_key) = api_key {
                settings.api_key = Some(api_key);
            }
            if let Some(model) = model {
                settings.model = model;
            }
            if let Some(base_url) = base_url {
                settings.llm_base_url = Some(base_url);
            }

            settings.save()?;
            println!("LLM configuration updated successfully!");
            Ok(())
        }
        Commands::Version => {
            println!("Bodhi {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Commands::Tools => {
            let registry = theasus_tools::ToolRegistry::new();
            println!("Available tools:");
            for tool in registry.list() {
                println!("  {} - {}", tool.name, tool.description);
            }
            Ok(())
        }
        Commands::Agents => {
            let registry = theasus_agents::AgentRegistry::new();
            println!("Available agents:");
            for agent in registry.list() {
                println!("  {} - {}", agent.name, agent.description);
            }
            Ok(())
        }
        Commands::List => {
            let registry = theasus_commands::CommandRegistry::new();
            println!("Available commands:");
            for (name, desc) in registry.list() {
                println!("  /{} - {}", name, desc);
            }
            Ok(())
        }
        Commands::Status => {
            println!("Session ID: {}", Uuid::new_v4());
            println!("Working directory: .");
            if let Ok(settings) = theasus_settings::Settings::load() {
                println!("LLM Provider: {}", settings.llm_provider);
                println!("Model: {}", settings.model);
            }
            Ok(())
        }
    }
}
