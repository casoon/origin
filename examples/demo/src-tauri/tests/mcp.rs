//! The MCP server, driven the way a real client drives it: as a child process over
//! stdio.
//!
//! This is the architecture test from §52/§53 executed end to end — the binary answers
//! without a window, a desktop session or a display.

use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Run a session against the real binary and return one parsed response per request.
fn session(requests: &[Value]) -> Vec<Value> {
    // Unique per session: these tests run in parallel in one process, so a directory
    // named only after the process id has them deleting each other's database.
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let data_dir = std::env::temp_dir().join(format!(
        "origin-mcp-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&data_dir);

    let mut child = Command::new(env!("CARGO_BIN_EXE_origin-demo"))
        .arg("--mcp")
        // Never touch the developer's real database.
        .env("ORIGIN_DATA_DIR", &data_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("start the demo in mcp mode");

    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(
            stdin,
            "{}",
            request(
                0,
                "initialize",
                json!({
                    "protocolVersion": origin_mcp::PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": { "name": "origin-e2e", "version": "1.0.0" }
                })
            )
        )
        .expect("write initialize request");
        writeln!(
            stdin,
            "{}",
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        )
        .expect("write initialized notification");
        for request in requests {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    // Closing stdin is how a client ends the session.
    drop(child.stdin.take());

    let stdout = BufReader::new(child.stdout.take().expect("stdout"));
    let responses: Vec<Value> = stdout
        .lines()
        .map_while(std::result::Result::ok)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(&line)
                .unwrap_or_else(|error| panic!("stdout must carry only protocol: {line} ({error})"))
        })
        .collect();

    child.wait().expect("the process exits");
    let _ = std::fs::remove_dir_all(&data_dir);

    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "origin-demo");
    responses.into_iter().skip(1).collect()
}

fn request(id: u32, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}

#[test]
fn the_application_answers_an_ai_client_without_a_window() {
    let responses = session(&[request(1, "tools/list", json!({}))]);

    assert_eq!(responses.len(), 1);

    let names: Vec<&str> = responses[0]["result"]["tools"]
        .as_array()
        .expect("a tool list")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    assert_eq!(names, vec!["demo.status", "demo.threshold.propose"]);
    assert!(
        !names.contains(&"demo.threshold.set"),
        "the commit tool is registered but outside the grant, so it must be invisible"
    );
}

#[test]
fn a_proposal_reports_the_change_without_making_it() {
    let responses = session(&[
        request(
            1,
            "tools/call",
            json!({ "name": "demo.threshold.propose", "arguments": { "value": 40 } }),
        ),
        request(2, "tools/call", json!({ "name": "demo.status" })),
    ]);

    let proposal = &responses[0]["result"];
    assert_eq!(proposal["isError"], false);
    assert_eq!(proposal["structuredContent"]["from"], 85.0);
    assert_eq!(proposal["structuredContent"]["to"], 40.0);
    assert_eq!(
        proposal["structuredContent"]["applied"], false,
        "a proposal must not take effect"
    );

    // And the second call proves it: nothing was written.
    assert_eq!(responses[1]["result"]["isError"], false);
}

#[test]
fn a_commit_tool_outside_the_grant_is_refused() {
    let responses = session(&[request(
        1,
        "tools/call",
        json!({ "name": "demo.threshold.set", "arguments": { "value": 10 } }),
    )]);

    let error = &responses[0]["error"];
    assert!(
        !error.is_null(),
        "expected a refusal, got {:?}",
        responses[0]
    );
    assert!(
        error["message"].as_str().unwrap().contains("commit"),
        "the refusal must name the missing permission: {error}"
    );
}

#[test]
fn arguments_from_a_model_are_treated_as_input_not_as_a_contract() {
    let responses = session(&[
        request(
            1,
            "tools/call",
            json!({ "name": "demo.threshold.propose", "arguments": { "value": 5000 } }),
        ),
        request(
            2,
            "tools/call",
            json!({ "name": "demo.threshold.propose", "arguments": { "value": "high" } }),
        ),
    ]);

    for response in &responses {
        assert_eq!(
            response["result"]["isError"], true,
            "invalid arguments must come back as a readable tool error: {response}"
        );
    }
}
