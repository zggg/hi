use std::time::Duration;

/// Shared HTTP client for LLM providers (connect + overall timeouts).
pub fn build() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
