use super::CollectorError;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, future::Future, io::Read, path::PathBuf, pin::Pin};
use zeroize::Zeroize;

pub(crate) const MAX_CREDENTIAL_BYTES: u64 = 64 * 1024;

pub trait ClaudeTokenSource: Send + Sync {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>>;
}

#[derive(Clone)]
pub struct CredentialLocations {
    pub claude: Vec<PathBuf>,
}

impl CredentialLocations {
    pub fn documented(config_dir: Option<PathBuf>, home_dir: Option<PathBuf>) -> Self {
        let mut claude = Vec::new();
        if let Some(config_dir) = config_dir {
            claude.push(config_dir.join(".credentials.json"));
        }
        if let Some(home_dir) = home_dir {
            let fallback = home_dir.join(".claude").join(".credentials.json");
            if !claude.contains(&fallback) {
                claude.push(fallback);
            }
        }
        Self { claude }
    }
}

pub struct CredentialBroker {
    environment_token: Option<SecretString>,
    locations: CredentialLocations,
}

impl CredentialBroker {
    pub fn new(mut environment: BTreeMap<String, String>, locations: CredentialLocations) -> Self {
        Self {
            environment_token: environment
                .remove("CLAUDE_CODE_OAUTH_TOKEN")
                .filter(|value| !value.is_empty())
                .map(SecretString::from),
            locations,
        }
    }

    pub fn claude_token(&self) -> Result<SecretString, CollectorError> {
        if let Some(value) = &self.environment_token {
            return Ok(SecretString::from(value.expose_secret().to_owned()));
        }
        let mut saw_invalid = false;
        for path in &self.locations.claude {
            match read_token(path) {
                Ok(Some(token)) => return Ok(token),
                Ok(None) => {}
                Err(()) => saw_invalid = true,
            }
        }
        Err(if saw_invalid {
            CollectorError::CredentialFileInvalid
        } else {
            CollectorError::CredentialsMissing
        })
    }
}

impl ClaudeTokenSource for CredentialBroker {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<SecretString, CollectorError>> + Send + '_>> {
        Box::pin(std::future::ready(self.claude_token()))
    }
}

fn read_token(path: &std::path::Path) -> Result<Option<SecretString>, ()> {
    let mut file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(()),
    };
    let metadata = file.metadata().map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() > MAX_CREDENTIAL_BYTES {
        return Err(());
    }
    let mut contents = Vec::with_capacity(metadata.len() as usize);
    file.by_ref()
        .take(MAX_CREDENTIAL_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|_| ())?;
    if contents.len() as u64 > MAX_CREDENTIAL_BYTES {
        return Err(());
    }
    let result = parse_token_bytes(&contents);
    contents.zeroize();
    result
}

pub(crate) fn parse_token_bytes(contents: &[u8]) -> Result<Option<SecretString>, ()> {
    if contents.len() as u64 > MAX_CREDENTIAL_BYTES {
        return Err(());
    }
    let mut wire: ClaudeCredentials = serde_json::from_slice(contents).map_err(|_| ())?;
    let token = wire
        .claude_ai_oauth
        .as_mut()
        .and_then(|oauth| oauth.access_token.take())
        .or_else(|| wire.oauth_access_token.take())
        .or_else(|| wire.access_token.take());
    match token {
        Some(mut value) if value.is_empty() => {
            value.zeroize();
            Ok(None)
        }
        Some(value) => Ok(Some(SecretString::from(value))),
        None => Ok(None),
    }
}

#[derive(Deserialize)]
struct ClaudeCredentials {
    #[serde(rename = "claudeAiOauth")]
    claude_ai_oauth: Option<OAuthCredentials>,
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
    #[serde(rename = "oauthAccessToken")]
    oauth_access_token: Option<String>,
}

impl Drop for ClaudeCredentials {
    fn drop(&mut self) {
        self.access_token.zeroize();
        self.oauth_access_token.zeroize();
    }
}

#[derive(Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}

impl Drop for OAuthCredentials {
    fn drop(&mut self) {
        self.access_token.zeroize();
    }
}
