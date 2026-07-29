use super::{broker::ClaudeTokenSource, Collector, CollectorError, MAX_RESPONSE_BYTES};
use crate::domain::{CollectionOutcome, Provider, ProviderUsageSnapshot, Source, UsageWindow};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Url,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use time::OffsetDateTime;

pub struct ClaudeCollector {
    token_source: std::sync::Arc<dyn ClaudeTokenSource>,
    client: reqwest::Client,
}

impl ClaudeCollector {
    pub fn new(token_source: std::sync::Arc<dyn ClaudeTokenSource>) -> Self {
        Self {
            token_source,
            client: secure_client().expect("static secure Claude client configuration is valid"),
        }
    }
}

impl Collector for ClaudeCollector {
    fn provider(&self) -> Provider {
        Provider::Claude
    }

    fn collect(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async move {
            let result = match self.token_source.claude_token().await {
                Ok(token) => {
                    fetch_usage_with_client(&self.client, token, OffsetDateTime::now_utc()).await
                }
                Err(error) => Err(error),
            };
            match result {
                Ok(snapshot) => CollectionOutcome::Success { snapshot },
                Err(error) => error.into_outcome(),
            }
        })
    }
}

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";

pub struct ClaudeRequestSpec {
    url: Url,
    headers: HeaderMap,
}

impl std::fmt::Debug for ClaudeRequestSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ClaudeRequestSpec")
            .field("url", &self.url)
            .field("header_names", &self.header_names())
            .finish()
    }
}

impl ClaudeRequestSpec {
    pub fn new(token: impl Into<SecretString>) -> Result<Self, CollectorError> {
        let token = token.into();
        let url = Url::parse(USAGE_URL).map_err(|_| CollectorError::Internal)?;
        if url.scheme() != "https" || url.host_str() != Some("api.anthropic.com") {
            return Err(CollectorError::Internal);
        }
        let mut headers = HeaderMap::new();
        let authorization = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
            .map_err(|_| CollectorError::CredentialFileInvalid)?;
        headers.insert(reqwest::header::AUTHORIZATION, authorization);
        headers.insert(
            HeaderName::from_static("anthropic-beta"),
            HeaderValue::from_static("oauth-2025-04-20"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static("claude-code/2.0.0 (CacheBite)"),
        );
        Ok(Self { url, headers })
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn header_names(&self) -> [&'static str; 3] {
        ["anthropic-beta", "authorization", "user-agent"]
    }

    pub(crate) fn into_request(self, client: &reqwest::Client) -> reqwest::RequestBuilder {
        client.get(self.url).headers(self.headers)
    }

    #[cfg(test)]
    pub(crate) fn build_for_test(self) -> Result<reqwest::Request, CollectorError> {
        self.into_request(&secure_client()?)
            .build()
            .map_err(|_| CollectorError::Internal)
    }
}

async fn fetch_usage_with_client(
    client: &reqwest::Client,
    token: SecretString,
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    let mut response = ClaudeRequestSpec::new(token)?
        .into_request(client)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                CollectorError::Timeout
            } else {
                CollectorError::Network
            }
        })?;
    if !response.status().is_success() {
        #[cfg(debug_assertions)]
        eprintln!(
            "[CacheBite:claude] usage endpoint returned HTTP {}",
            response.status()
        );
        return Err(CollectorError::Provider);
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(CollectorError::ResponseTooLarge);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| CollectorError::Network)?
    {
        if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(CollectorError::ResponseTooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    parse_usage(&body, now)
}

fn secure_client() -> Result<reqwest::Client, CollectorError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| CollectorError::Internal)
}

pub fn parse_usage(
    body: &[u8],
    now: OffsetDateTime,
) -> Result<ProviderUsageSnapshot, CollectorError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(CollectorError::ResponseTooLarge);
    }
    let wire: UsageWire = serde_json::from_slice(body).map_err(|_| CollectorError::Parse)?;
    let session = window(wire.five_hour, 300)?;
    let weekly = window(wire.seven_day, 10_080)?;
    if session.is_none() && weekly.is_none() {
        return Err(CollectorError::Parse);
    }
    Ok(ProviderUsageSnapshot {
        provider: Provider::Claude,
        plan_type: None,
        session,
        weekly,
        captured_at: now,
        source: Source::OauthApi,
        is_cached: false,
        revision: 0,
    })
}

fn window(value: Option<WindowWire>, minutes: u32) -> Result<Option<UsageWindow>, CollectorError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let percent = value
        .utilization
        .or(value.percent)
        .ok_or(CollectorError::Parse)?;
    let reset = value.resets_at.or(value.reset_at);
    UsageWindow::new(percent, minutes, reset)
        .map(Some)
        .map_err(|_| CollectorError::Parse)
}

#[derive(Deserialize)]
struct UsageWire {
    five_hour: Option<WindowWire>,
    seven_day: Option<WindowWire>,
}

#[derive(Deserialize)]
struct WindowWire {
    utilization: Option<f64>,
    percent: Option<f64>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    resets_at: Option<OffsetDateTime>,
    #[serde(default, with = "time::serde::rfc3339::option")]
    reset_at: Option<OffsetDateTime>,
}
