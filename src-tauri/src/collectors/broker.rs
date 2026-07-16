use super::CollectorError;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf};

const MAX_CREDENTIAL_BYTES: u64 = 64 * 1024;

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
    let wire: ClaudeCredentials = serde_json::from_slice(&contents).map_err(|_| ())?;
    Ok(wire
        .claude_ai_oauth
        .and_then(|oauth| oauth.access_token)
        .or(wire.oauth_access_token)
        .or(wire.access_token)
        .filter(|value| !value.is_empty())
        .map(SecretString::from))
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

#[derive(Deserialize)]
struct OAuthCredentials {
    #[serde(rename = "accessToken")]
    access_token: Option<String>,
}
