use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use theasus_language_model::{
    AssistantMessage, CompletionChunk, CompletionRequest, CompletionResponse, ContentBlock,
    LanguageModel, Message, Usage,
};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum LlmProvider {
    #[default]
    OpenAi,
    Anthropic,
    Ollama,
    Custom {
        name: String,
        base_url: String,
    },
}

pub struct OmikProvider {
    http_client: Arc<dyn theasus_http_client::HttpClient>,
    api_key: String,
    provider: LlmProvider,
    model: String,
    default_headers: HeaderMap,
}

impl OmikProvider {
    pub fn new(
        http_client: Arc<dyn theasus_http_client::HttpClient>,
        api_key: String,
        provider: LlmProvider,
        model: String,
    ) -> Self {
        let mut default_headers = HeaderMap::new();

        match &provider {
            LlmProvider::OpenAi => {
                default_headers.insert(
                    "Authorization",
                    HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
                );
            }
            LlmProvider::Anthropic => {
                default_headers.insert("x-api-key", HeaderValue::from_str(&api_key).unwrap());
                default_headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
            }
            LlmProvider::Ollama => {
                // Ollama typically doesn't need auth
            }
            LlmProvider::Custom { .. } => {
                default_headers.insert(
                    "Authorization",
                    HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
                );
            }
        }

        Self {
            http_client,
            api_key,
            provider,
            model,
            default_headers,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    fn get_base_url(&self) -> String {
        match &self.provider {
            LlmProvider::OpenAi => "https://api.openai.com".to_string(),
            LlmProvider::Anthropic => "https://api.anthropic.com".to_string(),
            LlmProvider::Ollama => "http://localhost:11434".to_string(),
            LlmProvider::Custom { base_url, .. } => base_url.clone(),
        }
    }

    fn get_endpoint(&self) -> String {
        match &self.provider {
            LlmProvider::OpenAi => "/v1/chat/completions".to_string(),
            LlmProvider::Anthropic => "/v1/messages".to_string(),
            LlmProvider::Ollama => "/api/chat".to_string(),
            LlmProvider::Custom { .. } => "/v1/chat/completions".to_string(),
        }
    }
}

#[async_trait]
impl LanguageModel for OmikProvider {
    fn id(&self) -> &str {
        &self.model
    }

    fn name(&self) -> &str {
        match &self.provider {
            LlmProvider::OpenAi => "OpenAI",
            LlmProvider::Anthropic => "Anthropic",
            LlmProvider::Ollama => "Ollama (Local)",
            LlmProvider::Custom { name, .. } => name,
        }
    }

    fn max_tokens(&self) -> Option<u32> {
        Some(4096)
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let base_url = self.get_base_url();
        let endpoint = self.get_endpoint();
        let url = format!("{}{}", base_url, endpoint);

        let client = reqwest::Client::new();

        let mut headers = self.default_headers.clone();

        match self.provider {
            LlmProvider::OpenAi | LlmProvider::Custom { .. } => {
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
            }
            LlmProvider::Anthropic => {
                headers.insert(
                    reqwest::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
            }
            LlmProvider::Ollama => {}
        }

        match self.provider {
            LlmProvider::OpenAi | LlmProvider::Custom { .. } => {
                let req_body = OpenAiRequest {
                    model: &request.model,
                    messages: request
                        .messages
                        .iter()
                        .map(convert_message_openai)
                        .collect(),
                    max_tokens: request.max_tokens,
                    temperature: request.temperature,
                    stream: false,
                };

                let response = client
                    .post(&url)
                    .headers(headers)
                    .json(&req_body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let body = response.text().await?;
                    return Err(anyhow!("API error: {}", body));
                }

                let resp: OpenAiResponse = response.json().await?;

                Ok(CompletionResponse {
                    message: Message::Assistant(AssistantMessage {
                        id: Uuid::new_v4(),
                        content: vec![ContentBlock::Text {
                            text: resp
                                .choices
                                .first()
                                .map(|c| c.message.content.clone())
                                .unwrap_or_default(),
                        }],
                        tool_calls: vec![],
                        usage: Usage {
                            input_tokens: resp.usage.prompt_tokens,
                            output_tokens: resp.usage.completion_tokens,
                            total_tokens: resp.usage.total_tokens,
                        },
                        model: self.model.clone(),
                        stop_reason: resp.choices.first().and_then(|c| c.finish_reason.clone()),
                        timestamp: Utc::now(),
                    }),
                    usage: Usage {
                        input_tokens: resp.usage.prompt_tokens,
                        output_tokens: resp.usage.completion_tokens,
                        total_tokens: resp.usage.total_tokens,
                    },
                    stop_reason: resp.choices.first().and_then(|c| c.finish_reason.clone()),
                })
            }
            LlmProvider::Anthropic => {
                let req_body = AnthropicRequest {
                    model: &request.model,
                    messages: request
                        .messages
                        .iter()
                        .map(convert_message_anthropic)
                        .collect(),
                    max_tokens: request.max_tokens.unwrap_or(4096),
                    temperature: request.temperature,
                    system: request.system.clone(),
                };

                let response = client
                    .post(&url)
                    .headers(headers)
                    .json(&req_body)
                    .send()
                    .await?;

                if !response.status().is_success() {
                    let body = response.text().await?;
                    return Err(anyhow!("API error: {}", body));
                }

                let resp: AnthropicResponse = response.json().await?;

                let text_content = resp
                    .content
                    .first()
                    .map(|c| match c {
                        AnthropicContent::Text { text } => text.clone(),
                    })
                    .unwrap_or_default();

                Ok(CompletionResponse {
                    message: Message::Assistant(AssistantMessage {
                        id: Uuid::new_v4(),
                        content: vec![ContentBlock::Text { text: text_content }],
                        tool_calls: vec![],
                        usage: Usage {
                            input_tokens: resp.usage.input_tokens,
                            output_tokens: resp.usage.output_tokens,
                            total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
                        },
                        model: self.model.clone(),
                        stop_reason: resp.stop_reason.clone(),
                        timestamp: Utc::now(),
                    }),
                    usage: Usage {
                        input_tokens: resp.usage.input_tokens,
                        output_tokens: resp.usage.output_tokens,
                        total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
                    },
                    stop_reason: resp.stop_reason,
                })
            }
            LlmProvider::Ollama => {
                let req_body = OllamaRequest {
                    model: &request.model,
                    messages: request
                        .messages
                        .iter()
                        .map(convert_message_ollama)
                        .collect(),
                    stream: false,
                };

                let response = client.post(&url).json(&req_body).send().await?;

                if !response.status().is_success() {
                    let body = response.text().await?;
                    return Err(anyhow!("API error: {}", body));
                }

                let resp: OllamaResponse = response.json().await?;

                Ok(CompletionResponse {
                    message: Message::Assistant(AssistantMessage {
                        id: Uuid::new_v4(),
                        content: vec![ContentBlock::Text {
                            text: resp.message.content,
                        }],
                        tool_calls: vec![],
                        usage: Usage::default(),
                        model: self.model.clone(),
                        stop_reason: Some("stop".to_string()),
                        timestamp: Utc::now(),
                    }),
                    usage: Usage::default(),
                    stop_reason: Some("stop".to_string()),
                })
            }
        }
    }

    async fn stream(
        &self,
        _request: CompletionRequest,
    ) -> Result<Box<dyn Stream<Item = Result<CompletionChunk>> + Send>> {
        todo!("Streaming not implemented")
    }
}

fn convert_message_openai(msg: &Message) -> OpenAiMessage {
    match msg {
        Message::User(m) => OpenAiMessage {
            role: "user".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        Message::Assistant(m) => OpenAiMessage {
            role: "assistant".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        Message::System(m) => OpenAiMessage {
            role: "system".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        _ => OpenAiMessage {
            role: "user".to_string(),
            content: String::new(),
        },
    }
}

fn convert_message_anthropic(msg: &Message) -> AnthropicMessage {
    match msg {
        Message::User(m) => AnthropicMessage {
            role: "user".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => AnthropicContent::Text { text: text.clone() },
                    _ => AnthropicContent::Text {
                        text: String::new(),
                    },
                })
                .collect(),
        },
        Message::Assistant(m) => AnthropicMessage {
            role: "assistant".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => AnthropicContent::Text { text: text.clone() },
                    _ => AnthropicContent::Text {
                        text: String::new(),
                    },
                })
                .collect(),
        },
        Message::System(m) => AnthropicMessage {
            role: "system".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => AnthropicContent::Text { text: text.clone() },
                    _ => AnthropicContent::Text {
                        text: String::new(),
                    },
                })
                .collect(),
        },
        _ => AnthropicMessage {
            role: "user".to_string(),
            content: vec![],
        },
    }
}

fn convert_message_ollama(msg: &Message) -> OllamaMessage {
    match msg {
        Message::User(m) => OllamaMessage {
            role: "user".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        Message::Assistant(m) => OllamaMessage {
            role: "assistant".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        Message::System(m) => OllamaMessage {
            role: "system".to_string(),
            content: m
                .content
                .iter()
                .map(|c| match c {
                    ContentBlock::Text { text } => text.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        },
        _ => OllamaMessage {
            role: "user".to_string(),
            content: String::new(),
        },
    }
}

#[derive(Debug, Serialize)]
struct OpenAiRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAiMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    id: String,
    choices: Vec<OpenAiChoice>,
    usage: OpenAiUsage,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    temperature: Option<f32>,
    system: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
enum AnthropicContent {
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}
