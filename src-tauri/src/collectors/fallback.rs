use super::Collector;
use crate::domain::{CollectionOutcome, Provider};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackTrigger {
    CredentialsMissing,
    CliMissing,
}

impl FallbackTrigger {
    pub fn matches(self, outcome: &CollectionOutcome) -> bool {
        matches!(
            (self, outcome),
            (
                Self::CredentialsMissing,
                CollectionOutcome::CredentialsMissing
            ) | (Self::CliMissing, CollectionOutcome::CliMissing)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("fallback collector provider does not match primary collector")]
pub struct ProviderMismatch;

pub struct FallbackCollector {
    provider: Provider,
    primary: Box<dyn Collector>,
    fallback: Box<dyn Collector>,
    trigger: FallbackTrigger,
}

impl FallbackCollector {
    pub fn new(
        primary: Box<dyn Collector>,
        fallback: Box<dyn Collector>,
        trigger: FallbackTrigger,
    ) -> Result<Self, ProviderMismatch> {
        let provider = primary.provider();
        if fallback.provider() != provider {
            return Err(ProviderMismatch);
        }
        Ok(Self {
            provider,
            primary,
            fallback,
            trigger,
        })
    }
}

impl Collector for FallbackCollector {
    fn provider(&self) -> Provider {
        self.provider
    }

    fn collect(
        &self,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = CollectionOutcome> + Send + '_>> {
        Box::pin(async move {
            let outcome = self.primary.collect().await;
            if self.trigger.matches(&outcome) {
                self.fallback.collect().await
            } else {
                outcome
            }
        })
    }
}
