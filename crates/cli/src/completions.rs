use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::Cli;

/// Generate shell completion scripts for the Bodhi CLI.
///
/// Outputs completion script to stdout for the specified shell.
pub fn generate_completions(shell: Shell) {
    let mut cmd = Cli::command();
    generate(shell, &mut cmd, "bodhi", &mut io::stdout());
}
