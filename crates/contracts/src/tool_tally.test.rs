//! The counting rules over hand-built events. That the driver's parser and gg bridge
//! produce the events these rules count is the driver's own test.

use super::*;
use crate::gg::GgTelemetryEvent;
use serde_json::json;

fn event(kind: serde_json::Value) -> HarnessEvent {
    let mut value = kind;
    value["timestamp"] = json!("2026-10-17T00:00:00Z");
    serde_json::from_value(value).expect("a normalized event")
}

fn gg(kind: GgTelemetryKind) -> HarnessEvent {
    HarnessEvent {
        timestamp: "2026-10-17T00:00:00Z".to_string(),
        session_id: Some("sess-1".to_string()),
        kind: EventKind::Gg {
            event: Box::new(GgTelemetryEvent {
                timestamp: "2026-10-17T00:00:00Z".to_string(),
                session_id: Some("sess-1".to_string()),
                agent_id: None,
                parent_agent_id: None,
                issue_id: None,
                kind,
            }),
        },
    }
}

#[test]
fn a_codex_run_counts_what_its_commands_and_file_changes_did() {
    let events = [
        event(json!({ "type": "read", "path": "src/main.rs" })),
        event(json!({ "type": "read", "path": "src/lib.rs", "startLine": 10, "endLine": 20 })),
        event(json!({ "type": "search", "query": "TODO", "path": "src" })),
        event(json!({ "type": "list", "path": "src" })),
        event(json!({ "type": "command", "command": "cargo build", "exitCode": 0 })),
        // One per file the change touched.
        event(json!({ "type": "write", "path": "src/a.rs" })),
        event(json!({ "type": "write", "path": "src/b.rs" })),
        event(json!({ "type": "agent", "message": "done" })),
    ];
    assert_eq!(
        codex_tool_calls(&events),
        BTreeMap::from([
            ("command".to_string(), 1),
            ("list".to_string(), 1),
            ("read".to_string(), 2),
            ("search".to_string(), 1),
            ("write".to_string(), 2),
        ])
    );
}

#[test]
fn a_gg_run_counts_its_tool_calls_and_program_functions_once_each() {
    let events = [
        gg(GgTelemetryKind::ToolCall {
            name: "read_file".to_string(),
            args: json!({ "path": "src/main.rs" }),
        }),
        // The human-facing event gg's bridge records beside a carry describes the
        // same call again, so it is not counted.
        event(json!({ "type": "read", "path": "src/main.rs" })),
        gg(GgTelemetryKind::ToolCall {
            name: "Shell".to_string(),
            args: json!({ "command": "cargo test" }),
        }),
        gg(GgTelemetryKind::ToolCall {
            name: "read_file".to_string(),
            args: json!({ "path": "src/lib.rs" }),
        }),
        gg(GgTelemetryKind::ApiCall {
            operation: "files.edit_file".to_string(),
        }),
        gg(GgTelemetryKind::ApiResult {
            operation: "files.edit_file".to_string(),
            ok: true,
            failure: None,
        }),
        gg(GgTelemetryKind::AssistantMessage {
            text: "done".to_string(),
        }),
    ];
    assert_eq!(
        gg_tool_calls(&events),
        BTreeMap::from([
            ("files.edit_file".to_string(), 1),
            ("read_file".to_string(), 2),
            // Keyed lowercased, as the parser's own tally is.
            ("shell".to_string(), 1),
        ])
    );
}

#[test]
fn only_codex_and_gg_are_counted_off_their_events() {
    assert!(counted_from_events(HarnessSlug::Codex));
    assert!(counted_from_events(HarnessSlug::Gg));
    assert!(!counted_from_events(HarnessSlug::Claude));
    assert_eq!(tool_calls_from_events(HarnessSlug::Pi, &[]), None);
    assert_eq!(
        tool_calls_from_events(HarnessSlug::Gg, &[]),
        Some(BTreeMap::new())
    );
}
