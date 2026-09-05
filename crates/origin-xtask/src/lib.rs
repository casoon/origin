//! Origin maintenance tasks, as a library.
//!
//! A derivative's `xtask` is three lines:
//!
//! ```ignore
//! fn main() -> std::process::ExitCode {
//!     origin_xtask::main()
//! }
//! ```
//!
//! That is the point. Architecture rules, the generator and the CI recipe arrive with
//! a version bump instead of being copied into each project and drifting apart —
//! a new rule lands in every derivative and fails its CI the same day (see the update
//! system plan, category C).

mod contracts;
mod generate;
mod migrations;
mod scaffold;
mod update;
mod validate;
mod version;

pub use generate::{check as check_generated, run as generate};
pub use scaffold::{Options as NewOptions, run as new};
pub use update::run as update;
pub use version::Version;

pub(crate) use generate::find_manifests;
pub use validate::run as validate;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Dispatch a task from the process arguments.
pub fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let flags: Vec<String> = std::env::args().skip(2).collect();

    let result = match task.as_deref() {
        Some("validate") => validate(&workspace_root()),
        Some("generate") if flags.iter().any(|flag| flag == "--check") => {
            check_generated(&workspace_root())
        }
        Some("generate") => generate(&workspace_root()),
        Some("update") => update(
            &workspace_root(),
            flags.iter().any(|flag| flag == "--dry-run"),
        ),
        Some("new") => new_from_args(&flags),
        Some("ci") => ci(),
        Some("demo") => demo(),
        Some(other) => {
            eprintln!("unknown task `{other}`");
            usage();
            return ExitCode::FAILURE;
        }
        None => {
            usage();
            return ExitCode::FAILURE;
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("\nxtask failed: {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "\nusage: cargo xtask <task>\n\n\
         tasks:\n  \
           validate           check the rules in ARCHITECTURE.md\n  \
           generate           write the files derived from app.toml\n  \
           generate --check   fail if generated files are stale or hand-edited\n  \
           update             run pending migrations and regenerate\n  \
           update --dry-run   report what an update would change\n  \
           new <slug>         create a new application from the template\n  \
           ci                 fmt --check, clippy -D warnings, test, generate --check, validate\n  \
           demo               run the reference application\n"
    );
}

/// Parse `new <slug> [--name X] [--id Y] [--into DIR] [--local]`.
fn new_from_args(flags: &[String]) -> Result<(), String> {
    let root = workspace_root();

    let slug = flags
        .first()
        .filter(|argument| !argument.starts_with("--"))
        .ok_or_else(|| {
            "usage: cargo xtask new <slug> [--name X] [--id Y] [--into DIR] [--local]".to_owned()
        })?
        .clone();

    let value = |name: &str| -> Option<String> {
        flags
            .iter()
            .position(|flag| flag == name)
            .and_then(|index| flags.get(index + 1))
            .cloned()
    };

    let options = scaffold::Options {
        name: value("--name").unwrap_or_else(|| title_case(&slug)),
        id: value("--id").unwrap_or_else(|| format!("dev.local.{}", slug.replace('-', ""))),
        into: value("--into")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.clone()),
        // Released Origin packages are the normal dependency source. `--local` is
        // reserved for Origin's downstream CI and side-by-side platform development.
        local: flags
            .iter()
            .any(|flag| flag == "--local")
            .then(|| root.clone()),
        slug,
    };

    scaffold::run(&root, &options)
}

/// `my-app` → `My App`.
fn title_case(slug: &str) -> String {
    slug.split(['-', '_'])
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn ci() -> Result<(), String> {
    let root = workspace_root();

    cargo(&["fmt", "--all", "--check"])?;
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])?;
    cargo(&["test", "--workspace"])?;
    check_generated(&root)?;
    validate(&root)
}

fn demo() -> Result<(), String> {
    run("pnpm", &["--filter", "@origin/demo", "tauri", "dev"])
}

fn cargo(args: &[&str]) -> Result<(), String> {
    run("cargo", args)
}

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("\n$ {program} {}", args.join(" "));

    let status = Command::new(program)
        .args(args)
        .current_dir(workspace_root())
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", args.join(" ")))
    }
}

/// The repository root.
///
/// Discovered by walking up from the current directory rather than taken from
/// `CARGO_MANIFEST_DIR`: as a library, this crate's manifest directory is somewhere in
/// the Cargo registry, not in the project being worked on.
pub fn workspace_root() -> PathBuf {
    let start = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for directory in start.ancestors() {
        if is_workspace_root(directory) {
            return directory.to_path_buf();
        }
    }

    start
}

fn is_workspace_root(directory: &Path) -> bool {
    let manifest = directory.join("Cargo.toml");
    let Ok(contents) = std::fs::read_to_string(&manifest) else {
        return false;
    };

    contents
        .lines()
        .any(|line| line.trim_start().starts_with("[workspace]"))
}
