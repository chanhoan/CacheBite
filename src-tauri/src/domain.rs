use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub fn is_valid_pet_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 64
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    OauthApi,
    CliRpc,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    Network,
    Provider,
    Parse,
    Internal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotInstalled,
    NotSignedIn,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "UsageWindowWire")]
pub struct UsageWindow {
    pub used_percent: f64,
    pub window_minutes: u32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub resets_at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
struct UsageWindowWire {
    used_percent: f64,
    window_minutes: u32,
    #[serde(with = "time::serde::rfc3339::option")]
    resets_at: Option<OffsetDateTime>,
}

impl UsageWindow {
    pub fn new(
        used_percent: f64,
        window_minutes: u32,
        resets_at: Option<OffsetDateTime>,
    ) -> Result<Self, &'static str> {
        if !used_percent.is_finite() {
            return Err("used_percent must be finite");
        }
        if window_minutes == 0 {
            return Err("window_minutes must be positive");
        }
        Ok(Self {
            used_percent: used_percent.clamp(0.0, 100.0),
            window_minutes,
            resets_at,
        })
    }
}

impl TryFrom<UsageWindowWire> for UsageWindow {
    type Error = &'static str;

    fn try_from(value: UsageWindowWire) -> Result<Self, Self::Error> {
        Self::new(value.used_percent, value.window_minutes, value.resets_at)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProviderUsageSnapshot {
    pub provider: Provider,
    pub plan_type: Option<String>,
    pub session: Option<UsageWindow>,
    pub weekly: Option<UsageWindow>,
    #[serde(with = "time::serde::rfc3339")]
    pub captured_at: OffsetDateTime,
    pub source: Source,
    pub is_cached: bool,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CollectionOutcome {
    Success { snapshot: ProviderUsageSnapshot },
    Failed { class: FailureClass },
    CredentialsMissing,
    CliMissing,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProviderUiSnapshot {
    #[serde(flatten)]
    pub snapshot: ProviderUsageSnapshot,
    pub failure_class: Option<FailureClass>,
    pub unavailable_reason: Option<UnavailableReason>,
}
