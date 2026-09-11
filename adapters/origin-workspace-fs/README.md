# origin-workspace-fs

Workspace filesystem adapter for Origin (`WorkspaceFs`, B2).

Part of [Origin](https://github.com/casoon/origin), a Rust/Tauri platform for building
desktop applications from a shared set of domain crates. See the workspace
[documentation](https://github.com/casoon/origin/tree/main/docs) for how the pieces fit
together, and [ARCHITECTURE.md](https://github.com/casoon/origin/blob/main/ARCHITECTURE.md)
for the rules this crate follows.

## Example

```rust
use origin_platform::{RelPath, WorkspaceFs, WorkspaceRoot};
use origin_workspace_fs::StdWorkspaceFs;

let fs = StdWorkspaceFs::new();
let root = WorkspaceRoot::new("/path/to/project")?;
let path = RelPath::new("src/main.rs")?;

let contents = fs.read_file(&root, &path).await?;
```

Implements read-only filesystem access for workspace repositories. All operations are strictly
confined to user-confirmed `WorkspaceRoot` locations. Path traversal and symlink escapes outside
the root are verified and rejected with permission errors.

## Stability

Pre-1.0 (`0.2.0`). Public types, enums and field sets may still change between minor
versions; pin an exact version if that matters to you.
