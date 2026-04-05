use theasus_core::Message as CoreMessage;
use theasus_language_model::{
    AssistantMessage, ContentBlock as LlmContentBlock, Message as LlmMessage, SystemMessage, Usage,
    UserMessage,
};

pub fn convert_core_to_llm(msg: &CoreMessage) -> LlmMessage {
    match msg {
        CoreMessage::User(m) => LlmMessage::User(UserMessage {
            id: m.id,
            content: m
                .content
                .iter()
                .map(|c| match c {
                    theasus_core::ContentBlock::Text { text } => {
                        LlmContentBlock::Text { text: text.clone() }
                    }
                    theasus_core::ContentBlock::Image { url, detail: _ } => {
                        LlmContentBlock::Text { text: format!("[Image: {}]", url) }
                    }
                    theasus_core::ContentBlock::ToolUse { tool } => {
                        LlmContentBlock::Text { text: format!("[Tool: {}]", tool.name) }
                    }
                    theasus_core::ContentBlock::ToolResult { tool_use_id: _, content } => {
                        LlmContentBlock::Text { text: content.clone() }
                    }
                })
                .collect(),
            timestamp: m.timestamp,
        }),
        CoreMessage::Assistant(m) => LlmMessage::Assistant(AssistantMessage {
            id: m.id,
            content: m
                .content
                .iter()
                .map(|c| match c {
                    theasus_core::ContentBlock::Text { text } => {
                        LlmContentBlock::Text { text: text.clone() }
                    }
                    theasus_core::ContentBlock::Image { url, detail: _ } => {
                        LlmContentBlock::Text { text: format!("[Image: {}]", url) }
                    }
                    theasus_core::ContentBlock::ToolUse { tool } => {
                        LlmContentBlock::Text { text: format!("[Tool: {}]", tool.name) }
                    }
                    theasus_core::ContentBlock::ToolResult { tool_use_id: _, content } => {
                        LlmContentBlock::Text { text: content.clone() }
                    }
                })
                .collect(),
            tool_calls: vec![],
            usage: Usage {
                input_tokens: m.usage.input_tokens,
                output_tokens: m.usage.output_tokens,
                total_tokens: m.usage.total_tokens,
            },
            model: m.model.clone(),
            stop_reason: m.stop_reason.clone(),
            timestamp: m.timestamp,
        }),
        CoreMessage::System(m) => LlmMessage::System(SystemMessage {
            id: uuid::Uuid::new_v4(),
            content: vec![LlmContentBlock::Text { text: m.content.clone() }],
            timestamp: chrono::Utc::now(),
        }),
        _ => LlmMessage::User(UserMessage {
            id: uuid::Uuid::new_v4(),
            content: vec![],
            timestamp: chrono::Utc::now(),
        }),
    }
}
