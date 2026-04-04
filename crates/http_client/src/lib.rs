use async_trait::async_trait;
use std::sync::Arc;

pub type Result<T> = std::result::Result<T, anyhow::Error>;

#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn send(&self, request: reqwest::Request) -> Result<reqwest::Response>;
    fn build_request(&self, method: &str, url: &str) -> reqwest::RequestBuilder;
}

pub type DynHttpClient = Arc<dyn HttpClient>;
