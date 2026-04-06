//! Integration tests for the commands crate.

use std::path::PathBuf;
use theasus_commands::{
    ClearCommand, Command, CommandContext, CommandRegistry, HelpCommand, SessionCommand,
    SessionsCommand,
};
use uuid::Uuid;

fn test_context() -> CommandContext {
    CommandContext { cwd: PathBuf::from("/tmp"), session_id: Uuid::new_v4() }
}

mod registry {
    use super::*;

    #[test]
    fn test_registry_has_builtin_commands() {
        let registry = CommandRegistry::new();
        let names = registry.list_names();

        assert!(names.contains(&"help".to_string()));
        assert!(names.contains(&"clear".to_string()));
        assert!(names.contains(&"exit".to_string()));
        assert!(names.contains(&"status".to_string()));
        assert!(names.contains(&"model".to_string()));
    }

    #[test]
    fn test_registry_has_session_commands() {
        let registry = CommandRegistry::new();
        let names = registry.list_names();

        assert!(names.contains(&"sessions".to_string()));
        assert!(names.contains(&"session".to_string()));
    }

    #[test]
    fn test_registry_has_git_commands() {
        let registry = CommandRegistry::new();
        let names = registry.list_names();

        assert!(names.contains(&"commit".to_string()));
        assert!(names.contains(&"diff".to_string()));
        assert!(names.contains(&"branch".to_string()));
    }

    #[test]
    fn test_aliases_work() {
        let registry = CommandRegistry::new();

        // /h should resolve to help
        let help_alias = registry.get("h");
        assert!(help_alias.is_some());
        assert_eq!(help_alias.unwrap().name(), "help");

        // /q should resolve to exit
        let quit_alias = registry.get("q");
        assert!(quit_alias.is_some());
        assert_eq!(quit_alias.unwrap().name(), "exit");
    }
}

mod help_command {
    use super::*;

    #[tokio::test]
    async fn test_help_returns_success() {
        let cmd = HelpCommand::new();
        let ctx = test_context();

        let result = cmd.execute("", &ctx).await.unwrap();

        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_help_contains_command_list() {
        let cmd = HelpCommand::new();
        let ctx = test_context();

        let result = cmd.execute("", &ctx).await.unwrap();

        assert!(result.output.contains("help") || result.output.contains("Available"));
    }
}

mod clear_command {
    use super::*;

    #[tokio::test]
    async fn test_clear_returns_success() {
        let cmd = ClearCommand::new();
        let ctx = test_context();

        let result = cmd.execute("", &ctx).await.unwrap();

        assert!(result.success);
    }
}

mod session_commands {
    use super::*;

    #[tokio::test]
    async fn test_sessions_command() {
        let cmd = SessionsCommand::new();
        let ctx = test_context();

        // Should work regardless of whether sessions exist
        let result = cmd.execute("", &ctx).await.unwrap();

        // Either succeeds with list or with "no sessions" message
        assert!(result.success || result.error.is_some());
    }

    #[tokio::test]
    async fn test_session_command_help() {
        let cmd = SessionCommand::new();
        let ctx = test_context();

        // No args should show help
        let result = cmd.execute("", &ctx).await.unwrap();

        assert!(!result.success);
        assert!(result.error.is_some());
        assert!(result.error.unwrap().contains("Usage"));
    }

    #[tokio::test]
    async fn test_session_command_unknown_subcommand() {
        let cmd = SessionCommand::new();
        let ctx = test_context();

        let result = cmd.execute("unknown", &ctx).await.unwrap();

        assert!(!result.success);
        assert!(result.error.unwrap().contains("Unknown"));
    }
}

mod command_parsing {
    use super::*;

    #[test]
    fn test_strip_slash_prefix() {
        let registry = CommandRegistry::new();

        // Should work with or without slash
        let with_slash = registry.get("/help");
        let without_slash = registry.get("help");

        assert!(with_slash.is_some());
        assert!(without_slash.is_some());
        assert_eq!(with_slash.unwrap().name(), without_slash.unwrap().name());
    }
}
