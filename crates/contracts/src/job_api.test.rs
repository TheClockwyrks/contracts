use super::*;

fn summary() -> JobSummary {
    JobSummary {
        test_case_slug: "carom".to_string(),
        test_case_version: "v1.0.0".to_string(),
        variant: "base".to_string(),
        harness_slug: "claude-code".to_string(),
        model_id: "claude-opus-4".to_string(),
        engine: None,
        started_at: None,
        gg_preset: None,
    }
}

/// The console reads a notification's identity off the top level, not out of a
/// nested object: `JobSummary` is flattened into every notification.
#[test]
fn a_notification_flattens_the_run_identity() {
    let value = serde_json::to_value(Notification::completed("j1", summary(), "r1")).unwrap();
    assert_eq!(value["testCaseSlug"], "carom");
    assert_eq!(value["testCaseVersion"], "v1.0.0");
    assert_eq!(value["variant"], "base");
    assert_eq!(value["harnessSlug"], "claude-code");
    assert_eq!(value["modelId"], "claude-opus-4");
}

/// The engine rides in the flattened identity, and only when the run names one: an
/// absent field is the `none` engine, the same defaulting the launch request and the
/// run record use, so a client that reads it back gets one answer rather than two
/// spellings of the engineless run.
#[test]
fn a_run_events_identity_carries_the_engine_only_when_one_is_named() {
    let value = serde_json::to_value(RunEvent::enqueued("j1", summary())).unwrap();
    assert!(value.get("engine").is_none());

    let on_engine = JobSummary {
        engine: Some("simple-2d".to_string()),
        ..summary()
    };
    let value = serde_json::to_value(RunEvent::enqueued("j1", on_engine)).unwrap();
    assert_eq!(value["engine"], "simple-2d");
}

/// The start time rides in the flattened identity too, and only once the run has one:
/// a queued run's absent field is what the console renders as a dash, and is what stops
/// it from ticking a duration for a run that has not begun.
#[test]
fn a_run_events_identity_carries_the_start_only_once_the_run_has_started() {
    let value = serde_json::to_value(RunEvent::enqueued("j1", summary())).unwrap();
    assert!(value.get("startedAt").is_none());

    let started = JobSummary {
        started_at: Some("2026-09-06T00:02:00Z".to_string()),
        ..summary()
    };
    let value = serde_json::to_value(RunEvent::enqueued("j1", started)).unwrap();
    assert_eq!(value["startedAt"], "2026-09-06T00:02:00Z");
}

/// A completed run points the console at the record it produced; the kind is the
/// kebab-case wire token the console switches on.
#[test]
fn a_completed_notification_carries_the_record_and_no_message() {
    let value = serde_json::to_value(Notification::completed("j1", summary(), "r1")).unwrap();
    assert_eq!(value["kind"], "run-completed");
    assert_eq!(value["outcome"], "completed");
    assert_eq!(value["jobId"], "j1");
    assert_eq!(value["recordId"], "r1");
    assert!(value.get("message").is_none());
}

/// An infrastructure failure that produced no record carries the reason and no
/// record id, so the console raises an alert with no link to follow.
#[test]
fn a_failed_notification_without_a_record_omits_the_record_id() {
    let value = serde_json::to_value(Notification::failed(
        "j1",
        summary(),
        "image pull failed",
        None,
    ))
    .unwrap();
    assert_eq!(value["kind"], "run-completed");
    assert_eq!(value["outcome"], "failed");
    assert_eq!(value["message"], "image pull failed");
    assert!(value.get("recordId").is_none());
}

/// A publish failure is its own kind — the console must not treat it as a run
/// completion and prune the in-flight list — and it is keyed by the **publish** job
/// while linking to the run that could not be released, so two failed attempts at
/// the same run are two distinct alerts rather than one overwriting the other.
#[test]
fn a_publish_failed_notification_keys_on_the_publish_job_and_links_to_the_run() {
    let value = serde_json::to_value(Notification::publish_failed(
        "p1",
        summary(),
        "r1",
        "`gh repo create` failed: HTTP 503",
    ))
    .unwrap();
    assert_eq!(value["kind"], "publish-failed");
    assert_eq!(value["outcome"], "failed");
    assert_eq!(value["jobId"], "p1");
    assert_eq!(value["recordId"], "r1");
    assert_eq!(value["message"], "`gh repo create` failed: HTTP 503");
    assert_eq!(value["testCaseSlug"], "carom");
}

/// The launch shape used by the engine tests below. Every optional dimension is
/// left unset, which is what a launcher that predates a given dimension sends.
fn launch_body() -> LaunchBody {
    LaunchBody {
        test_case: "carom".to_string(),
        version: "v1.0.0".to_string(),
        variant: "base".to_string(),
        harness: HarnessSlug::Claude,
        model: "anthropic/claude-opus-4".to_string(),
        orchestrator: None,
        engine: None,
        max_runtime_seconds: None,
        auth_mode: None,
        retry_count: None,
        gg_capability_set: None,
        gg_model_windows: Default::default(),
        gg_model_providers: Default::default(),
        gg_model_modalities: Default::default(),
        gg_model_prices: Default::default(),
        model_prices: None,
    }
}

/// A launch that names no engine is the `none` default. The key is absent from the
/// wire body rather than written as null, so a body says only what the launch
/// actually chose — and the backend, which stores the body verbatim, holds exactly
/// that.
#[test]
fn a_launch_without_an_engine_omits_the_key_entirely() {
    let value = serde_json::to_value(launch_body()).expect("serialize");
    assert!(value.get("engine").is_none());

    let parsed: LaunchBody = serde_json::from_value(value).expect("deserialize");
    assert!(parsed.engine.is_none());
}

/// A selected engine travels as its bare slug; resolution against the catalogue
/// and against the case's supported set happens when the run executes.
#[test]
fn a_selected_engine_round_trips_as_its_slug() {
    let mut body = launch_body();
    body.engine = Some("simple-2d".to_string());

    let value = serde_json::to_value(&body).expect("serialize");
    assert_eq!(value["engine"], "simple-2d");

    let parsed: LaunchBody = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed.engine.as_deref(), Some("simple-2d"));
}

/// The held report travels as `storing`, the store fields as camelCase, and an
/// update that sets neither store field omits both keys rather than writing null.
#[test]
fn a_storing_report_round_trips_and_omits_absent_store_fields() {
    let update = StatusUpdate {
        state: DriverState::Storing,
        record: None,
        detail: Some("artifacts service answered 503".to_string()),
        artifacts_stored: None,
        store_fault: None,
    };
    let value = serde_json::to_value(&update).expect("serialize");
    assert_eq!(value["state"], "storing");
    assert!(value.get("artifactsStored").is_none());
    assert!(value.get("storeFault").is_none());

    let parsed: StatusUpdate = serde_json::from_value(value).expect("deserialize");
    assert!(matches!(parsed.state, DriverState::Storing));
    assert!(parsed.artifacts_stored.is_none());
    assert!(parsed.store_fault.is_none());
}

/// A terminal report's store fields round-trip under their wire names.
#[test]
fn a_terminal_report_carries_the_store_fields() {
    let value = serde_json::json!({
        "state": "succeeded",
        "artifactsStored": false,
        "storeFault": "artifacts service answered 503",
    });
    let parsed: StatusUpdate = serde_json::from_value(value).expect("deserialize");
    assert_eq!(parsed.artifacts_stored, Some(false));
    assert_eq!(
        parsed.store_fault.as_deref(),
        Some("artifacts service answered 503")
    );

    let back = serde_json::to_value(&parsed).expect("serialize");
    assert_eq!(back["artifactsStored"], false);
    assert_eq!(back["storeFault"], "artifacts service answered 503");
}

/// A job holding a record while its driver uploads reads `storing`, which is not
/// terminal: the run is not over until the terminal report lands.
#[test]
fn a_storing_job_is_read_from_the_database_and_is_not_terminal() {
    assert_eq!(JobState::from_db("storing"), JobState::Storing);
    assert!(!JobState::Storing.is_terminal());
    assert_eq!(
        serde_json::to_value(JobState::Storing).expect("serialize"),
        "storing"
    );
}

#[test]
fn a_validating_job_is_in_flight() {
    assert_eq!(JobState::from_db("validating"), JobState::Validating);
    assert!(!JobState::Validating.is_terminal());
    assert_eq!(
        serde_json::to_value(JobState::Validating).unwrap(),
        "validating"
    );
    assert_eq!(
        serde_json::to_value(DriverState::Validating).unwrap(),
        "validating"
    );
    let parsed: DriverState = serde_json::from_str(r#""validating""#).unwrap();
    assert!(matches!(parsed, DriverState::Validating));
}

#[test]
fn an_active_job_carries_its_lease_expiry_only_when_it_holds_one() {
    let mut job = ActiveJobOut {
        run_id: "j1".into(),
        summary: summary(),
        state: JobState::Running,
        lease_expires_at: None,
    };
    let json = serde_json::to_value(&job).unwrap();
    assert!(json.get("leaseExpiresAt").is_none());

    job.lease_expires_at = Some("2026-10-19T12:02:00Z".into());
    let json = serde_json::to_value(&job).unwrap();
    assert_eq!(json["leaseExpiresAt"], "2026-10-19T12:02:00Z");
    let back: ActiveJobOut = serde_json::from_value(json).unwrap();
    assert_eq!(
        back.lease_expires_at.as_deref(),
        Some("2026-10-19T12:02:00Z")
    );
}

#[test]
fn a_lease_renewal_names_its_jobs_as_job_ids() {
    let body: LeaseRenewal = serde_json::from_str(r#"{"jobIds":["a","b"]}"#).unwrap();
    assert_eq!(body.job_ids, ["a", "b"]);
    assert_eq!(
        serde_json::to_value(&body).unwrap(),
        serde_json::json!({ "jobIds": ["a", "b"] })
    );
}

#[test]
fn an_event_batch_carries_its_first_sequence_beside_its_events() {
    let json = serde_json::json!({
        "sequence": 7,
        "events": [
            { "timestamp": "2026-10-19T12:00:00Z", "type": "agent", "message": "hi" },
            {
                "timestamp": "2026-10-19T12:00:01Z",
                "sessionId": "s1",
                "type": "agent",
                "message": "there"
            }
        ]
    });
    let batch: EventBatch = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(batch.sequence, 7);
    assert_eq!(batch.events.len(), 2);
    assert_eq!(batch.events[1].session_id.as_deref(), Some("s1"));
    assert_eq!(serde_json::to_value(&batch).unwrap(), json);
    let back: EventBatch = serde_json::from_value(serde_json::to_value(&batch).unwrap()).unwrap();
    assert_eq!(back, batch);
}
