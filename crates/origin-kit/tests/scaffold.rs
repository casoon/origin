//! What `origin new` produces, and the packaging rule the template has to keep.

use std::path::{Path, PathBuf};

/// Removes its directory again, whether the test passed or panicked.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("origin-kit-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("cannot create the scratch directory");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The rule behind the `.in` suffix: `cargo package` skips every directory that holds
/// a `Cargo.toml`, so a template manifest under its real name would be missing from
/// the published crate — and `origin new` would write a project without a manifest.
/// A test, because the failure is invisible in a checkout and only appears after
/// `cargo install origin-kit` (ADR-0032).
#[test]
fn no_template_file_is_named_cargo_toml() {
    let offenders: Vec<&str> = origin_kit::template_files()
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| {
            Path::new(path)
                .file_name()
                .is_some_and(|name| name == "Cargo.toml")
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "these template manifests must be stored with the `.in` suffix, or `cargo \
         package` will drop their directories: {offenders:?}"
    );
}

#[test]
fn the_template_is_embedded() {
    let paths: Vec<&str> = origin_kit::template_files()
        .iter()
        .map(|(path, _)| *path)
        .collect();

    for expected in [
        "Cargo.toml.in",
        "app.toml",
        "src-tauri/Cargo.toml.in",
        "src-tauri/src/lib.rs",
        "src-tauri/icons/icon.icns",
        "xtask/Cargo.toml.in",
        "ui/src/App.svelte",
        // Dotfiles are the ones a packaging step drops without a word, and a project
        // without its CI workflow or `.gitignore` looks complete until the first push.
        ".github/workflows/ci.yml",
        ".github/workflows/release.yml",
        ".github/dependabot.yml",
        ".cargo/config.toml",
        ".gitignore",
    ] {
        assert!(paths.contains(&expected), "{expected} is not embedded");
    }
}

#[test]
fn a_scaffolded_project_has_no_placeholders_left() {
    let scratch = Scratch::new("scaffold");

    let options = origin_kit::Options {
        slug: "sample-app".to_owned(),
        name: "Sample App".to_owned(),
        id: "dev.example.sampleapp".to_owned(),
        into: scratch.0.clone(),
        local: None,
    };
    origin_kit::run(&options).expect("scaffolding failed");

    let project = scratch.0.join("sample-app");

    // The renamed manifests arrive under their real names, and nothing is left behind
    // under the suffix.
    for manifest in ["Cargo.toml", "src-tauri/Cargo.toml", "xtask/Cargo.toml"] {
        assert!(project.join(manifest).is_file(), "{manifest} is missing");
        assert!(
            !project.join(format!("{manifest}.in")).exists(),
            "{manifest}.in was written as well"
        );
    }

    // The icons survived the trip as bytes rather than as text.
    let icon = project.join("src-tauri").join("icons").join("icon.icns");
    assert!(
        std::fs::metadata(&icon).expect("no icon").len() > 0,
        "the icon is empty"
    );

    for file in text_files(&project) {
        let contents = std::fs::read_to_string(&file).expect("cannot read a generated file");
        assert!(
            leftover_placeholder(&contents).is_none(),
            "{} still contains the placeholder {}",
            file.display(),
            leftover_placeholder(&contents).unwrap_or_default()
        );
    }

    let manifest = std::fs::read_to_string(project.join("Cargo.toml")).expect("no Cargo.toml");
    assert!(manifest.contains("origin-app"));
    assert!(
        manifest.parse::<toml_edit::DocumentMut>().is_ok(),
        "the generated Cargo.toml is not valid TOML"
    );

    let app = std::fs::read_to_string(project.join("app.toml")).expect("no app.toml");
    assert!(app.contains("dev.example.sampleapp"));
    assert!(app.contains("\"Sample App\""));
}

/// Every file in the generated project that is checked for leftover placeholders.
///
/// Binary files are skipped by extension, and so is `pnpm-lock.yaml` and anything a
/// developer's tooling may have put there — nothing of that kind exists straight after
/// scaffolding, which is the state this test runs in.
fn text_files(directory: &Path) -> Vec<PathBuf> {
    let binary = ["png", "ico", "icns", "jpg", "jpeg", "webp"];
    let mut files = Vec::new();

    for entry in std::fs::read_dir(directory)
        .expect("cannot read the project")
        .flatten()
    {
        let path = entry.path();
        if path.is_dir() {
            files.extend(text_files(&path));
            continue;
        }
        let is_binary = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| binary.contains(&extension));
        if !is_binary {
            files.push(path);
        }
    }

    files
}

/// The first `__SHOUTING__` token left in a file, if any.
///
/// Matched by shape rather than against the known list, so a placeholder added to the
/// template without a substitution fails this test instead of reaching a user's
/// project verbatim.
fn leftover_placeholder(contents: &str) -> Option<String> {
    let bytes = contents.as_bytes();
    let mut start = 0;

    while let Some(index) = contents[start..].find("__") {
        let open = start + index + 2;
        let length = contents[open..].find("__")?;
        let token = &bytes[open..open + length];
        let shouting = length >= 2
            && token
                .iter()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || *byte == b'_');

        if shouting {
            return Some(format!("__{}__", &contents[open..open + length]));
        }

        start = open;
    }

    None
}
