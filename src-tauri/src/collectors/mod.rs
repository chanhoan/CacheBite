pub mod broker;
pub mod claude;
pub mod codex;

use crate::domain::{CollectionOutcome, FailureClass, Provider};
use std::{future::Future, pin::Pin};

pub const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CollectorError {
    #[error("credentials are missing")]
    CredentialsMissing,
    #[error("credential file is invalid")]
    CredentialFileInvalid,
    #[error("provider CLI is unavailable")]
    CliMissing,
    #[error("provider request failed")]
    Network,
    #[error("provider rejected the request")]
    Provider,
    #[error("provider response was invalid")]
    Parse,
    #[error("provider protocol was invalid")]
    Protocol,
    #[error("provider operation timed out")]
    Timeout,
    #[error("provider response exceeded the size limit")]
    ResponseTooLarge,
    #[error("internal collector failure")]
    Internal,
}

impl CollectorError {
    pub fn into_outcome(self) -> CollectionOutcome {
        match self {
            Self::CredentialsMissing => CollectionOutcome::CredentialsMissing,
            Self::CliMissing => CollectionOutcome::CliMissing,
            Self::Network | Self::Timeout => CollectionOutcome::Failed {
                class: FailureClass::Network,
            },
            Self::Provider => CollectionOutcome::Failed {
                class: FailureClass::Provider,
            },
            Self::Parse | Self::Protocol | Self::ResponseTooLarge | Self::CredentialFileInvalid => {
                CollectionOutcome::Failed {
                    class: FailureClass::Parse,
                }
            }
            Self::Internal => CollectionOutcome::Failed {
                class: FailureClass::Internal,
            },
        }
    }
}

pub trait Collector: Send + Sync {
    fn provider(&self) -> Provider;
    fn collect(&self) -> Pin<Box<dyn Future<Output = CollectionOutcome> + Send + '_>>;
}

/// Deterministic collector used only when the explicit E2E fixture gate is set.
pub struct UnavailableFixtureCollector(pub Provider);

impl Collector for UnavailableFixtureCollector {
    fn provider(&self) -> Provider {
        self.0
    }
    fn collect(&self) -> Pin<Box<dyn Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async { CollectionOutcome::CredentialsMissing })
    }
}

#[cfg(test)]
mod tests;
