//! The `origin` command.
//!
//! Arguments are parsed by hand, as in `origin-xtask`: one command with four flags
//! does not need a parser, and a tool whose whole job is to start a project should not
//! make a dependency decision on its user's behalf before the project exists.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next();
    let flags: Vec<String> = arguments.collect();

    let result = match command.as_deref() {
        Some("new") => new(&flags),
        Some("--version" | "-V") => {
            println!("origin {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Some("--help" | "-h" | "help") => {
            usage();
            return ExitCode::SUCCESS;
        }
        Some(other) => {
            eprintln!("unknown command `{other}`");
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
            eprintln!("\norigin failed: {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() {
    eprintln!(
        "\nusage: origin <command>\n\n\
         commands:\n  \
           new <slug>         create a new application from the template\n  \
           --version          print the version of this tool\n  \
           --help             print this message\n\n\
         `new` flags:\n  \
           --name <name>      display name; defaults to the slug in title case\n  \
           --id <id>          reverse-DNS product id; defaults to dev.local.<slug>\n  \
           --into <dir>       parent directory for the new project; defaults to .\n  \
           --local            depend on the surrounding Origin checkout instead of\n                     \
                              the released crates\n\n\
         Inside a generated project, the project's own tasks are `cargo xtask\n\
         validate`, `generate` and `update`.\n"
    );
}

/// Parse `new <slug> [--name X] [--id Y] [--into DIR] [--local]`.
fn new(flags: &[String]) -> Result<(), String> {
    let slug = flags
        .first()
        .filter(|argument| !argument.starts_with("--"))
        .ok_or_else(|| {
            "usage: origin new <slug> [--name X] [--id Y] [--into DIR] [--local]".to_owned()
        })?
        .clone();

    let value = |name: &str| -> Option<String> {
        flags
            .iter()
            .position(|flag| flag == name)
            .and_then(|index| flags.get(index + 1))
            .cloned()
    };

    let local = flags.iter().any(|flag| flag == "--local");

    let options = origin_kit::Options {
        name: value("--name").unwrap_or_else(|| title_case(&slug)),
        id: value("--id").unwrap_or_else(|| format!("dev.local.{}", slug.replace('-', ""))),
        into: value("--into")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
        // Released Origin packages are the normal dependency source. `--local` is
        // reserved for Origin's downstream CI and side-by-side platform development.
        local: if local {
            Some(origin_checkout()?)
        } else {
            None
        },
        slug,
    };

    origin_kit::run(&options)
}

/// The Origin checkout `--local` should point the generated project at.
///
/// Reported as an error rather than guessed: `workspace_root` falls back to the
/// current directory when it finds no workspace, and silently generating a project
/// whose dependencies point at a directory that holds no platform crates would only
/// fail later, during its first build.
fn origin_checkout() -> Result<PathBuf, String> {
    let root = origin_xtask::workspace_root();

    if is_origin_checkout(&root) {
        return Ok(root);
    }

    Err(format!(
        "--local needs to run inside a checkout of Origin; {} is not one",
        root.display()
    ))
}

fn is_origin_checkout(root: &Path) -> bool {
    root.join("crates")
        .join("origin-app")
        .join("Cargo.toml")
        .is_file()
        && root
            .join("host")
            .join("origin-tauri")
            .join("Cargo.toml")
            .is_file()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slug_becomes_a_display_name() {
        assert_eq!(title_case("my-app"), "My App");
        assert_eq!(title_case("metricbit"), "Metricbit");
        assert_eq!(title_case("two_words"), "Two Words");
    }

    #[test]
    fn a_directory_without_the_platform_crates_is_not_a_checkout() {
        assert!(!is_origin_checkout(Path::new("/")));
    }
}
