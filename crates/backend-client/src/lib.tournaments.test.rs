//! Tests for the tournament calls the arena records and resumes through.

use test_cabinet_contracts::match_play::{
    TournamentPairing, TournamentRecord, TournamentState, match_id,
};
use test_cabinet_contracts::validation::AdversarialOutcome;

use super::*;

/// Serve one canned response on a fresh local port. Answers the base URL and a
/// receiver of the whole request, head and body, so a test can prove what was sent.
async fn serve(response: String) -> (String, tokio::sync::oneshot::Receiver<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut request = Vec::new();
        let mut buf = [0u8; 8192];
        // Read the head, then as much body as its Content-Length names.
        loop {
            let read = socket.read(&mut buf).await.unwrap_or(0);
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buf[..read]);
            let text = String::from_utf8_lossy(&request);
            if let Some(head_end) = text.find("\r\n\r\n") {
                let length = text[..head_end]
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().ok())?
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + length {
                    break;
                }
            }
        }
        let _ = tx.send(String::from_utf8_lossy(&request).into_owned());
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    (format!("http://{addr}"), rx)
}

fn json_response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn summary() -> MatchSummary {
    MatchSummary {
        match_id: match_id("a", "b"),
        red_id: "a".to_string(),
        blue_id: "b".to_string(),
        winner: None,
        win_type: "time_limit".to_string(),
        outcome_for_red: AdversarialOutcome::Draw,
        red_score: 0,
        blue_score: 0,
        ticks: 5,
        red_kills: 0,
        blue_kills: 0,
        red_fuel: 1,
        blue_fuel: 1,
        replay_key: Some(match_id("a", "b")),
        detail: None,
    }
}

#[tokio::test]
async fn a_tournament_the_backend_does_not_hold_reads_as_none() {
    let body = r#"{"error":{"code":"not_found","message":"tournament `t` not found"}}"#;
    let (base, _) = serve(json_response("404 Not Found", body)).await;
    let read = HttpBackendClient::new(base)
        .recorded_tournament("t")
        .await
        .expect("a 404 is an answer");
    assert!(read.is_none());
}

#[tokio::test]
async fn a_refused_read_is_an_error_not_an_absent_tournament() {
    let body = r#"{"error":{"code":"unauthorized","message":"no identity"}}"#;
    let (base, _) = serve(json_response("401 Unauthorized", body)).await;
    let error = HttpBackendClient::new(base)
        .recorded_tournament("t")
        .await
        .expect_err("a refusal is not an absent tournament");
    assert!(error.to_string().contains("no identity"), "{error}");
}

#[tokio::test]
async fn a_recorded_tournament_is_read_with_its_state_and_pairings() {
    let recorded = RecordedTournament {
        record: TournamentRecord {
            id: "t".to_string(),
            created_at: "2026-10-26T00:00:00Z".to_string(),
            test_case_slug: "foray".to_string(),
            test_case_version: "v1".to_string(),
            variant: "base".to_string(),
            participants: Vec::new(),
            standings: Vec::new(),
            matches: vec![summary()],
        },
        state: TournamentState::Running,
        detail: None,
        pairings: vec![TournamentPairing {
            match_id: match_id("a", "b"),
            red_id: "a".to_string(),
            blue_id: "b".to_string(),
        }],
        published_at: "2026-10-26T00:00:00Z".to_string(),
    };
    let body = serde_json::to_string(&recorded).unwrap();
    let (base, request) = serve(json_response("200 OK", &body)).await;
    let read = HttpBackendClient::new(base)
        .recorded_tournament("t")
        .await
        .expect("read");
    assert_eq!(read, Some(recorded));
    assert!(request.await.unwrap().starts_with("GET /tournaments/t "));
}

#[tokio::test]
async fn a_match_is_recorded_under_its_pairing() {
    let (base, request) = serve("HTTP/1.1 204 No Content\r\n\r\n".to_string()).await;
    HttpBackendClient::new(base)
        .with_token(Some("svc".to_string()))
        .record_tournament_match("t", &summary())
        .await
        .expect("recorded");
    let request = request.await.unwrap();
    assert!(
        request.starts_with("PUT /tournaments/t/matches/a__vs__b "),
        "{request}"
    );
    assert!(request.contains("authorization: Bearer svc"), "{request}");
    let body = &request[request.find("\r\n\r\n").unwrap() + 4..];
    let sent: MatchSummary = serde_json::from_str(body).unwrap();
    assert_eq!(sent, summary());
}

#[tokio::test]
async fn a_transition_is_posted_tagged_by_its_state() {
    let (base, request) = serve("HTTP/1.1 204 No Content\r\n\r\n".to_string()).await;
    HttpBackendClient::new(base)
        .transition_tournament(
            "t",
            &TournamentTransition::Failed {
                detail: "the engine failed".to_string(),
            },
        )
        .await
        .expect("moved");
    let request = request.await.unwrap();
    assert!(
        request.starts_with("POST /tournaments/t/state "),
        "{request}"
    );
    let body = &request[request.find("\r\n\r\n").unwrap() + 4..];
    let sent: serde_json::Value = serde_json::from_str(body).unwrap();
    assert_eq!(
        sent,
        serde_json::json!({ "state": "failed", "detail": "the engine failed" })
    );
}

#[tokio::test]
async fn the_unfinished_tournaments_are_listed() {
    let (base, request) = serve(json_response("200 OK", r#"{"tournaments":[]}"#)).await;
    let listed = HttpBackendClient::new(base)
        .unfinished_tournaments()
        .await
        .expect("listed");
    assert!(listed.is_empty());
    assert!(
        request
            .await
            .unwrap()
            .starts_with("GET /tournaments/unfinished ")
    );
}
