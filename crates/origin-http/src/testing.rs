//! Offline test double for [`HttpClient`].

use crate::{Headers, HttpClient, HttpRequest, HttpResponse};
use async_trait::async_trait;
use origin_core::{AppError, Result};
use std::collections::VecDeque;
use std::sync::Mutex;

/// Answers requests from a queue and records what it was asked.
///
/// Connector tests use this instead of a live service, so they are deterministic and
/// run without a network.
#[derive(Debug, Default)]
pub struct MockHttpClient {
    queued: Mutex<VecDeque<Result<HttpResponse>>>,
    recorded: Mutex<Vec<HttpRequest>>,
}

impl MockHttpClient {
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a response. Responses are returned in the order they were queued.
    pub fn push(&self, response: HttpResponse) -> &Self {
        self.queued
            .lock()
            .expect("mock poisoned")
            .push_back(Ok(response));
        self
    }

    /// Queue a JSON response with the given status.
    pub fn push_json(&self, status: u16, body: &str) -> &Self {
        self.push(HttpResponse::new(
            status,
            Headers::from_iter([("content-type", "application/json")]),
            body.as_bytes().to_vec(),
        ))
    }

    /// Queue a transport failure, e.g. to test offline handling.
    pub fn push_error(&self, error: AppError) -> &Self {
        self.queued
            .lock()
            .expect("mock poisoned")
            .push_back(Err(error));
        self
    }

    /// Every request that was sent, in order.
    pub fn requests(&self) -> Vec<HttpRequest> {
        self.recorded.lock().expect("mock poisoned").clone()
    }

    /// The last request, for the common single-call assertion.
    pub fn last_request(&self) -> Option<HttpRequest> {
        self.recorded.lock().expect("mock poisoned").last().cloned()
    }

    /// Fail the test if queued responses were never consumed.
    pub fn assert_all_consumed(&self) {
        let remaining = self.queued.lock().expect("mock poisoned").len();
        assert_eq!(
            remaining, 0,
            "{remaining} queued response(s) were never used"
        );
    }
}

#[async_trait]
impl HttpClient for MockHttpClient {
    async fn send(&self, request: HttpRequest) -> Result<HttpResponse> {
        self.recorded
            .lock()
            .expect("mock poisoned")
            .push(request.clone());

        self.queued
            .lock()
            .expect("mock poisoned")
            .pop_front()
            .unwrap_or_else(|| {
                Err(AppError::internal(format!(
                    "no response queued for {} {}",
                    request.method, request.url
                )))
            })
    }
}
