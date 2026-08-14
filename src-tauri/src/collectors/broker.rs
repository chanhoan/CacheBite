use super::CollectorError;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, future::Future, io::Read, path::PathBuf, pin::Pin};
use zeroize::Zeroize;

pub(crate) const MAX_CREDENTIAL_BYTES: u64 = 64 * 1024;

pub trait ClaudeTokenSource: Send + Sync {
    fn claude_token(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<ClaudeCredential, CollectorError>> + Send + '_>>;
}

/// A Claude credential as the collectors need it: the bearer token, plus the
/// subscription tier that sits beside it in the same file.
///
/// The tier is a plain `String`, not a `SecretString`: `"pro"` / `"max"` is a
/// plan grade, not an authorization value or an account identifier, and the
/// panel is meant to display it. Only the token is zeroized.
pub struct ClaudeCredential {
    pub token: SecretString,
    pub subscription_type: Option<String>,
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

    pub fn claude_token(&self) -> Result<ClaudeCredential, CollectorError> {
        if let Some(value) = &self.environment_token {
            // `CLAUDE_CODE_OAUTH_TOKEN` bypasses the credential file, which is
            // the only place the tier is recorded — so there is nothing to
            // report and the panel simply shows no chip.
            return Ok(ClaudeCredential {
                token: SecretString::from(value.expose_secret().to_owned()),
                subscription_type: None,
            });
        }
        let mut saw_invalid = false;
        for path in &self.locations.claude {
            match read_credential(path) {
                Ok(Some(credential)) => return Ok(credential),
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
    ) -> Pin<Box<dyn Future<Output = Result<ClaudeCredential, CollectorError>> + Send + '_>> {
        Box::pin(std::future::ready(self.claude_token()))
    }
}

fn read_credential(path: &std::path::Path) -> Result<Option<ClaudeCredential>, ()> {
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

pub(crate) fn parse_token_bytes(contents: &[u8]) -> Result<Option<ClaudeCredential>, ()> {
    if contents.len() as u64 > MAX_CREDENTIAL_BYTES {
        return Err(());
    }
    let mut wire: ClaudeCredentials = serde_json::from_slice(contents).map_err(|_| ())?;
    // Read the tier before the token is taken: the tier lives in the same
    // nested object, and taking the token first is a refactor away from
    // leaving this reading an emptied struct.
    let subscription_type = wire
        .claude_ai_oauth
        .as_mut()
        .and_then(|oauth| oauth.subscription_type.take())
        .filter(|value| !value.is_empty());
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
        Some(value) => Ok(Some(ClaudeCredential {
            token: SecretString::from(value),
            subscription_type,
        })),
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
    // The one field beyond the token this parser opens. Claude Code's own
    // `/status` reads the tier from here; the usage endpoint does not carry it
    // (checked — 18 top-level keys, none a tier). `rateLimitTier` sits beside
    // it and is deliberately left unparsed: nothing displays it.
    #[serde(rename = "subscriptionType")]
    subscription_type: Option<String>,
}

impl Drop for OAuthCredentials {
    fn drop(&mut self) {
        // Only the token. The tier is not a secret, and zeroizing it would tell
        // the next reader that it is one.
        self.access_token.zeroize();
    }
}
