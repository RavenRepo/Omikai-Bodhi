//! Example: Basic LLM query
//!
//! Run with: `cargo run --example simple_query`
//!
//! Note: This example requires LLM configuration. Run:
//! `bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o`

use theasus_core::{new_theasus, Config, QueryEngine};
use theasus_core::engine::QueryEngineConfig;
use theasus_settings::Settings;

#[tokio::main]
async fn main() {
    println!("Bodhi Simple Query Example\n");

    // Load settings to check if LLM is configured
    let settings = Settings::load().unwrap_or_default();
    
    println!("Current LLM Configuration:");
    println!("  Provider: {}", settings.llm_provider);
    println!("  Model:    {}", settings.model);
    println!("  API Key:  {}", if settings.api_key.is_some() { "[configured]" } else { "[not set]" });
    println!();

    // Create Theasus instance
    let config = Config {
        api_key: settings.api_key.clone(),
        model: settings.model.clone(),
        ..Default::default()
    };

    let _theasus = new_theasus(config).await.expect("Failed to create Theasus instance");
    println!("Theasus instance created successfully!");

    // Create query engine with configuration
    let engine_config = QueryEngineConfig {
        model: settings.model.clone(),
        max_tokens: Some(2048),
        temperature: 0.7,
        system_prompt: Some("You are Bodhi, an AI terminal assistant.".to_string()),
        max_tool_calls: 10,
        max_iterations: 5,
    };

    let mut engine = QueryEngine::new(engine_config);
    println!("Query engine created with model: {}", settings.model);

    // Add a sample message
    engine.add_user_message("What is 2 + 2?");
    println!("\nAdded user message to conversation.");
    
    let messages = engine.get_messages();
    println!("Current conversation has {} message(s).", messages.len());

    // Display conversation state
    println!("\nConversation State:");
    for (i, msg) in messages.iter().enumerate() {
        match msg {
            theasus_core::Message::User(m) => {
                println!("  [{}] User: {:?}", i, m.content);
            }
            theasus_core::Message::Assistant(m) => {
                println!("  [{}] Assistant: {:?}", i, m.content);
            }
            theasus_core::Message::System(m) => {
                println!("  [{}] System: {:?}", i, m.content);
            }
            theasus_core::Message::Progress(m) => {
                println!("  [{}] Progress: {}", i, m.message);
            }
            theasus_core::Message::Attachment(m) => {
                println!("  [{}] Attachment: {} bytes", i, m.content.len());
            }
        }
    }

    if settings.api_key.is_none() {
        println!("\n⚠️  Note: LLM not configured. To make actual queries, run:");
        println!("   bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o");
    } else {
        println!("\n✓ LLM is configured. You can use `bodhi query \"your question\"` to make queries.");
    }
}
