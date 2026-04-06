//! Integration tests for the tools crate.

use std::path::PathBuf;
use theasus_tools::{
    BashTool, FileReadTool, FileWriteTool, GlobTool, GrepTool, Tool, ToolCall, ToolContext,
    ToolRegistry,
};
use uuid::Uuid;

fn test_context() -> ToolContext {
    ToolContext {
        cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        session_id: Uuid::new_v4(),
        user_id: None,
    }
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

mod file_operations {
    use super::*;

    #[tokio::test]
    async fn test_file_write_and_read() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        // Write file
        let write_tool = FileWriteTool::new();
        let write_result = write_tool
            .execute(
                serde_json::json!({
                    "path": file_path.to_str().unwrap(),
                    "content": "Hello, World!"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(write_result.success, "Write should succeed");

        // Read file
        let read_tool = FileReadTool::new();
        let read_result = read_tool
            .execute(
                serde_json::json!({
                    "path": file_path.to_str().unwrap()
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(read_result.success, "Read should succeed");
        assert!(read_result.output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_file_read_nonexistent() {
        let context = test_context();
        let read_tool = FileReadTool::new();

        let result = read_tool
            .execute(
                serde_json::json!({
                    "path": "/nonexistent/path/file.txt"
                }),
                &context,
            )
            .await;

        // Tool may return error or unsuccessful result for nonexistent file
        if let Ok(r) = result {
            assert!(!r.success, "Should fail for nonexistent file");
        }
        // Error is also acceptable
    }
}

mod glob_tool {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_glob_finds_files() {
        let dir = temp_dir();

        // Create some test files
        fs::write(dir.path().join("test1.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("test2.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("other.txt"), "text").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let glob_tool = GlobTool::new();
        let result = glob_tool
            .execute(
                serde_json::json!({
                    "pattern": "*.rs"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("test1.rs"));
        assert!(result.output.contains("test2.rs"));
        assert!(!result.output.contains("other.txt"));
    }

    #[tokio::test]
    async fn test_glob_recursive() {
        let dir = temp_dir();
        let sub_dir = dir.path().join("subdir");
        fs::create_dir(&sub_dir).unwrap();

        fs::write(dir.path().join("root.rs"), "").unwrap();
        fs::write(sub_dir.join("nested.rs"), "").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let glob_tool = GlobTool::new();
        let result = glob_tool
            .execute(
                serde_json::json!({
                    "pattern": "**/*.rs"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("root.rs"));
        assert!(result.output.contains("nested.rs"));
    }
}

mod grep_tool {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_grep_finds_pattern() {
        let dir = temp_dir();
        fs::write(dir.path().join("test.rs"), "fn hello() {}\nfn goodbye() {}").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let grep_tool = GrepTool::new();
        let result = grep_tool
            .execute(
                serde_json::json!({
                    "pattern": "hello",
                    "path": dir.path().to_str().unwrap()
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn test_grep_no_match() {
        let dir = temp_dir();
        fs::write(dir.path().join("test.txt"), "foo bar baz").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let grep_tool = GrepTool::new();
        let result = grep_tool
            .execute(
                serde_json::json!({
                    "pattern": "nonexistent",
                    "path": dir.path().to_str().unwrap()
                }),
                &context,
            )
            .await;

        // Grep with no matches may succeed with empty output or fail
        if let Ok(r) = result {
            assert!(r.output.is_empty() || !r.success);
        }
        // No match error is also acceptable
    }
}

mod bash_tool {
    use super::*;

    #[tokio::test]
    async fn test_bash_echo() {
        let context = test_context();
        let bash_tool = BashTool::new();

        let result = bash_tool
            .execute(
                serde_json::json!({
                    "command": "echo 'Hello from bash'"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Hello from bash"));
    }

    #[tokio::test]
    async fn test_bash_pwd() {
        let dir = temp_dir();
        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let bash_tool = BashTool::new();
        let result = bash_tool
            .execute(
                serde_json::json!({
                    "command": "pwd"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_bash_failing_command() {
        let context = test_context();
        let bash_tool = BashTool::new();

        let result = bash_tool
            .execute(
                serde_json::json!({
                    "command": "exit 1"
                }),
                &context,
            )
            .await
            .unwrap();

        assert!(!result.success);
    }
}

mod registry {
    use super::*;

    #[test]
    fn test_registry_has_all_tools() {
        let registry = ToolRegistry::new();
        let tools = registry.list();

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

        assert!(tool_names.contains(&"bash"));
        assert!(tool_names.contains(&"file_read"));
        assert!(tool_names.contains(&"file_write"));
        assert!(tool_names.contains(&"glob"));
        assert!(tool_names.contains(&"grep"));
        assert!(tool_names.contains(&"web_fetch"));
    }

    #[tokio::test]
    async fn test_registry_execute() {
        let registry = ToolRegistry::new();
        let result =
            registry.execute("bash", serde_json::json!({"command": "echo test"})).await.unwrap();

        assert!(result.success);
    }

    #[tokio::test]
    async fn test_registry_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("unknown_tool", serde_json::json!({})).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let registry = ToolRegistry::new();
        let context = test_context();

        let calls = vec![
            ToolCall::new("bash", serde_json::json!({"command": "echo one"})),
            ToolCall::new("bash", serde_json::json!({"command": "echo two"})),
            ToolCall::new("bash", serde_json::json!({"command": "echo three"})),
        ];

        let results = registry.execute_parallel(calls, &context).await;

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.result.success);
        }
    }
}

mod caching {
    use super::*;
    use std::fs;

    #[tokio::test]
    async fn test_cache_hit() {
        let dir = temp_dir();
        fs::write(dir.path().join("test.rs"), "content").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let registry = ToolRegistry::new();

        // First call - cache miss
        let input = serde_json::json!({"pattern": "*.rs"});
        registry.execute_with_context("glob", input.clone(), &context).await.unwrap();

        // Second call - should be cache hit
        registry.execute_with_context("glob", input, &context).await.unwrap();

        let stats = registry.cache_stats();
        assert!(stats.hits >= 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_write() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "original").unwrap();

        let mut context = test_context();
        context.cwd = dir.path().to_path_buf();

        let registry = ToolRegistry::new();

        // Read file (populates cache)
        let read_input = serde_json::json!({"path": file_path.to_str().unwrap()});
        registry.execute_with_context("file_read", read_input.clone(), &context).await.unwrap();

        // Write file (should invalidate cache)
        let write_input = serde_json::json!({
            "path": file_path.to_str().unwrap(),
            "content": "modified"
        });
        registry.execute_with_context("file_write", write_input, &context).await.unwrap();

        // Read again - should get fresh content
        let result =
            registry.execute_with_context("file_read", read_input, &context).await.unwrap();

        assert!(result.output.contains("modified"));
    }
}
