//! Example: Demonstrate tool execution
//!
//! Run with: `cargo run --example tool_usage`

use std::path::PathBuf;
use theasus_tools::{ToolContext, ToolRegistry};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    println!("Bodhi Tool Usage Example\n");

    // Create tool registry with all default tools
    let registry = ToolRegistry::new();

    // List available tools
    println!("Available Tools:");
    for tool in registry.list() {
        println!("  - {} : {}", tool.name, tool.description);
    }
    println!();

    // Create execution context
    let context = ToolContext {
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        session_id: Uuid::new_v4(),
        user_id: None,
    };

    // Example 1: Use glob to find Rust files
    println!("--- Example 1: Glob Tool ---");
    let glob_result = registry
        .execute_with_context(
            "glob",
            serde_json::json!({
                "pattern": "*.toml"
            }),
            &context,
        )
        .await;

    match glob_result {
        Ok(result) => {
            println!("Found files:\n{}", result.output);
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 2: Read a file
    println!("\n--- Example 2: File Read Tool ---");
    let read_result = registry
        .execute_with_context(
            "file_read",
            serde_json::json!({
                "path": "Cargo.toml"
            }),
            &context,
        )
        .await;

    match read_result {
        Ok(result) => {
            if result.success {
                let preview: String = result.output.lines().take(10).collect::<Vec<_>>().join("\n");
                println!("First 10 lines of Cargo.toml:\n{}\n...", preview);
            } else {
                println!("Read failed: {}", result.error.unwrap_or_default());
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 3: Use grep to search
    println!("\n--- Example 3: Grep Tool ---");
    let grep_result = registry
        .execute_with_context(
            "grep",
            serde_json::json!({
                "pattern": "theasus",
                "path": "Cargo.toml"
            }),
            &context,
        )
        .await;

    match grep_result {
        Ok(result) => {
            println!("Grep results:\n{}", result.output);
        }
        Err(e) => println!("Error: {}", e),
    }

    // Show tool schema
    println!("\n--- Tool Schema Example ---");
    if let Some(bash_tool) = registry.get("bash") {
        let def = bash_tool.definition();
        println!("Bash tool schema:");
        println!("{}", serde_json::to_string_pretty(&def.input_schema).unwrap_or_default());
    }
}
