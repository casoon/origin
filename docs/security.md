# Security

See [ADR-0007](../adr/0007-security-and-capability-model.md) for the decision.

## Two permission levels

They are never mixed.

| Level | Answers | Examples |
| --- | --- | --- |
| Product | What may the app do at an external service? | `ReadNotifications`, `WriteProjects` |
| Platform | What may it do on this machine? | Filesystem, Shell, Process, Credential Store |

## Capabilities are scoped per window

Each window gets a named security profile, and each profile lists its permissions
explicitly. There is no `fs: all` and no `shell: all` — `cargo xtask validate` fails the
build on one.

```text
main window       → standard-dashboard   read state, receive events
settings window   → account-settings     manage accounts and credentials
workspace window  → local-workspace      limited filesystem, configured tools only
```

The most effective capability is the one you never grant. Notifications and URL opening
happen in Rust through platform contracts, so the demo's frontend needs **no**
notification and **no** opener permission at all.

## Capabilities are build-time, not runtime

A product that did not wire an `Opener` into its composition root cannot open URLs — the
command fails with a permission error. The capability is absent, not disabled.

## Credentials

Credentials go to the OS credential store through `SecretStore`, never into the
application database:

| Platform | Backend |
| --- | --- |
| macOS | Keychain |
| Windows | Credential Manager |
| Linux | Secret Service |

`Secret` redacts itself in `Debug` and `Display`; reading the real value requires an
explicit `.expose()`, which makes every call site reviewable.

## Opening URLs

`TauriOpener` rejects anything that is not `http://` or `https://` before it reaches the
OS. `file://` and custom schemes are how "open a link" quietly becomes "launch a
program".

## Logging

Never log secrets or unfiltered personal data. Attach correlation fields
(`account_id`, `sync_id`, `job_id`) via `origin_telemetry::spans` instead of writing
identifiers into the message.
