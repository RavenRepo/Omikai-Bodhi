use crate::{Tool, ToolContext, ToolDefinition, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const MAX_RESPONSE_SIZE: usize = 10 * 1024; // 10KB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchInput {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchOutput {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub truncated: bool,
}

pub struct WebFetchTool;

impl WebFetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "web_fetch"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "web_fetch".to_string(),
            description: "Make HTTP requests to fetch data from URLs".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch"
                    },
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD"],
                        "description": "HTTP method (default: GET)"
                    },
                    "headers": {
                        "type": "object",
                        "additionalProperties": { "type": "string" },
                        "description": "Optional HTTP headers to include"
                    },
                    "body": {
                        "type": "string",
                        "description": "Optional request body (for POST, PUT, PATCH)"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn execute(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> crate::Result<ToolResult> {
        let fetch_input: WebFetchInput =
            serde_json::from_value(input).map_err(|e| crate::TheasusError::Tool {
                tool: "web_fetch".to_string(),
                reason: format!("Invalid input: {}", e),
            })?;

        let client = reqwest::Client::new();

        let method = match fetch_input.method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            "HEAD" => reqwest::Method::HEAD,
            _ => {
                return Ok(ToolResult::error(format!(
                    "Unsupported HTTP method: {}",
                    fetch_input.method
                )));
            }
        };

        let mut request = client.request(method, &fetch_input.url);

        if let Some(headers) = fetch_input.headers {
            for (key, value) in headers {
                request = request.header(&key, &value);
            }
        }

        if let Some(body) = fetch_input.body {
            request = request.body(body);
        }

        let response = request.send().await.map_err(|e| crate::TheasusError::Tool {
            tool: "web_fetch".to_string(),
            reason: format!("Request failed: {}", e),
        })?;

        let status = response.status().as_u16();
        let response_headers: HashMap<String, String> = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
            .collect();

        let body_bytes = response.bytes().await.map_err(|e| crate::TheasusError::Tool {
            tool: "web_fetch".to_string(),
            reason: format!("Failed to read response body: {}", e),
        })?;

        let (body, truncated) = if body_bytes.len() > MAX_RESPONSE_SIZE {
            let truncated_bytes = &body_bytes[..MAX_RESPONSE_SIZE];
            let body = String::from_utf8_lossy(truncated_bytes).to_string();
            (body, true)
        } else {
            let body = String::from_utf8_lossy(&body_bytes).to_string();
            (body, false)
        };

        let output = WebFetchOutput {
            status,
            headers: response_headers,
            body,
            truncated,
        };

        let output_json = serde_json::to_string_pretty(&output).map_err(|e| {
            crate::TheasusError::Tool {
                tool: "web_fetch".to_string(),
                reason: format!("Failed to serialize output: {}", e),
            }
        })?;

        Ok(ToolResult::success(output_json))
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context() -> ToolContext {
        ToolContext {
            cwd: PathBuf::from("."),
            session_id: uuid::Uuid::new_v4(),
            user_id: None,
        }
    }

    #[test]
    fn test_tool_definition() {
        let tool = WebFetchTool::new();
        let def = tool.definition();
        assert_eq!(def.name, "web_fetch");
        assert!(def.input_schema.is_object());
    }

    #[test]
    fn test_default_method() {
        let input: WebFetchInput = serde_json::from_str(r#"{"url": "https://example.com"}"#).unwrap();
        assert_eq!(input.method, "GET");
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool::new();
        let context = test_context();

        let result = tool
            .execute(
                serde_json::json!({
                    "url": "not-a-valid-url"
                }),
                &context,
            )
            .await;

        // Should return an error since the URL is invalid
        assert!(result.is_err() || !result.unwrap().success);
    }

    #[test]
    fn test_web_fetch_output_structure() {
        let output = WebFetchOutput {
            status: 200,
            headers: HashMap::from([("content-type".to_string(), "application/json".to_string())]),
            body: r#"{"key": "value"}"#.to_string(),
            truncated: false,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("200"));
        assert!(json.contains("content-type"));
    }
}
