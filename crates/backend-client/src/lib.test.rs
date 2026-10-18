//! Tests for the backend client: the wire bodies it reads and writes, against
//! one-shot local listeners standing in for the backend.

use super::*;
use test_cabinet_contracts::metrics::{Cost, RunMetrics, TokenCounts};
use test_cabinet_contracts::run_record::{
    HarnessSlug, RunEnvironment, RunLinks, RunRecord, RunState, RunStatus, RunSubject, RunTooling,
};
use test_cabinet_contracts::validation::ValidationSummary;

/// Serve a single fixed `200` JSON response on a fresh local port, returning the
/// bound base URL. The one-shot server answers exactly one request, which is all
/// a single `list_runs` / `read_run` call makes.
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

/// Serve a single fixed response with a custom status line and content type, for
/// the publish-path tests (a `202` enqueue ack and an `x-ndjson` live stream),
/// returning the bound base URL.
async fn serve_once_raw(status: &str, content_type: &str, body: impl Into<String>) -> String {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let status = status.to_string();
    let content_type = content_type.to_string();
    let body = body.into();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 1024];
        let _ = socket.read(&mut buf).await;
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    format!("http://{addr}")
}

/// A minimal valid run record whose `links` are empty, so a test can prove the
/// backend's separately-served links are what end up on the resolved run.
fn sample_record(id: &str) -> RunRecord {
    RunRecord {
        id: id.to_string(),
        started_at: "2026-06-14T10:00:00Z".to_string(),
        finished_at: "2026-06-14T10:05:00Z".to_string(),
        subject: RunSubject {
            test_case_slug: "pong".to_string(),
            test_case_version: "v1.0.0".to_string(),
            test_type: test_cabinet_contracts::test_case::TestType::EndToEnd,
            variant: "base".to_string(),
            harness_slug: HarnessSlug::Claude,
            harness_version: None,
            orchestrator_slug: "one-shot".to_string(),
            engine_slug: "none".to_string(),
            engine_version: None,
            model_id: "anthropic/claude-opus-4".to_string(),
            gg_capability_set: None,
            gg_summary: None,
        },
        tooling: RunTooling {
            test_cabinet_commit: None,
        },
        environment: RunEnvironment {
            os: "Debian GNU/Linux 12".to_string(),
            container_image: "testcabinet.azurecr.io/test-cabinet-claude@sha256:abc".to_string(),
            node_version: None,
            auth_mode: test_cabinet_contracts::run_record::AuthMode::ApiKey,
        },
        metrics: RunMetrics {
            run_time_seconds: 300.0,
            tokens: TokenCounts {
                uncached_input: Some(1),
                cached_input: Some(0),
                output: Some(1),
                reasoning: Some(0),
            },
            cost: Cost {
                comparable: Some(0.0),
                actual: Some(0.0),
            },
            ..RunMetrics::default()
        },
        validation: ValidationSummary {
            debug_scripts: Vec::new(),
            loaded: true,
            requirements: Vec::new(),
            detail: None,
            install: None,
            build: None,
            checks: vec![],
            proofs: vec![],
            asset: None,
            voxel: None,
            ui: None,
            material: None,
            particle: None,
            audio: None,
            adversarial: None,
            performance: None,
        },
        links: RunLinks::default(),
        status: RunStatus {
            state: RunState::Completed,
            detail: None,
        },
        game_jam_readme: None,
        tool_calls: Default::default(),
        game_jam_prior_entries: Vec::new(),
        seed_commit: None,
        code_analysis: None,
        toolchain: None,
        showcase: None,
    }
}

#[tokio::test]
async fn list_runs_parses_page_cursor_links_and_reviews() {
    // The backend names the cursor `nextBefore`, serves the reviews array (with
    // reviewer identity), and serves the resolved links separately from the blob.
    let record = serde_json::to_value(sample_record("run-1")).expect("serialize");
    let body = serde_json::json!({
        "runs": [{
            "record": record,
            "published": true,
            "reviews": [{
                "reviewerId": "u1",
                "reviewer": "Ada L.",
                "username": "ada",
                "ratings": [{ "domain": "gameplay", "rating": "great" }],
                "writeup": "Plays well.",
                "checklist": [],
                "reviewedAt": "2026-06-14T11:00:00Z",
            }],
            "links": {
                "sourceRepo": "https://example.com/repo",
                "playableBuild": "https://abc.pages.dev",
            },
        }],
        "nextBefore": "2026-06-14T10:00:00Z",
    });
    let base = serve_once(body.to_string()).await;

    let page = HttpBackendClient::new(base)
        .list_runs(None, Some(10))
        .await
        .expect("list runs");

    assert_eq!(page.next_before.as_deref(), Some("2026-06-14T10:00:00Z"));
    assert_eq!(page.runs.len(), 1);
    let run = &page.runs[0];
    assert_eq!(run.record.id, "run-1");
    assert!(run.published);
    assert_eq!(run.reviews.len(), 1);
    assert_eq!(run.reviews[0].reviewer, "Ada L.");
    assert_eq!(
        run.reviews[0].ratings,
        vec![test_cabinet_contracts::review::DomainRating {
            domain: "gameplay".to_string(),
            rating: test_cabinet_contracts::review::Rating::Great,
        }]
    );
    // The separately-served links win and are merged onto the record blob (whose
    // own links were empty), so both views agree.
    assert_eq!(
        run.links.source_repo.as_deref(),
        Some("https://example.com/repo")
    );
    assert_eq!(
        run.record.links.playable_build.as_deref(),
        Some("https://abc.pages.dev")
    );
}

#[tokio::test]
async fn read_run_parses_a_single_stored_run() {
    let record = serde_json::to_value(sample_record("run-9")).expect("serialize");
    let body = serde_json::json!({
        "record": record,
        "published": true,
        "reviews": [{
            "reviewerId": "u2",
            "reviewer": "Grace H.",
            "username": "grace",
            "ratings": [{ "domain": "gameplay", "rating": "scuffed" }],
            "writeup": "Janky.",
            "checklist": [],
            "reviewedAt": "2026-06-14T12:00:00Z",
        }],
        "links": { "sourceRepo": null, "playableBuild": null },
    });
    let base = serve_once(body.to_string()).await;

    let run = HttpBackendClient::new(base)
        .read_run("run-9")
        .await
        .expect("read run");

    assert_eq!(run.record.id, "run-9");
    assert_eq!(run.reviews.len(), 1);
    assert_eq!(
        run.reviews[0].ratings,
        vec![test_cabinet_contracts::review::DomainRating {
            domain: "gameplay".to_string(),
            rating: test_cabinet_contracts::review::Rating::Scuffed,
        }]
    );
    assert_eq!(run.reviews[0].writeup, "Janky.");
    assert!(run.links.source_repo.is_none());
}

/// Serve one `201 Created` on a fresh local port and hand back the base URL plus
/// a receiver that yields the request's body once it arrives — for proving what
/// a client call puts on the wire, not just what it parses off it.
async fn capture_once() -> (String, tokio::sync::oneshot::Receiver<String>) {
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
        // Read until the headers announce a body length that has fully arrived.
        loop {
            let n = socket.read(&mut buf).await.unwrap_or(0);
            if n == 0 {
                break;
            }
            raw.extend_from_slice(&buf[..n]);
            let text = String::from_utf8_lossy(&raw);
            if let Some((head, body)) = text.split_once("\r\n\r\n") {
                let length = head
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(name, value)| {
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().ok())
                                .flatten()
                        })
                    })
                    .unwrap_or(0);
                if body.len() >= length {
                    break;
                }
            }
        }
        let text = String::from_utf8_lossy(&raw).into_owned();
        let body = text
            .split_once("\r\n\r\n")
            .map(|(_, body)| body.to_string())
            .unwrap_or_default();
        let _ = tx.send(body);
        let response = "HTTP/1.1 201 Created\r\nContent-Length: 0\r\n\r\n";
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    (format!("http://{addr}"), rx)
}

#[tokio::test]
async fn submit_review_posts_the_writeups_aesthetic_beside_its_ratings_and_checklist() {
    // The writeup file's run-wide `aesthetic` tier and its `review.<id>` override
    // lines are a validator-rated run's whole review channel, so the wire body has
    // to carry both — a review that posts `ratings: []` alone is refused by the
    // backend as empty.
    use test_cabinet_contracts::review::{AestheticRating, ReviewVerdict, VerdictStatus, Writeup};
    let (base, body) = capture_once().await;
    let writeup = Writeup {
        ratings: vec![],
        aesthetic: Some(AestheticRating::Amazing),
        body: "Gorgeous.".to_string(),
        checklist: vec![ReviewVerdict {
            id: "serve".to_string(),
            status: VerdictStatus::Pass,
            note: Some("works when driven by hand".to_string()),
        }],
    };

    HttpBackendClient::new(base)
        .submit_review("run-42", &writeup)
        .await
        .expect("submit review");

    let posted: serde_json::Value =
        serde_json::from_str(&body.await.expect("request body")).expect("json body");
    assert_eq!(
        posted,
        serde_json::json!({
            "ratings": [],
            "aesthetic": "amazing",
            "writeup": "Gorgeous.",
            "checklist": [
                { "id": "serve", "status": "pass", "note": "works when driven by hand" },
            ],
        })
    );
}

#[tokio::test]
async fn submit_review_omits_the_aesthetic_from_a_legacy_writeup() {
    // A legacy writeup (functional ratings only) posts exactly what it always did.
    use test_cabinet_contracts::review::{DomainRating, Rating, Writeup};
    let (base, body) = capture_once().await;
    let writeup = Writeup {
        ratings: vec![DomainRating {
            domain: "gameplay".to_string(),
            rating: Rating::Great,
        }],
        aesthetic: None,
        body: "Solid.".to_string(),
        checklist: vec![],
    };

    HttpBackendClient::new(base)
        .submit_review("run-42", &writeup)
        .await
        .expect("submit review");

    let posted: serde_json::Value =
        serde_json::from_str(&body.await.expect("request body")).expect("json body");
    assert!(posted.get("aesthetic").is_none());
    assert_eq!(posted["ratings"][0]["rating"], "great");
}

#[tokio::test]
async fn publish_run_parses_the_async_enqueue_ack() {
    // Publishing is async: the backend answers `202` with the enqueued publish
    // job's id and the live URL to observe it on.
    let body = serde_json::json!({
        "publishJobId": "pj-42",
        "liveUrl": "/publish-jobs/pj-42/live",
    });
    let base = serve_once_raw("202 Accepted", "application/json", body.to_string()).await;

    let ack = HttpBackendClient::new(base)
        .publish_run("run-42")
        .await
        .expect("publish run");

    assert_eq!(ack.publish_job_id, "pj-42");
    assert_eq!(ack.live_url, "/publish-jobs/pj-42/live");
}

#[tokio::test]
async fn watch_publish_job_streams_progress_then_the_terminal_result() {
    // The live stream is NDJSON, each line tagged with a `type` discriminator:
    // `progress` lines, then the terminal `result`.
    let stream = "\
{\"type\":\"progress\",\"message\":\"creating repository\"}\n\
{\"type\":\"progress\",\"message\":\"deploying build\"}\n\
{\"type\":\"result\",\"state\":\"succeeded\",\"sourceRepo\":\"https://example.com/repo\",\"playableBuild\":\"https://abc.pages.dev\"}\n";
    let base = serve_once_raw("200 OK", "application/x-ndjson", stream).await;

    let client = HttpBackendClient::new(base);
    let mut items = Vec::new();
    let mut on_item = |item: PublishLiveItem| items.push(item);
    client
        .watch_publish_job("pj-42", &mut on_item)
        .await
        .expect("watch publish job");

    assert_eq!(items.len(), 3);
    assert_eq!(
        items[0],
        PublishLiveItem::Progress(test_cabinet_contracts::publish_job_api::PublishProgress {
            message: "creating repository".to_string(),
        })
    );
    assert_eq!(
        items[2],
        PublishLiveItem::Result(PublishResult {
            state: test_cabinet_contracts::publish_job_api::PublishState::Succeeded,
            source_repo: Some("https://example.com/repo".to_string()),
            playable_build: Some("https://abc.pages.dev".to_string()),
            detail: None,
        })
    );
}

#[test]
fn emit_publish_line_surfaces_a_malformed_line_as_a_failure_result() {
    // A line that is neither a tagged progress nor result item ends the watch with
    // a legible failure rather than being silently dropped.
    let mut items = Vec::new();
    let mut on_item = |item: PublishLiveItem| items.push(item);
    emit_publish_line("{\"type\":\"mystery\"}", &mut on_item);
    emit_publish_line("", &mut on_item); // empty lines are skipped

    assert_eq!(items.len(), 1);
    match &items[0] {
        PublishLiveItem::Result(result) => {
            assert_eq!(
                result.state,
                test_cabinet_contracts::publish_job_api::PublishState::Failed
            );
            assert!(result.detail.as_deref().unwrap().contains("mystery"));
        }
        other => panic!("expected a failure result, got {other:?}"),
    }
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
            &["carom@v1.0.0".to_string()],
            true,
            IngestMode::Absent,
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

#[tokio::test]
async fn a_whole_checkout_scan_names_no_targets() {
    let (base, request) =
        serve_ingest_feed(&[r#"{"event":"done","total":0,"ingested":0,"skipped":0}"#]).await;
    let mut on_progress = |_: &IngestProgress| {};
    HttpBackendClient::new(base)
        .ingest(&[], false, IngestMode::Absent, &mut on_progress)
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
        .ingest(&[], false, IngestMode::Changed, &mut on_progress)
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
        .ingest(&[], true, IngestMode::Absent, &mut on_progress)
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
        .ingest(&[], false, IngestMode::Absent, &mut on_progress)
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
        .ingest(&[], false, IngestMode::Absent, &mut on_progress)
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
