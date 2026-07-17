use crate::domain::{
    CollectionOutcome, FailureClass, Provider, ProviderUsageSnapshot, Source, UsageWindow,
};
use time::OffsetDateTime;

#[test]
fn usage_window_validates_and_clamps_percent() {
    assert_eq!(
        UsageWindow::new(101.0, 300, None).unwrap().used_percent,
        100.0
    );
    assert_eq!(UsageWindow::new(-1.0, 300, None).unwrap().used_percent, 0.0);
    assert!(UsageWindow::new(f64::NAN, 300, None).is_err());
    assert!(UsageWindow::new(f64::INFINITY, 300, None).is_err());
    assert!(UsageWindow::new(1.0, 0, None).is_err());
}

#[test]
fn invalid_numeric_json_is_rejected() {
    let json = r#"{"used_percent":"90","window_minutes":300,"resets_at":null}"#;
    assert!(serde_json::from_str::<UsageWindow>(json).is_err());
}

#[test]
fn safe_snapshot_round_trips_without_credentials() {
    let captured_at = OffsetDateTime::parse(
        "2026-07-16T12:00:00Z",
        &time::format_description::well_known::Rfc3339,
    )
    .unwrap();
    let snapshot = ProviderUsageSnapshot {
        provider: Provider::Claude,
        plan_type: Some("pro".into()),
        session: Some(UsageWindow::new(70.0, 300, None).unwrap()),
        weekly: None,
        captured_at,
        source: Source::OauthApi,
        is_cached: false,
        revision: 3,
    };
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains("token"));
    assert!(json.contains("\"captured_at\""));
    assert!(json.contains("\"is_cached\""));
    assert!(!json.contains("capturedAt"));
    assert_eq!(
        serde_json::from_str::<ProviderUsageSnapshot>(&json).unwrap(),
        snapshot
    );
}

#[test]
fn collection_outcomes_are_typed_and_credential_free() {
    let outcome = CollectionOutcome::Failed {
        class: FailureClass::Parse,
    };
    assert_eq!(
        serde_json::to_string(&outcome).unwrap(),
        r#"{"kind":"failed","class":"parse"}"#
    );
}
