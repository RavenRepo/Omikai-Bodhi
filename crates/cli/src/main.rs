use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

mod app;
mod llm;
pub use llm::LlmManager;

#[derive(Parser)]
#[command(name = "bodhi")]
#[command(about = "Bodhi AI Terminal", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    config: Option<PathBuf>,
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
    Commands,

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

    match cli.command {
        Commands::Run { cwd } => {
            tracing::info!("Starting Bodhi terminal in: {:?}", cwd);
            println!("Bodhi AI Terminal v{}", env!("CARGO_PKG_VERSION"));
            println!("Use /help for available commands, /exit to quit");
            println!("Working directory: {:?}", cwd);

            let mut app = app::App::new(cwd);
            app.run()?;
            Ok(())
        }
        Commands::Query { prompt, stream } => {
            let prompt = prompt.join(" ");
            tracing::info!("Executing query: {}", prompt);

            if stream {
                println!("[streaming mode not yet implemented]");
            }

            let tool_registry = Arc::new(theasus_tools::ToolRegistry::new());
            let tool = tool_registry.get("bash").unwrap();
            let context = theasus_tools::ToolContext {
                cwd: std::path::PathBuf::from("."),
                session_id: Uuid::new_v4(),
                user_id: None,
            };

            let result = tokio::runtime::Runtime::new()?.block_on(async {
                tool.execute(serde_json::json!({ "command": &prompt }), &context)
                    .await
            });

            match result {
                Ok(result) => {
                    println!("{}", result.output);
                    if let Some(error) = result.error {
                        eprintln!("Error: {}", error);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
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
        Commands::ConfigLlm {
            provider,
            api_key,
            model,
            base_url,
        } => {
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
        Commands::Commands => {
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
