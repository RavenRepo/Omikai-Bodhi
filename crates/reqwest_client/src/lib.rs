use async_trait::async_trait;
use reqwest::Client;

pub struct ReqwestClient {
    client: Client,
    api_key: Option<String>,
}

impl ReqwestClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder().build()?;
        Ok(Self { client, api_key: None })
    }

    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = Some(api_key.to_string());
        self
    }

    fn apply_headers(&self, request: &mut reqwest::Request) {
        request.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        request.headers_mut().insert(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        if let Some(key) = &self.api_key {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_static("x-api-key"),
                reqwest::header::HeaderValue::from_str(key).unwrap(),
            );
        }
    }
}

impl Default for ReqwestClient {
    fn default() -> Self {
        Self::new().expect("Failed to create ReqwestClient")
    }
}

#[async_trait]
impl theasus_http_client::HttpClient for ReqwestClient {
    async fn send(
        &self,
        mut request: reqwest::Request,
    ) -> theasus_http_client::Result<reqwest::Response> {
        self.apply_headers(&mut request);
        Ok(self.client.execute(request).await?)
    }

    fn build_request(&self, method: &str, url: &str) -> reqwest::RequestBuilder {
        self.client.request(method.parse().unwrap(), url)
    }
}
