# Security

## Capabilities are generated, and narrow

Each window gets a **named security profile**, declared in `app.toml`:

```toml
[security.windows]
main = { profile = "standard-dashboard" }
settings = { profile = "account-settings" }
```

`cargo xtask generate` turns that into `src-tauri/capabilities/*.json`. Those files are
generated — editing one works until the next generate wipes it, and CI fails first.

| Profile | For |
| --- | --- |
| `readonly-dashboard` | reads state, receives events |
| `standard-dashboard` | the usual main window |
| `account-settings` | manages accounts through commands |

**No profile grants filesystem, shell or process access.** That is not a convention —
it is asserted by a test in the platform, and `cargo xtask validate` fails the build if
a capability file grants one.

Two things follow that are easy to miss:

- Permissions are listed **explicitly** rather than pulling in a plugin's `default` set.
  A plugin default grows when the plugin is updated, silently widening every window that
  used it.
- Opening URLs and showing notifications happen **in Rust**, behind platform contracts.
  The frontend therefore needs no permission for either. The most effective capability
  is the one you never grant.

## Adding a window

Adding a window means choosing a profile. If none fits, that is a conversation to have
deliberately — not a permission to append to an existing file. Raise it upstream, or
declare an override and write down why:

```toml
[origin.overrides]
hand_written_capabilities = true
```

## Credentials

Credentials go to the OS keychain through `SecretStore`, never into the database:

| Platform | Backend |
| --- | --- |
| macOS | Keychain |
| Windows | Credential Manager |
| Linux | Secret Service |

`Secret` redacts itself in `Debug` and `Display`. Reading the real value requires an
explicit `.expose()`, which makes every call site something a reviewer can find.

Deleting the local database must cost the user a resync and nothing else.

## Logging

Never log secrets or unfiltered personal data. Attach correlation fields instead of
writing identifiers into messages:

```rust
tracing::info!(account_id = %account, connector = %connector, "sync started");
```

HTTP headers redact themselves when formatted with `Debug`, so a bearer token cannot
reach a log line through `tracing::debug!(?headers, …)`.

## If you add AI features

An MCP tool is invoked by a language model acting on content it read somewhere, and
that content may be hostile. The default grant is read, search and propose — never
commit or delete. Model write access as a *proposal* a human confirms.

## Before you ship

- Replace the placeholder icons.
- Read [releasing.md](releasing.md): until signing is configured, artifacts are
  unsigned, and an unsigned build reaching users is a security decision, not an
  oversight.
- Check that no capability file has grown a permission nobody can explain.
