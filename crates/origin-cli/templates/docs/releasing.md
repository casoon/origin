# Releasing

## The short version

```bash
git tag v1.0.0
git push --tags
```

That triggers `.github/workflows/release.yml`: a matrix build for macOS (Apple silicon
and Intel), Windows and Linux, published as a **draft** release so a human looks at the
artifacts before anyone can download them.

## Unsigned by default, and honest about it

Signing is **not** configured out of the box, and that is deliberate: a template cannot
hold anyone's certificates, and pretending otherwise would produce a pipeline that fails
mysteriously the first time it runs.

So the release workflow builds successfully without any secrets and prints exactly what
the artifact is:

```text
Target: macos-aarch64
macOS signing: NOT configured — this artifact is UNSIGNED
Update signature: NOT configured — in-app updates cannot be verified
```

Unsigned is fine while you are the only user. It stops being fine the moment someone
else installs the app:

| Platform | What an unsigned build does |
| --- | --- |
| macOS | Gatekeeper refuses to open it; the user must right-click → Open, or clear the quarantine attribute |
| Windows | SmartScreen shows a full-screen warning naming an unknown publisher |
| Linux | Nothing — Linux packaging does not expect signatures |

## Configuring signing

Add these as GitHub Actions secrets. Every one is optional; each one you add turns on
one more part of the pipeline.

### macOS

| Secret | What it is |
| --- | --- |
| `APPLE_CERTIFICATE` | Developer ID Application certificate, base64-encoded `.p12` |
| `APPLE_CERTIFICATE_PASSWORD` | its password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: Your Name (TEAMID)` |

That gets you a signed build. For **notarisation** — without which Gatekeeper still
warns on first launch — add either an App Store Connect API key
(`APPLE_API_KEY`, `APPLE_API_ISSUER`) or an Apple ID
(`APPLE_ID`, `APPLE_TEAM_ID`, and an app-specific password).

Requires a paid Apple Developer account.

### Windows

| Secret | What it is |
| --- | --- |
| `WINDOWS_CERTIFICATE` | code signing certificate, base64-encoded `.pfx` |
| `WINDOWS_CERTIFICATE_PASSWORD` | its password |

An OV certificate still accumulates SmartScreen reputation over time; an EV certificate
does not have that problem and costs more.

### Linux

Nothing to configure. AppImage, `.deb` and `.rpm` ship unsigned; distributions sign at
the repository level, which is out of scope here.

## In-app updates

Off by default. An updater that cannot verify a signature is **worse than no updater**:
it turns a compromised endpoint into arbitrary code execution on every installation. The
manifest refuses to enable one over plain HTTP for the same reason.

To turn it on:

1. Generate a keypair: `cargo tauri signer generate -w ~/.tauri/myapp.key`
2. Add the private key as `TAURI_SIGNING_PRIVATE_KEY` (and its password) in CI secrets.
3. Put the **public** key in `src-tauri/tauri.conf.json` under `plugins.updater.pubkey`.
4. Declare the endpoints in `app.toml`:

```toml
[distribution]
channel = "stable"

[distribution.updater]
enabled = true
endpoints = ["https://releases.example.com/__PACKAGE_NAME__/{{target}}/{{current_version}}"]
```

Keep the private key out of the repository, and keep a backup of it: losing it means
every installed copy can no longer verify updates and has to be reinstalled by hand.

## Channels

`stable`, `beta` and `nightly` are separate audiences with separate endpoints. A user on
`beta` should not be silently moved to `nightly` by a release process, so give each
channel its own endpoint rather than switching one behind users' backs.

## A release checklist

- [ ] Version bumped in `app.toml` and `src-tauri/tauri.conf.json` (`cargo xtask validate` checks they agree)
- [ ] `cargo xtask ci` passes
- [ ] Real icons, not the placeholders
- [ ] Tag matches the version
- [ ] Draft release reviewed before publishing

## Before this ships to anyone but you

Unsigned and unverified is a fine starting point (see above), but it must not still be
true the day this app reaches real users. Before the first release outside your own
machine:

- [ ] Signing is configured for every platform you distribute to — see "Configuring
      signing" above. The workflow's "Report signing status" step in the build log must
      say `configured`, not `UNSIGNED`, for each target you ship.
- [ ] If in-app updates are enabled, the update signature keypair (`TAURI_SIGNING_PRIVATE_KEY`)
      is configured, and the private key has a backup outside CI secrets — see "In-app
      updates" above.
- [ ] The whole update path has been tested end to end at least once: install an older
      signed build, publish a newer signed release, and confirm the running app detects
      it, downloads it, verifies the signature and applies it. A green CI build proves
      the pipeline compiles — it does not prove an update actually installs.

Skipping this is not "unsigned for now" — a signed build with an unverified updater is
still a live risk, and users the app already prompts to update deserve the check to have
actually happened once.
