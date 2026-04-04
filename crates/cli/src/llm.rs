use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use theasus_omik_provider::{LlmProvider, OmikProvider};

pub struct LlmManager {
    pub client: Option<Arc<dyn theasus_language_model::LanguageModel>>,
    pub settings: theasus_settings::Settings,
}

impl LlmManager {
    pub fn new() -> Self {
        let settings = theasus_settings::Settings::load().unwrap_or_default();
        let client = Self::create_client(&settings);
        
        Self {
            client,
            settings,
        }
    }

    fn create_client(settings: &theasus_settings::Settings) -> Option<Arc<dyn theasus_language_model::LanguageModel>> {
        let api_key = settings.api_key.as_ref()?;
        
        let provider = match settings.llm_provider.to_lowercase().as_str() {
            "openai" => LlmProvider::OpenAi,
            "anthropic" => LlmProvider::Anthropic,
            "ollama" => LlmProvider::Ollama,
            "custom" => LlmProvider::Custom { 
                name: "custom".to_string(), 
                base_url: settings.llm_base_url.clone().unwrap_or_default() 
            },
            _ => LlmProvider::OpenAi,
        };

        let http_client = theasus_reqwest_client::ReqwestClient::new().ok()?;
        
        let http_client: Arc<dyn theasus_http_client::HttpClient> = Arc::new(http_client);
        
        let client = OmikProvider::new(
            http_client,
            api_key.clone(),
            provider,
            settings.model.clone(),
        );

        Some(Arc::new(client))
    }

    pub fn reconfigure(&mut self) {
        self.settings = theasus_settings::Settings::load().unwrap_or_default();
        self.client = Self::create_client(&self.settings);
    }

    pub fn is_configured(&self) -> bool {
        self.client.is_some()
    }

    pub async fn complete(&self, prompt: &str) -> Result<theasus_language_model::CompletionResponse> {
        match &self.client {
            Some(client) => {
                let request = theasus_language_model::CompletionRequest {
                    model: self.settings.model.clone(),
                    messages: vec![
                        theasus_language_model::Message::User(theasus_language_model::UserMessage {
                            id: Uuid::new_v4(),
                            content: vec![theasus_language_model::ContentBlock::Text {
                                text: prompt.to_string(),
                            }],
                            timestamp: Utc::now(),
                        })
                    ],
                    max_tokens: Some(4096),
                    temperature: Some(0.7),
                    system: Some("You are Bodhi, an AI terminal assistant.".to_string()),
                    tools: None,
                    stream: false,
                };
                client.complete(request).await.map_err(|e| anyhow::anyhow!("LLM error: {}", e))
            }
            None => Err(anyhow::anyhow!("LLM not configured. Run: bodhi config-llm --provider openai --api-key YOUR_KEY --model gpt-4o")),
        }
    }
}

impl Default for LlmManager {
    fn default() -> Self {
        Self::new()
    }
}
