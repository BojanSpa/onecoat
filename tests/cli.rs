//! End-to-end tests for the CLI.
//!
//! Every test spawns the real binary with `LOCALAPPDATA` and `APPDATA` pointed at a fresh
//! temporary root, so the live Windows Terminal files are never touched (N-4).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use tempfile::TempDir;

/// A temporary environment root and the binary that runs against it.
struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            root: TempDir::new().unwrap(),
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_onecoat"))
            .args(args)
            .env("LOCALAPPDATA", self.root.path())
            .env("APPDATA", self.root.path())
            .output()
            .unwrap()
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    /// The fragment path onecoat derives from `LOCALAPPDATA`.
    fn fragment(&self) -> PathBuf {
        self.root
            .path()
            .join("Microsoft")
            .join("Windows Terminal")
            .join("Fragments")
            .join("onecoat")
            .join("schemes.json")
    }

    /// The user theme directory onecoat derives from `APPDATA`.
    fn themes(&self) -> PathBuf {
        self.root.path().join("onecoat").join("themes")
    }

    fn install_theme(&self, fixture: &str, name: &str) -> PathBuf {
        let dir = self.themes();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::copy(fixture_path(fixture), &path).unwrap();
        path
    }

    /// Puts a fixture at the fragment path, as if a previous apply had written it.
    fn install_fragment(&self, fixture: &str) -> PathBuf {
        let fragment = self.fragment();
        std::fs::create_dir_all(fragment.parent().unwrap()).unwrap();
        std::fs::copy(fixture_path(fixture), &fragment).unwrap();
        fragment
    }
}

/// `<path>` with `suffix` appended to the whole file name.
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

fn fixture_bytes(relative: &str) -> Vec<u8> {
    std::fs::read(fixture_path(relative)).unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

/// A file's size and modification time.
fn stamp(path: &Path) -> (u64, SystemTime) {
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.len(), metadata.modified().unwrap())
}

/// Every entry under `root`, as paths relative to it, sorted.
fn tree(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            found.push(path.strip_prefix(root).unwrap().display().to_string());
            if path.is_dir() {
                walk(root, &path, found);
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.sort();
    found
}

#[test]
fn list_prints_the_bundled_theme() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["list"]);
    assert_eq!(code(&output), 0);
    assert_eq!(
        stdout(&output),
        "id   appearance origin  name\nnord dark       bundled Nord\n"
    );
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}

#[test]
fn list_json_prints_machine_readable_rows() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["list", "--json"]);
    assert_eq!(code(&output), 0);
    let expected = r#"[
  {
    "id": "nord",
    "name": "Nord",
    "appearance": "dark",
    "origin": "bundled",
    "derived": false
  }
]
"#;
    assert_eq!(stdout(&output), expected);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!([{
            "id": "nord",
            "name": "Nord",
            "appearance": "dark",
            "origin": "bundled",
            "derived": false,
        }])
    );
    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}

#[test]
fn a_user_theme_shadows_the_bundled_one() {
    let sandbox = Sandbox::new();
    sandbox.install_theme("themes/shadow-nord.toml", "shadow-nord.toml");

    let listed = sandbox.run(&["list", "--json"]);
    assert_eq!(code(&listed), 0);
    let rows: serde_json::Value = serde_json::from_str(&stdout(&listed)).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1, "{rows}");
    assert_eq!(rows[0]["id"], "nord");
    assert_eq!(rows[0]["name"], "Shadow Nord");
    assert_eq!(rows[0]["origin"], "user");

    let human = sandbox.run(&["list"]);
    let human = stdout(&human);
    let lines: Vec<&str> = human.lines().collect();
    assert_eq!(lines.len(), 2, "{human}");
    assert!(
        lines[1].contains("user") && lines[1].contains("Shadow Nord"),
        "{}",
        lines[1]
    );

    let applied = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&applied), 0, "{}", stderr(&applied));
    let fragment = std::fs::read(sandbox.fragment()).unwrap();
    let fragment: serde_json::Value = serde_json::from_slice(&fragment).unwrap();
    assert_eq!(fragment["schemes"][0]["background"], "#101820");
}

#[test]
fn use_writes_the_fragment_and_reports_it() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0);
    assert!(output.stderr.is_empty(), "{}", stderr(&output));

    let fragment = sandbox.fragment();
    assert_eq!(stdout(&output), format!("wrote {}\n", fragment.display()));
    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/nord-dark-fragment.json")
    );
    assert!(!sibling(&fragment, ".onecoat.bak").exists());
    assert!(!sibling(&fragment, ".onecoat.tmp").exists());
}

#[test]
fn a_second_use_neither_rewrites_nor_rebacks_up() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let backup = sibling(&fragment, ".onecoat.bak");

    let first = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));
    let written_stamp = stamp(&fragment);
    let backup_stamp = stamp(&backup);

    let second = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));
    assert_eq!(
        stdout(&second),
        format!("unchanged {}\n", fragment.display())
    );
    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/nord-dark-fragment.json")
    );
    assert_eq!(
        std::fs::read(&backup).unwrap(),
        fixture_bytes("wt/previous-fragment.json")
    );
    assert_eq!(stamp(&fragment), written_stamp);
    assert_eq!(stamp(&backup), backup_stamp);
}

#[test]
fn use_backs_up_the_previous_fragment() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert_eq!(
        std::fs::read(sibling(&fragment, ".onecoat.bak")).unwrap(),
        fixture_bytes("wt/previous-fragment.json")
    );
    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/nord-dark-fragment.json")
    );
    assert!(!sibling(&fragment, ".onecoat.tmp").exists());
}

#[test]
fn a_failed_backup_leaves_the_fragment_intact() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let backup = sibling(&fragment, ".onecoat.bak");
    std::fs::create_dir(&backup).unwrap();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 3);
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    let message = stderr(&output);
    assert!(message.contains(&backup.display().to_string()), "{message}");
    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/previous-fragment.json")
    );
    assert!(!sibling(&fragment, ".onecoat.tmp").exists());
}

#[test]
fn use_reports_an_unknown_id() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["use", "nope"]);
    assert_eq!(code(&output), 3);
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    let message = stderr(&output);
    assert!(message.contains("nope"), "{message}");
    assert!(message.contains("onecoat list"), "{message}");
    assert!(!sandbox.fragment().exists());
}

#[test]
fn usage_errors_exit_two() {
    let sandbox = Sandbox::new();
    for args in [Vec::new(), vec!["use"], vec!["bogus"]] {
        let output = sandbox.run(&args);
        assert_eq!(code(&output), 2, "args {args:?}");
    }
    let version = sandbox.run(&["--version"]);
    assert_eq!(code(&version), 0);
}

#[test]
fn a_broken_user_theme_fails_both_commands() {
    let sandbox = Sandbox::new();
    let path = sandbox.install_theme("themes/reserved-id.toml", "reserved-id.toml");
    for args in [vec!["list"], vec!["use", "nord"]] {
        let output = sandbox.run(&args);
        assert_eq!(code(&output), 3, "args {args:?}");
        assert!(stdout(&output).is_empty(), "{}", stdout(&output));
        let message = stderr(&output);
        assert!(message.contains(&path.display().to_string()), "{message}");
        assert!(message.contains("titanium"), "{message}");
    }
}

#[test]
fn writes_stay_inside_the_fragment_directory() {
    let sandbox = Sandbox::new();
    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let base = PathBuf::from("Microsoft")
        .join("Windows Terminal")
        .join("Fragments")
        .join("onecoat");
    let mut expected = vec![
        PathBuf::from("Microsoft"),
        PathBuf::from("Microsoft").join("Windows Terminal"),
        PathBuf::from("Microsoft")
            .join("Windows Terminal")
            .join("Fragments"),
        base.clone(),
        base.join("schemes.json"),
    ]
    .into_iter()
    .map(|path| path.display().to_string())
    .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(tree(sandbox.root()), expected);

    // With a fragment already in place, the backup is the only extra entry (R-11).
    let sandbox = Sandbox::new();
    sandbox.install_fragment("wt/previous-fragment.json");
    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    expected.push(base.join("schemes.json.onecoat.bak").display().to_string());
    expected.sort();
    assert_eq!(tree(sandbox.root()), expected);
}
