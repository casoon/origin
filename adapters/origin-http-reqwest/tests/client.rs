//! Integration tests for the reqwest-backed `HttpClient` — real sockets, real headers,
//! and the response-size guard. A fake `HttpClient` cannot prove any of this; only a
//! server that actually sends bytes over TCP can.

use origin_domain::ErrorKind;
use origin_http::{HttpClient, HttpRequest};
use origin_http_reqwest::ReqwestHttpClient;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// Starts a server that writes `response` verbatim to the first connection it accepts,
/// then closes the socket. Returns the address to send a request to.
async fn one_shot_server(response: &'static [u8]) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        let mut buffer = [0u8; 1024];
        // Drain the request so the client's write does not block on a full socket
        // buffer; its content is not what this suite is testing.
        let _ = stream.read(&mut buffer).await;
        let _ = stream.write_all(response).await;
        let _ = stream.shutdown().await;
    });

    address
}

#[tokio::test]
async fn a_response_within_the_limit_is_read_in_full() {
    let address =
        one_shot_server(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello")
            .await;

    let client = ReqwestHttpClient::new("origin-tests").unwrap();
    let response = client
        .send(HttpRequest::get(format!("http://{address}/")))
        .await
        .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"hello");
}

#[tokio::test]
async fn a_declared_content_length_over_the_limit_is_rejected_before_reading_the_body() {
    // The server promises a huge body but never has to send it — the point of checking
    // `Content-Length` first is that the client never gets this far.
    let address =
        one_shot_server(b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n")
            .await;

    let client = ReqwestHttpClient::builder("origin-tests")
        .max_response_bytes(16)
        .build()
        .unwrap();

    let error = client
        .send(HttpRequest::get(format!("http://{address}/")))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ExternalService);
    assert!(error.to_string().contains("16 byte limit"), "got: {error}");
}

#[tokio::test]
async fn a_body_with_no_declared_length_is_still_capped_while_streaming() {
    // No `Content-Length` and no `Transfer-Encoding`: the body is delimited by the
    // connection closing, which is exactly the case a `Content-Length` check alone
    // would miss — this is what the running-total check during the read is for.
    let address = one_shot_server(
        b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nthis body is well over sixteen bytes long",
    )
    .await;

    let client = ReqwestHttpClient::builder("origin-tests")
        .max_response_bytes(16)
        .build()
        .unwrap();

    let error = client
        .send(HttpRequest::get(format!("http://{address}/")))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::ExternalService);
    assert!(error.to_string().contains("16 byte limit"), "got: {error}");
}

#[tokio::test]
async fn a_connection_refused_is_reported_as_offline() {
    // Nothing is listening on this port: binding to :0 above and immediately dropping
    // the listener frees it, guaranteeing a real connection failure to classify.
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("local addr");
    drop(listener);

    let client = ReqwestHttpClient::new("origin-tests").unwrap();
    let error = client
        .send(HttpRequest::get(format!("http://{address}/")))
        .await
        .unwrap_err();

    assert_eq!(error.kind(), ErrorKind::Offline);
}
