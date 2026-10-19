//! Tests for how the client meets an environment's edge: a refusal is reported as
//! one, a redirect to the sign-in page is never followed, and a launch whose
//! caller is identified by an Access credential sends no empty bearer.

use super::*;
use test_cabinet_contracts::run_record::HarnessSlug;

/// Serve one canned response on a fresh local port. Answers the base URL and a
/// receiver of the request head, so a test can prove what was sent.
async fn serve(response: String) -> (String, tokio::sync::oneshot::Receiver<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let (tx, rx) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");
        let mut buf = [0u8; 8192];
        let read = socket.read(&mut buf).await.unwrap_or(0);
        let _ = tx.send(String::from_utf8_lossy(&buf[..read]).into_owned());
        let _ = socket.write_all(response.as_bytes()).await;
        let _ = socket.flush().await;
    });
    (format!("http://{addr}"), rx)
}

/// A response with `status`, extra header lines and an HTML body.
fn html_response(status: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\n{headers}Content-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
}

/// A listener that counts the connections it accepts: the target a followed
/// redirect would land on.
async fn sign_in_page() -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            drop(socket);
        }
    });
    (format!("http://{addr}"), hits)
}

#[tokio::test]
async fn an_ingest_redirected_to_the_sign_in_page_is_the_edges_refusal() {
    let (login, hits) = sign_in_page().await;
    let location = format!("{login}/cdn-cgi/access/login/console.testcabinet.ai?kid=abc&meta=xyz");
    let (base, _) = serve(html_response(
        "302 Found",
        &format!("Location: {location}\r\n"),
        "<html>sign in</html>",
    ))
    .await;

    let error = HttpBackendClient::new(base)
        .ingest(&ingest_body(&[], false, IngestMode::Absent), &mut |_| {})
        .await
        .expect_err("the edge refused the ingest");

    let message = error.to_string();
    assert!(message.contains("refused by the edge"), "{message}");
    assert!(message.contains("302"), "{message}");
    assert!(
        message.contains(&format!(
            "`{login}/cdn-cgi/access/login/console.testcabinet.ai`"
        )),
        "{message}"
    );
    assert!(
        !message.contains("meta=xyz"),
        "the query is left out: {message}"
    );
    assert!(!message.contains("<html>"), "{message}");
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "the redirect is not followed"
    );
}

#[tokio::test]
async fn a_read_redirected_to_the_sign_in_page_is_the_edges_refusal() {
    let (base, _) = serve(html_response(
        "302 Found",
        "Location: https://team.cloudflareaccess.com/cdn-cgi/access/login\r\n",
        "",
    ))
    .await;

    let error = HttpBackendClient::new(base)
        .versions("carom")
        .await
        .expect_err("the edge refused the read");
    assert!(error.to_string().contains("refused by the edge"), "{error}");
}

#[tokio::test]
async fn an_edge_forbidden_page_is_its_refusal_not_its_markup() {
    let (base, _) = serve(html_response(
        "403 Forbidden",
        "",
        "<!DOCTYPE html><html><body>Forbidden. You don't have permission</body></html>",
    ))
    .await;

    let error = HttpBackendClient::new(base)
        .ingest(&ingest_body(&[], false, IngestMode::Changed), &mut |_| {})
        .await
        .expect_err("the edge refused the ingest");
    let message = error.to_string();
    assert!(
        message.contains("refused by the edge (403 Forbidden)"),
        "{message}"
    );
    assert!(!message.contains("DOCTYPE"), "{message}");
}

#[tokio::test]
async fn the_backends_own_refusal_keeps_its_message() {
    let body = r#"{"error":{"code":"forbidden","message":"this route takes a service caller"}}"#;
    let (base, _) = serve(format!(
        "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    ))
    .await;

    let error = HttpBackendClient::new(base)
        .ingest(&ingest_body(&[], false, IngestMode::Absent), &mut |_| {})
        .await
        .expect_err("the backend refused the ingest");
    let message = error.to_string();
    assert!(
        message.contains("this route takes a service caller"),
        "{message}"
    );
    assert!(!message.contains("refused by the edge"), "{message}");
}

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

#[tokio::test]
async fn a_launch_with_no_token_of_its_own_presents_the_clients_credentials() {
    let ack = r#"{"jobId":"job-1","statusUrl":"/jobs/job-1","liveUrl":"/jobs/job-1/live"}"#;
    let (base, head) = serve(format!(
        "HTTP/1.1 202 Accepted\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{ack}",
        ack.len()
    ))
    .await;

    let job = HttpBackendClient::new(base)
        .with_access([AccessCredential::UserToken("user-token".to_string())])
        .launch_run(&launch_body(), "")
        .await
        .expect("the launch is accepted");
    assert_eq!(job, "job-1");

    let head = head
        .await
        .expect("the request arrived")
        .to_ascii_lowercase();
    assert!(!head.contains("authorization:"), "{head}");
    assert!(head.contains("cf-access-token: user-token"), "{head}");
}
