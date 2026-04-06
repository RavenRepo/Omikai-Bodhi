//! Integration tests for session persistence.

use super::*;

#[test]
fn test_full_session_workflow() {
    let store = SessionStore::open_in_memory().unwrap();

    // Create session
    let mut session = store.create_session(Some("workflow-test"), "gpt-4o").unwrap();

    // Add conversation
    let msg1 = SessionMessage::user("What is Rust?");
    store.add_message(session.id, &msg1).unwrap();
    session.add_message(msg1);

    let msg2 = SessionMessage::assistant("Rust is a systems programming language...");
    store.add_message(session.id, &msg2).unwrap();
    session.add_message(msg2);

    // Update tokens
    store.update_tokens(session.id, 150).unwrap();

    // Reload and verify
    let loaded = store.load_session(session.id).unwrap();
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.total_tokens, 150);

    // List should show it
    let summaries = store.list_sessions().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].message_count, 2);

    // Cleanup
    store.delete_session(session.id).unwrap();
    assert_eq!(store.session_count().unwrap(), 0);
}

#[test]
fn test_message_ordering() {
    let store = SessionStore::open_in_memory().unwrap();
    let session = store.create_session(None, "gpt-4o").unwrap();

    // Add messages with slight delay simulation
    for i in 0..5 {
        let msg = SessionMessage::user(format!("Message {}", i));
        store.add_message(session.id, &msg).unwrap();
    }

    let loaded = store.load_session(session.id).unwrap();
    for (i, msg) in loaded.messages.iter().enumerate() {
        assert_eq!(msg.content, format!("Message {}", i));
    }
}

#[test]
fn test_tool_calls_persistence() {
    let store = SessionStore::open_in_memory().unwrap();
    let session = store.create_session(None, "gpt-4o").unwrap();

    let tool_calls_json = r#"[{"name": "file_read", "arguments": {"path": "test.txt"}}]"#;
    let msg = SessionMessage::assistant("Let me read that file.").with_tool_calls(tool_calls_json);

    store.add_message(session.id, &msg).unwrap();

    let loaded = store.load_session(session.id).unwrap();
    assert_eq!(loaded.messages[0].tool_calls, Some(tool_calls_json.to_string()));
}
