use crate::{Command, CommandContext, CommandResult};
use async_trait::async_trait;
use std::fmt;
use std::process::Stdio;
use tokio::process::Command as ProcessCommand;

async fn run_git_command(args: &[&str], cwd: &std::path::Path) -> Result<String, String> {
    let output = ProcessCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Failed to execute git: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// /commit command
pub struct CommitCommand;

impl CommitCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for CommitCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommitCommand").finish()
    }
}

impl Clone for CommitCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for CommitCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for CommitCommand {
    fn name(&self) -> &str {
        "commit"
    }

    fn description(&self) -> &str {
        "Stage all changes and commit with a message"
    }

    fn args_description(&self) -> Option<&str> {
        Some("[message] - If no message, AI will generate one")
    }

    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let message = args.trim();

        // If no message provided, get diff for AI to generate one
        if message.is_empty() {
            let diff = run_git_command(&["--no-pager", "diff"], &context.cwd).await;
            let staged_diff =
                run_git_command(&["--no-pager", "diff", "--staged"], &context.cwd).await;

            let combined_diff = match (diff, staged_diff) {
                (Ok(d), Ok(s)) => format!("{}\n{}", d, s),
                (Ok(d), Err(_)) => d,
                (Err(_), Ok(s)) => s,
                (Err(e), Err(_)) => return Ok(CommandResult::error(format!("No changes to commit or git error: {}", e))),
            };

            if combined_diff.trim().is_empty() {
                return Ok(CommandResult::error("No changes to commit"));
            }

            return Ok(CommandResult::success(format!(
                "No commit message provided. Here's the diff for AI to generate a message:\n\n```diff\n{}\n```\n\nPlease provide a commit message or ask AI to generate one.",
                combined_diff
            )));
        }

        // Stage all changes
        if let Err(e) = run_git_command(&["add", "-A"], &context.cwd).await {
            return Ok(CommandResult::error(format!("Failed to stage changes: {}", e)));
        }

        // Commit with message
        match run_git_command(&["commit", "-m", message], &context.cwd).await {
            Ok(output) => Ok(CommandResult::success(format!("Committed:\n{}", output))),
            Err(e) => Ok(CommandResult::error(format!("Commit failed: {}", e))),
        }
    }
}

// /diff command
pub struct DiffCommand;

impl DiffCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for DiffCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiffCommand").finish()
    }
}

impl Clone for DiffCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for DiffCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for DiffCommand {
    fn name(&self) -> &str {
        "diff"
    }

    fn description(&self) -> &str {
        "Show git diff of changes"
    }

    fn args_description(&self) -> Option<&str> {
        Some("[file] - Optional file path to diff")
    }

    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let file_path = args.trim();

        let git_args = if file_path.is_empty() {
            vec!["--no-pager", "diff"]
        } else {
            vec!["--no-pager", "diff", "--", file_path]
        };

        match run_git_command(&git_args, &context.cwd).await {
            Ok(output) => {
                if output.trim().is_empty() {
                    Ok(CommandResult::success("No changes detected"))
                } else {
                    Ok(CommandResult::success(format!("```diff\n{}\n```", output)))
                }
            }
            Err(e) => Ok(CommandResult::error(format!("Git diff failed: {}", e))),
        }
    }
}

// /review command
pub struct ReviewCommand;

impl ReviewCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for ReviewCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReviewCommand").finish()
    }
}

impl Clone for ReviewCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for ReviewCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for ReviewCommand {
    fn name(&self) -> &str {
        "review"
    }

    fn description(&self) -> &str {
        "Get AI code review of current changes"
    }

    async fn execute(
        &self,
        _args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        // Try staged changes first
        let staged = run_git_command(&["--no-pager", "diff", "--staged"], &context.cwd).await;
        let unstaged = run_git_command(&["--no-pager", "diff"], &context.cwd).await;

        let diff = match (&staged, &unstaged) {
            (Ok(s), _) if !s.trim().is_empty() => s.clone(),
            (_, Ok(u)) if !u.trim().is_empty() => u.clone(),
            _ => {
                return Ok(CommandResult::error("No changes to review"));
            }
        };

        // Get file stats
        let stats = run_git_command(&["--no-pager", "diff", "--stat"], &context.cwd)
            .await
            .unwrap_or_default();

        Ok(CommandResult::success(format!(
            "## Code Review Request\n\n**Changed Files:**\n```\n{}\n```\n\n**Diff:**\n```diff\n{}\n```\n\nPlease review these changes for:\n- Bugs or logic errors\n- Security issues\n- Performance concerns\n- Code style and best practices",
            stats, diff
        )))
    }
}

// /branch command
pub struct BranchCommand;

impl BranchCommand {
    pub fn new() -> Self {
        Self
    }
}

impl fmt::Debug for BranchCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BranchCommand").finish()
    }
}

impl Clone for BranchCommand {
    fn clone(&self) -> Self {
        Self
    }
}

impl Default for BranchCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Command for BranchCommand {
    fn name(&self) -> &str {
        "branch"
    }

    fn aliases(&self) -> &[&str] {
        &["br"]
    }

    fn description(&self) -> &str {
        "List branches or checkout/create a branch"
    }

    fn args_description(&self) -> Option<&str> {
        Some("[name] - Branch name to checkout or create")
    }

    async fn execute(
        &self,
        args: &str,
        context: &CommandContext,
    ) -> theasus_core::Result<CommandResult> {
        let branch_name = args.trim();

        if branch_name.is_empty() {
            // List branches
            match run_git_command(&["--no-pager", "branch", "-a", "-v"], &context.cwd).await {
                Ok(output) => Ok(CommandResult::success(format!("Branches:\n{}", output))),
                Err(e) => Ok(CommandResult::error(format!("Failed to list branches: {}", e))),
            }
        } else {
            // Try to checkout existing branch first
            match run_git_command(&["checkout", branch_name], &context.cwd).await {
                Ok(output) => Ok(CommandResult::success(format!(
                    "Switched to branch '{}':\n{}",
                    branch_name, output
                ))),
                Err(_) => {
                    // Branch doesn't exist, create it
                    match run_git_command(&["checkout", "-b", branch_name], &context.cwd).await {
                        Ok(output) => Ok(CommandResult::success(format!(
                            "Created and switched to new branch '{}':\n{}",
                            branch_name, output
                        ))),
                        Err(e) => Ok(CommandResult::error(format!(
                            "Failed to create branch '{}': {}",
                            branch_name, e
                        ))),
                    }
                }
            }
        }
    }
}
