use serde::Serialize;

/// Why an update attempt did not complete. Typed rather than a message so the
/// renderer never receives a URL, a response body, or a filesystem path — the
/// same rule the provider `FailureClass` follows.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateFailure {
    Offline,
    RateLimited,
    MetadataInvalid,
    ArtifactMissing,
    DownloadFailed,
    VerificationFailed,
    InstallFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
        notes: Option<String>,
    },
    Downloading {
        received: u64,
        total: Option<u64>,
    },
    Installing {
        version: String,
    },
    Failed {
        reason: UpdateFailure,
    },
}

impl UpdateStatus {
    /// Whether an install is already under way. A background check must not
    /// overwrite the progress the user is watching.
    pub fn is_installing(&self) -> bool {
        matches!(
            self,
            UpdateStatus::Downloading { .. } | UpdateStatus::Installing { .. }
        )
    }
}

/// Release notes are public, but an unbounded body would resize the panel to
/// the height of a changelog.
pub const MAX_NOTES_CHARS: usize = 400;

/// Trims a release body and caps it at [`MAX_NOTES_CHARS`] characters.
///
/// Counting characters rather than bytes is what keeps a multi-byte body from
/// being split mid-codepoint; `char_indices` gives the byte offset of the first
/// character past the cap, which is always a valid boundary.
pub fn truncate_notes(notes: &str) -> Option<String> {
    let trimmed = notes.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.char_indices().nth(MAX_NOTES_CHARS) {
        None => Some(trimmed.to_owned()),
        Some((cutoff, _)) => Some(format!("{}…", &trimmed[..cutoff])),
    }
}

/// The whole update surface the renderer ever sees.
///
/// `camelCase` on the wire so `gateway.ts` needs no field mapper, matching the
/// `PetSummary` precedent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStateDto {
    pub current_version: String,
    pub status: UpdateStatus,
}
