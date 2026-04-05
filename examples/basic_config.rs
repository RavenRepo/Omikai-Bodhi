//! Example: Load and display Bodhi configuration
//!
//! Run with: `cargo run --example basic_config`

use theasus_settings::{PermissionMode, Settings, Theme};

fn main() {
    println!("Bodhi Configuration Example\n");

    // Load existing settings or create defaults
    let settings = Settings::load().unwrap_or_else(|e| {
        println!("Note: Could not load settings ({}), using defaults\n", e);
        Settings::default()
    });

    // Display current configuration
    println!("Current Settings:");
    println!("  Model:           {}", settings.model);
    println!("  LLM Provider:    {}", settings.llm_provider);
    println!(
        "  API Key:         {}",
        if settings.api_key.is_some() { "[configured]" } else { "[not set]" }
    );
    println!("  Theme:           {:?}", settings.theme);
    println!("  Permission Mode: {:?}", settings.permission_mode);

    if let Some(base_url) = &settings.llm_base_url {
        println!("  Base URL:        {}", base_url);
    }

    if let Some(budget) = settings.max_budget_usd {
        println!("  Max Budget:      ${:.2}", budget);
    }

    println!("\nConfiguration path: {:?}", Settings::get_config_path());

    // Show how to build settings programmatically
    println!("\n--- Building Settings Programmatically ---");

    let custom_settings = theasus_settings::SettingsBuilder::new()
        .model("claude-3-sonnet")
        .theme(Theme::Light)
        .permission_mode(PermissionMode::Auto)
        .max_budget_usd(50.0)
        .build();

    println!("Custom Settings:");
    println!("  Model:           {}", custom_settings.model);
    println!("  Theme:           {:?}", custom_settings.theme);
    println!("  Permission Mode: {:?}", custom_settings.permission_mode);
    println!("  Max Budget:      ${:.2}", custom_settings.max_budget_usd.unwrap_or(0.0));
}
