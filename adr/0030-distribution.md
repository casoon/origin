# ADR-0030  Distribution

Status:   Accepted
Date:     2026-08-24
Supersedes the draft in ADR-0012.

## Context

Signing, notarisation and updater wiring get re-solved per project, usually under time
pressure shortly before a first release. The pieces are not hard individually; what
costs time is discovering, one failure at a time, which ones exist.

A template cannot solve this by shipping credentials. It can solve the discovery
problem.

## Decision

The release path is tag-driven and identical for every product:

```text
git tag v1.2.0 → CI → build matrix → package → draft release
```

Four decisions make it usable before anyone has a certificate:

- **Unsigned builds succeed.** Every signing secret is optional. A missing one produces
  an unsigned artifact, never a failed release and never a half-signed one. A pipeline
  that fails mysteriously the first time it runs is a pipeline people route around.
- **The build log states what the artifact is.** "macOS signing: NOT configured — this
  artifact is UNSIGNED" appears in plain text. An unsigned build that *looks* signed is
  the failure mode worth preventing; and each platform's consequence is documented
  rather than discovered.
- **Releases are drafts.** A human sees the artifacts before anyone can download them.
- **The updater is off by default, and cannot be enabled over plain HTTP.** An updater
  that cannot verify a signature is worse than no updater: it turns a compromised
  endpoint into arbitrary code execution on every installation. The manifest rejects a
  non-`https` endpoint at parse time.

Identity — product id, name, version — lives in `app.toml`, and `cargo xtask validate`
checks that `tauri.conf.json` agrees. The two are edited in different situations, and a
mismatch is invisible until a release ships under the wrong version or an installer
collides with another application's identifier.

Signing identities are never in the manifest. They are CI secrets.

## Consequences

- A new product can build releases on day one and add signing when it has an audience.
- Channels (`stable`, `beta`, `nightly`) get separate endpoints, so a release process
  cannot move a user between them behind their back.
- The Linux path stays unsigned by design; distribution-level signing is out of scope.
- Nothing here is verified end to end in this repository — there is no product to sign.
  The first real release will find something, and that is the honest expectation.
