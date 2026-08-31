//! Tests the listener over real TCP — the part a fake cannot prove.

use origin_auth::RedirectListener;
use origin_auth_loopback::LoopbackRedirect;
use origin_domain::ErrorKind;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Send one raw HTTP request to the listener and return its response body.
async fn request(redirect_uri: &str, target: &str) -> String {
    let authority = redirect_uri
        .trim_start_matches("http://")
        .split('/')
        .next()
        .expect("redirect uri has an authority");

    let mut stream = TcpStream::connect(authority).await.expect("connect");
    stream
        .write_all(format!("GET {target} HTTP/1.1\r\nhost: {authority}\r\n\r\n").as_bytes())
        .await
        .expect("write request");

    let mut response = String::new();
    let _ = stream.read_to_string(&mut response).await;
    response
}

/// Helper to bind a listener with a 5-second test timeout so socket failures fail fast.
async fn test_listener() -> LoopbackRedirect {
    LoopbackRedirect::bind()
        .await
        .unwrap()
        .with_timeout(Duration::from_secs(5))
}

#[tokio::test]
async fn the_redirect_uri_points_at_an_ephemeral_loopback_port() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    assert!(uri.starts_with("http://127.0.0.1:"), "got {uri}");
    assert!(uri.ends_with("/callback"));
    assert!(
        !uri.contains(":0/"),
        "an ephemeral bind must report the real port, got {uri}"
    );
}

#[tokio::test]
async fn two_listeners_never_share_a_port() {
    let first = test_listener().await;
    let second = test_listener().await;

    assert_ne!(first.redirect_uri(), second.redirect_uri());
}

#[tokio::test]
async fn a_matching_redirect_yields_the_code() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client =
        tokio::spawn(async move { request(&uri, "/callback?code=code-abc&state=state-1").await });

    let code = listener.wait("state-1").await.unwrap();
    assert_eq!(code.as_str(), "code-abc");

    let response = client.await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "got: {response}");
    assert!(response.contains("close this window"));
}

#[tokio::test]
async fn a_percent_encoded_code_is_decoded() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client =
        tokio::spawn(async move { request(&uri, "/callback?code=a%2Fb%2Bc&state=s").await });

    assert_eq!(listener.wait("s").await.unwrap().as_str(), "a/b+c");
    client.await.unwrap();
}

#[tokio::test]
async fn a_denied_authorization_is_reported_with_its_description() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client = tokio::spawn(async move {
        request(
            &uri,
            "/callback?error=access_denied&error_description=The+user+said+no&state=s",
        )
        .await
    });

    let error = listener.wait("s").await.unwrap_err();
    client.await.unwrap();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(
        error.to_string().contains("The user said no"),
        "got: {error}"
    );
}

#[tokio::test]
async fn a_redirect_with_the_wrong_state_is_rejected() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client =
        tokio::spawn(async move { request(&uri, "/callback?code=stolen&state=attacker").await });

    let error = listener.wait("our-state").await.unwrap_err();
    client.await.unwrap();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(error.to_string().contains("expected state"), "got: {error}");
}

#[tokio::test]
async fn an_error_redirect_must_pass_the_state_check_too() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client = tokio::spawn(async move {
        request(
            &uri,
            "/callback?error=access_denied&error_description=attacker-controlled&state=attacker",
        )
        .await
    });

    let error = listener.wait("our-state").await.unwrap_err();
    client.await.unwrap();

    assert!(error.to_string().contains("expected state"), "got: {error}");
    assert!(!error.to_string().contains("attacker-controlled"));
}

#[tokio::test]
async fn unrelated_requests_do_not_end_the_wait() {
    let listener = test_listener().await;
    let uri = listener.redirect_uri();

    let client = tokio::spawn(async move {
        // Browsers ask for this the moment they open the page.
        let favicon = request(&uri, "/favicon.ico").await;
        assert!(favicon.starts_with("HTTP/1.1 404"), "got: {favicon}");

        request(&uri, "/callback?code=code-abc&state=s").await
    });

    assert_eq!(listener.wait("s").await.unwrap().as_str(), "code-abc");
    client.await.unwrap();
}

#[tokio::test]
async fn waiting_forever_is_not_an_option() {
    let listener = LoopbackRedirect::bind()
        .await
        .unwrap()
        .with_timeout(Duration::from_millis(150));

    let error = listener.wait("s").await.unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Authentication);
    assert!(error.to_string().contains("timed out"), "got: {error}");
}
