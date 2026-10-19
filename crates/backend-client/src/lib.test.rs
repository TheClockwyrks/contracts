//! Tests for the backend client: the wire bodies it reads and writes, against
//! one-shot local listeners standing in for the backend.

use super::*;

/// Serve a single fixed `200` JSON response on a fresh local port, returning the
/// bound base URL. The one-shot server answers exactly one request, which is all
/// a single read makes.
async fn serve_once(json: impl Into<String>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let json = json.into();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        // Drain the request headers (we don't route on them — one endpoint).
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            json.len(),
            json
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn resolve_version_carries_voxel_volume_and_rig() {
    // The resolved-version wire body carries a voxel case's `[voxel]` bounding
    // volume and a voxel-animation case's required `[model]` rig through to the
    // reconstructed `TestCaseVersion`. These are the twin of the backend's
    // `VersionResponse`; if `into_version` drops them (it once hardcoded both to
    // `None`), the runner seeds a voxel run with no volume and seeding fails with
    // "voxel case has no [voxel]". A voxel-animation body exercises both fields.
    let body = serde_json::json!({
        "slug": "ironward",
        "version": "v1.0.0",
        "name": "Ironward",
        "difficulty": "medium",
        "tags": ["voxel"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 3600,
        "engines": [{ "slug": "none" }],
        "testType": "asset-generation",
        "assetKind": "voxel-animation",
        "tool": { "binary": "voxel-anim", "preview": "{part}.png" },
        "output": { "actions": "{part}.actions.json" },
        "voxel": { "width": 24, "height": 12, "depth": 24, "background": "transparent" },
        "model": {
            "parts": [
                { "name": "chassis", "pivot": [12, 0, 12] },
                { "name": "turret", "parent": "chassis", "pivot": [12, 6, 12] }
            ],
            "joints": [{
                "name": "turret_yaw",
                "part": "turret",
                "kind": "rotation",
                "axis": "y",
                "pivot": [12, 6, 12],
                "min": -3.0,
                "max": 3.0,
                "rest": 0.0,
                "drive": "caller"
            }]
        },
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("ironward", "v1.0.0")
        .await
        .expect("resolve version");

    let voxel = version.voxel.expect("voxel volume survives the wire");
    assert_eq!((voxel.width, voxel.height, voxel.depth), (24, 12, 24));
    let model = version.model.expect("required rig survives the wire");
    assert_eq!(model.parts.len(), 2);
    assert_eq!(model.joints.len(), 1);
    assert_eq!(model.joints[0].name, "turret_yaw");
}

#[tokio::test]
async fn resolve_version_carries_instrumentation_and_item_validation() {
    // The resolved-version wire body carries the case's `[instrumentation]` handle and
    // each auto-validated review item's `validation` (script key + outputs) through to
    // the reconstructed `TestCaseVersion`. These are reporter-side and host-only, but
    // the validator drives them; if `into_version` drops them (it once hardcoded both
    // to `None`), auto-validation silently never runs in the deployed flow. The
    // script's `source_path` arrives as the store-relative key, which
    // `materialize_version` later roots on disk.
    let body = serde_json::json!({
        "slug": "carom",
        "version": "v2.0.0",
        "name": "Carom",
        "difficulty": "easy",
        "tags": ["end-to-end"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 1800,
        "engines": [{ "slug": "none" }],
        "testType": "end-to-end",
        "instrumentation": { "handle": "__carom" },
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "commonReviewItems": [{
            "id": "ball-spin",
            "title": "Ball spin",
            "text": "The ball spins.",
            "weight": 1,
            "validation": {
                "script": "validation/ball-spin.mjs",
                "outputs": [
                    { "id": "spin", "name": "Spin", "kind": "image" }
                ]
            }
        }],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("carom", "v2.0.0")
        .await
        .expect("resolve version");

    assert_eq!(
        version.instrumentation.as_ref().map(|i| i.handle.as_str()),
        Some("__carom"),
        "the `[instrumentation]` handle survives the wire"
    );
    let validation = version.common_review_items[0]
        .validation
        .as_ref()
        .expect("the item's validation driver survives the wire");
    assert_eq!(
        validation.script.as_deref(),
        Some(std::path::Path::new("validation/ball-spin.mjs")),
        "the script is the store-relative key until materialization roots it"
    );
    assert_eq!(validation.script_rel, "validation/ball-spin.mjs");
    assert_eq!(validation.outputs.len(), 1);
    assert_eq!(validation.outputs[0].id, "spin");
    assert_eq!(validation.outputs[0].name, "Spin");
    assert_eq!(
        validation.outputs[0].kind,
        test_cabinet_contracts::test_case::MediaKind::Image
    );
}

#[tokio::test]
async fn resolve_version_carries_the_engines_the_case_supports() {
    // The engine dimension is gated on what the case declares, and a run resolved
    // over HTTP is held to that gate in the driver pod. If the client synthesizes a
    // support set instead of reading the served one, every engine-backed run
    // enqueued through the backend dies on "unsupported engine" no matter what the
    // case declares. Both spellings arrive: a bare engine and a pinned one.
    let body = serde_json::json!({
        "slug": "carom",
        "version": "v3.0.0",
        "name": "Carom",
        "difficulty": "easy",
        "tags": ["end-to-end"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 1800,
        "engines": [
            { "slug": "none" },
            { "slug": "simple-2d", "minVersion": "1.0.0" }
        ],
        "testType": "end-to-end",
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("carom", "v3.0.0")
        .await
        .expect("resolve version");

    assert_eq!(version.engine_slugs(), vec!["none", "simple-2d"]);
    let pinned = version.engine_support("simple-2d").expect("declared");
    assert_eq!(pinned.min_version, Some(semver::Version::new(1, 0, 0)));
    assert_eq!(pinned.max_version, None);
    // The bare spelling constrains nothing, so the gate never needs a version for it.
    assert!(
        !version
            .engine_support("none")
            .expect("declared")
            .is_bounded()
    );
}

#[tokio::test]
async fn resolve_version_decodes_object_shaped_packages() {
    // The backend serves each shipped package as a `{ name, description }` object
    // (the description is gallery-only). If the client decodes `packages` as bare
    // strings, the whole body fails with "error decoding response body" and every
    // case that declares `packages` — all full-stack cases do — refuses to resolve.
    // Keep the name, drop the description.
    let body = serde_json::json!({
        "slug": "junction",
        "version": "v1.0.0",
        "name": "Junction",
        "difficulty": "hard",
        "tags": ["simulation"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 3600,
        "engines": [{ "slug": "none" }],
        "testType": "full-stack",
        "packages": [
            { "name": "@clockwyrks/particle-runtime", "description": "produced-effect runtime" }
        ],
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("junction", "v1.0.0")
        .await
        .expect("resolve version");

    assert_eq!(version.packages, vec!["@clockwyrks/particle-runtime"]);
}

#[tokio::test]
async fn resolve_version_carries_the_full_stack_asset_dimension_across_the_wire() {
    // A dispatcher-scheduled run is always backend-driven: the driver materializes its
    // version through `resolve_version` and core resolves the run image off what comes
    // back. So `assetDimension` has to survive the wire, or a 3D full-stack case
    // silently resolves the 2D image and the run dies inside the container on the first
    // `voxel` invocation — a failure a local `tcab run` (which resolves off the on-disk
    // manifest) would never reproduce.
    let body = serde_json::json!({
        "slug": "gantry",
        "version": "v1.0.0",
        "name": "Gantry",
        "difficulty": "medium",
        "tags": ["simulation"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 3600,
        "engines": [{ "slug": "none" }],
        "testType": "full-stack",
        "assetDimension": "3d",
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("gantry", "v1.0.0")
        .await
        .expect("resolve version");

    assert_eq!(
        version.asset_dimension,
        test_cabinet_contracts::test_case::AssetDimension::ThreeD
    );
}

#[tokio::test]
async fn resolve_version_defaults_the_asset_dimension_when_the_backend_omits_it() {
    // A store written before the discriminator existed serves no `assetDimension` at
    // all. That must resolve to the 2D image rather than failing the whole body, which
    // is what an absent `#[serde(default)]` would do — and would take every case with
    // it, not just full-stack ones.
    let body = serde_json::json!({
        "slug": "junction",
        "version": "v1.0.0",
        "name": "Junction",
        "difficulty": "hard",
        "tags": ["simulation"],
        "summary": null,
        "description": null,
        "maxRuntimeSeconds": 3600,
        "engines": [{ "slug": "none" }],
        "testType": "full-stack",
        "promptTemplate": "",
        "commonSpecs": [],
        "assets": [],
        "variants": [],
        "commonReferences": [],
        "checks": []
    });
    let base = serve_once(body.to_string()).await;

    let version = HttpBackendClient::new(base)
        .resolve_version("junction", "v1.0.0")
        .await
        .expect("resolve version");

    assert_eq!(
        version.asset_dimension,
        test_cabinet_contracts::test_case::AssetDimension::TwoD
    );
}

// --- Ingest -----------------------------------------------------------------

/// Serve one NDJSON progress feed and hand back the bound base URL, alongside a
/// handle the test reads the request body out of so it can prove what was sent.
async fn serve_ingest_feed(lines: &[&str]) -> (String, std::sync::Arc<std::sync::Mutex<String>>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let body = lines.join("\n") + "\n";
    let request = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let seen = request.clone();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 4096];
        let read = socket.read(&mut buf).await.unwrap_or(0);
        *seen.lock().expect("the request is recorded") =
            String::from_utf8_lossy(&buf[..read]).to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    (format!("http://{addr}"), request)
}

#[tokio::test]
async fn ingest_reports_every_line_and_closes_with_the_summary() {
    let (base, request) = serve_ingest_feed(&[
        r#"{"event":"start","total":2}"#,
        r#"{"event":"version","index":1,"total":2,"slug":"carom-end-to-end","version":"v1.0.0","ingested":true,"renderedReferences":0}"#,
        r#"{"event":"version","index":2,"total":2,"slug":"carom-ball","version":"v1.0.0","ingested":false,"renderedReferences":0}"#,
        r#"{"event":"done","total":2,"ingested":1,"skipped":1}"#,
    ])
    .await;

    let mut seen: Vec<IngestProgress> = Vec::new();
    let mut on_progress = |progress: &IngestProgress| seen.push(progress.clone());
    let summary = HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&["carom@v1.0.0".to_string()], true, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");

    assert_eq!(
        summary,
        IngestSummary {
            total: 2,
            ingested: 1,
            skipped: 1
        }
    );
    assert_eq!(seen.len(), 4);
    assert!(matches!(seen[0], IngestProgress::Start { total: 2 }));
    assert!(matches!(
        &seen[1],
        IngestProgress::Version { slug, ingested: true, .. } if slug == "carom-end-to-end"
    ));

    // The targets and the force flag reach the endpoint in the shape it accepts, and
    // the streamed feed is asked for.
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        request.contains("application/x-ndjson"),
        "unexpected request: {request}"
    );
    assert!(
        request.contains(r#""testCases":["carom@v1.0.0"]"#),
        "unexpected request: {request}"
    );
    assert!(
        request.contains(r#""force":true"#),
        "unexpected request: {request}"
    );
}

/// Whether a recorded request head carries `Authorization: Bearer <token>`.
fn presents_bearer(request: &str, token: &str) -> bool {
    let expected = format!("authorization: bearer {token}");
    request
        .lines()
        .any(|line| line.trim().to_ascii_lowercase() == expected)
}

#[tokio::test]
async fn ingest_presents_the_clients_token() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .with_token(Some("svc-token".to_string()))
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");

    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        presents_bearer(&request, "svc-token"),
        "the ingest presents the service token: {request}"
    );
}

#[tokio::test]
async fn a_read_presents_the_clients_token() {
    // Reads need an identity too, so the token rides every request, not only the
    // mutating ones.
    let (base, request) = serve_ingest_feed(&[r#"{"versions":["v1.0.0"]}"#]).await;
    let versions = HttpBackendClient::new(base)
        .with_token(Some("job-token".to_string()))
        .versions("carom")
        .await
        .expect("the read succeeds");

    assert_eq!(versions, vec!["v1.0.0".to_string()]);
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        request.starts_with("GET /test-cases/carom/versions "),
        "{request}"
    );
    assert!(
        presents_bearer(&request, "job-token"),
        "the read presents the token: {request}"
    );
}

#[tokio::test]
async fn a_client_without_a_token_sends_no_authorization() {
    let (base, request) = serve_ingest_feed(&[r#"{"versions":[]}"#]).await;
    HttpBackendClient::new(base)
        .with_token(Some(String::new()))
        .versions("carom")
        .await
        .expect("the read succeeds");

    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        !request.to_ascii_lowercase().contains("authorization:"),
        "an empty token sends no header: {request}"
    );
}

/// The value a recorded request head carries for `name`, matched case-insensitively.
fn header_value(request: &str, name: &str) -> Option<String> {
    let prefix = format!("{}:", name.to_ascii_lowercase());
    request.lines().find_map(|line| {
        line.to_ascii_lowercase()
            .starts_with(&prefix)
            .then(|| line[prefix.len()..].trim().to_string())
    })
}

#[tokio::test]
async fn every_request_presents_the_access_service_token() {
    let (base, request) = serve_ingest_feed(&[r#"{"versions":[]}"#]).await;
    HttpBackendClient::new(base)
        .with_access([AccessCredential::ServiceToken {
            client_id: "client.access".to_string(),
            client_secret: "s3cret".to_string(),
        }])
        .versions("carom")
        .await
        .expect("the read succeeds");

    let request = request.lock().expect("the request is recorded").clone();
    assert_eq!(
        header_value(&request, ACCESS_CLIENT_ID_HEADER).as_deref(),
        Some("client.access"),
        "{request}"
    );
    assert_eq!(
        header_value(&request, ACCESS_CLIENT_SECRET_HEADER).as_deref(),
        Some("s3cret"),
        "{request}"
    );
    assert!(
        !request.to_ascii_lowercase().contains("authorization:"),
        "an Access credential is no bearer token: {request}"
    );
}

#[tokio::test]
async fn an_ingest_presents_the_user_token_beside_the_bearer() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .with_token(Some("svc-token".to_string()))
        .with_access([AccessCredential::UserToken("user-jwt".to_string())])
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");

    let request = request.lock().expect("the request is recorded").clone();
    assert_eq!(
        header_value(&request, ACCESS_TOKEN_HEADER).as_deref(),
        Some("user-jwt"),
        "{request}"
    );
    assert!(presents_bearer(&request, "svc-token"), "{request}");
}

#[tokio::test]
async fn the_local_stack_is_sent_the_local_identity() {
    let (base, request) = serve_ingest_feed(&[r#"{"versions":[]}"#]).await;
    HttpBackendClient::new(base)
        .with_access([AccessCredential::LocalIdentity("dev@local".to_string())])
        .versions("carom")
        .await
        .expect("the read succeeds");

    let request = request.lock().expect("the request is recorded").clone();
    assert_eq!(
        header_value(&request, LOCAL_IDENTITY_HEADER).as_deref(),
        Some("dev@local"),
        "{request}"
    );
}

#[test]
fn debug_prints_no_credential() {
    let client = HttpBackendClient::new("http://backend")
        .with_token(Some("svc-token".to_string()))
        .with_access([
            AccessCredential::UserToken("user-jwt".to_string()),
            AccessCredential::ServiceToken {
                client_id: "client.access".to_string(),
                client_secret: "s3cret".to_string(),
            },
        ]);
    let printed = format!("{client:?}");
    for secret in ["svc-token", "user-jwt", "client.access", "s3cret"] {
        assert!(!printed.contains(secret), "{printed}");
    }
    assert!(printed.contains("http://backend"), "{printed}");
}

#[tokio::test]
async fn a_refresh_is_asked_for_and_its_notice_reported() {
    let notice = "ingest: test suites update failed; ingesting without them";
    let notice_line = format!(r#"{{"event":"notice","message":"{notice}"}}"#);
    let (base, request) = serve_ingest_feed(&[
        &notice_line,
        r#"{"event":"start","total":0}"#,
        r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#,
    ])
    .await;
    let mut seen: Vec<IngestProgress> = Vec::new();
    let mut on_progress = |progress: &IngestProgress| seen.push(progress.clone());
    let mut body = ingest_body(&[], false, IngestMode::Changed);
    body.refresh = true;
    HttpBackendClient::new(base)
        .ingest(&body, &mut on_progress)
        .await
        .expect("a notice does not fail the scan");

    assert_eq!(
        seen.first(),
        Some(&IngestProgress::Notice {
            message: notice.to_string()
        })
    );
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        request.contains(r#""refresh":true"#),
        "unexpected request: {request}"
    );
}

#[tokio::test]
async fn a_scan_that_asks_no_refresh_sends_none() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        !request.contains("refresh"),
        "unexpected request: {request}"
    );
}

#[tokio::test]
async fn a_whole_checkout_scan_names_no_targets() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        !request.contains("testCases"),
        "unexpected request: {request}"
    );
}

#[tokio::test]
async fn a_changed_scan_names_its_mode_and_reads_each_skip_reason() {
    let (base, request) = serve_ingest_feed(&[
        r#"{"event":"start","total":1}"#,
        r#"{"event":"version","index":1,"total":1,"slug":"carom","version":"v1.0.0","ingested":false,"renderedReferences":0,"reason":"unchanged"}"#,
        r#"{"event":"done","total":1,"ingested":0,"skipped":1}"#,
    ])
    .await;
    let mut seen: Vec<IngestProgress> = Vec::new();
    let mut on_progress = |progress: &IngestProgress| seen.push(progress.clone());
    HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], false, IngestMode::Changed),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");

    assert!(matches!(
        &seen[1],
        IngestProgress::Version { ingested: false, reason: Some(reason), .. } if reason == "unchanged"
    ));
    let request = request.lock().expect("the request is recorded").clone();
    assert!(
        request.contains(r#""mode":"changed""#),
        "unexpected request: {request}"
    );
}

#[tokio::test]
async fn a_scan_without_a_mode_sends_none() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], true, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect("the scan completes");
    let request = request.lock().expect("the request is recorded").clone();
    assert!(!request.contains("mode"), "unexpected request: {request}");
}

#[tokio::test]
async fn a_streamed_error_line_fails_the_scan() {
    let (base, _) = serve_ingest_feed(&[
        r#"{"event":"start","total":1}"#,
        r#"{"event":"error","message":"could not render reference `title`"}"#,
    ])
    .await;
    let mut on_progress = |_: &IngestProgress| {};
    let err = HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect_err("a streamed error fails the scan");
    assert!(
        format!("{err}").contains("could not render reference `title`"),
        "unexpected error: {err}"
    );
}

#[tokio::test]
async fn a_stream_that_never_closes_is_a_failure() {
    // A truncated feed says nothing about whether the scan finished, so it is a
    // failure rather than a silent success.
    let (base, _) = serve_ingest_feed(&[r#"{"event":"start","total":1}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    let err = HttpBackendClient::new(base)
        .ingest(
            &ingest_body(&[], false, IngestMode::Absent),
            &mut on_progress,
        )
        .await
        .expect_err("a truncated feed fails the scan");
    assert!(
        format!("{err}").contains("did not finish"),
        "unexpected error: {err}"
    );
}

/// Serve one answer to a reference build upload on a fresh local port, handing
/// back the base URL and the request line and headers it received.
async fn serve_upload(
    status: &'static str,
    body: &'static str,
) -> (String, tokio::sync::oneshot::Receiver<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut raw = Vec::new();
        let mut buf = [0u8; 4096];
        while !String::from_utf8_lossy(&raw).contains("\r\n\r\n") {
            let n = socket.read(&mut buf).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        let _ = tx.send(text.split("\r\n\r\n").next().unwrap_or_default().to_owned());
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    (format!("http://{addr}"), rx)
}

#[tokio::test]
async fn a_reference_build_is_put_as_a_gzip_and_its_url_read_back() {
    let (base, head) = serve_upload(
        "201 Created",
        r#"{"engine":"none","url":"/suites/carom/versions/v1.0.0/reference-builds/none/"}"#,
    )
    .await;
    let upload = HttpBackendClient::new(base)
        .upload_suite_reference_build("carom", "v1.0.0", "none", vec![0x1f, 0x8b, 0, 0])
        .await
        .expect("the upload is stored");
    assert_eq!(
        upload,
        ReferenceBuildUpload {
            engine: "none".into(),
            url: "/suites/carom/versions/v1.0.0/reference-builds/none/".into(),
        }
    );
    let head = head
        .await
        .expect("the request arrived")
        .to_ascii_lowercase();
    assert!(
        head.starts_with("put /suites/carom/versions/v1.0.0/reference-builds/none "),
        "{head}"
    );
    assert!(head.contains("content-type: application/gzip"), "{head}");
}

#[tokio::test]
async fn a_refused_reference_build_upload_names_the_backends_reason() {
    let (base, _) = serve_upload(
        "422 Unprocessable Entity",
        r#"{"error":{"code":"unprocessable","message":"engine `phaser` is not declared"}}"#,
    )
    .await;
    let error = HttpBackendClient::new(base)
        .upload_suite_reference_build("carom", "v1.0.0", "phaser", vec![1, 2, 3])
        .await
        .expect_err("the upload is refused");
    assert!(
        error
            .to_string()
            .contains("engine `phaser` is not declared"),
        "{error}"
    );
}
