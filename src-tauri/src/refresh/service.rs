use serde::{Deserialize, Serialize};
use tokio::sync::watch;

use super::{ProviderState, RefreshHandle, RefreshReason};
use crate::domain::Provider;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProviderStates {
    pub claude: ProviderState,
    pub codex: ProviderState,
}

#[derive(Clone)]
pub struct RefreshService {
    claude: RefreshHandle,
    codex: RefreshHandle,
}

impl RefreshService {
    pub fn new(claude: RefreshHandle, codex: RefreshHandle) -> Self {
        debug_assert_eq!(claude.provider(), Provider::Claude);
        debug_assert_eq!(codex.provider(), Provider::Codex);
        Self { claude, codex }
    }

    pub fn get_provider_states(&self) -> ProviderStates {
        ProviderStates {
            claude: self.claude.current(),
            codex: self.codex.current(),
        }
    }

    pub async fn refresh_provider(&self, provider: Provider) -> Result<(), &'static str> {
        match provider {
            Provider::Claude => &self.claude,
            Provider::Codex => &self.codex,
        }
        .trigger(RefreshReason::Manual)
        .await
    }

    pub async fn focus_or_resume(&self, reason: RefreshReason) -> Result<(), &'static str> {
        let (claude, codex) = tokio::join!(self.claude.trigger(reason), self.codex.trigger(reason));
        claude.and(codex)
    }

    pub fn subscribe(&self, provider: Provider) -> watch::Receiver<ProviderState> {
        match provider {
            Provider::Claude => &self.claude,
            Provider::Codex => &self.codex,
        }
        .subscribe()
    }
}
