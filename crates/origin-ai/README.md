# origin-ai

Embedded AI inference port for Origin applications.

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_ai::{AiService, Prompt};
use std::sync::Arc;

async fn summarize(ai: Arc<dyn AiService>, text: &str) -> origin_domain::Result<String> {
    let prompt = Prompt::new("Summarize this text", text);
    let completion = ai.complete(prompt).await?;
    Ok(completion.text)
}
```

`AiService` represents inference the application performs itself (ADR-0028). It is
completely independent of MCP (ADR-0027), which makes the application controllable by
an external AI client.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
