# Capabilities

**These files are generated. Do not edit them.**

They are derived from `../../app.toml` by `cargo xtask generate`, one file per security
profile (§20, ADR-0007). `cargo xtask generate --check` runs in CI and fails on a
hand-edit or a stale file.

To change what a window may do, change its profile in `app.toml` and regenerate. To
change what a *profile* means, change it in `origin-manifest` — where it changes for
every Origin application at once, and is reviewed once instead of per project.

Rules the profiles enforce:

- No blanket `fs:*`, `shell:*` or `process:*` grants. No profile grants any of them.
- Permissions are listed explicitly rather than pulling in a plugin's `default` set —
  a plugin default grows when the plugin is updated, silently widening every window
  that used it.
- Opening URLs and showing notifications happen in Rust behind platform contracts, so
  the frontend needs no permission for either.
