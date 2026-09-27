use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use tempfile::TempDir;

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

    fn fragment(&self) -> PathBuf {
        self.root
            .path()
            .join("Microsoft")
            .join("Windows Terminal")
            .join("Fragments")
            .join("onecoat")
            .join("schemes.json")
    }

    fn settings(&self) -> PathBuf {
        self.root
            .path()
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState")
            .join("settings.json")
    }

    fn themes(&self) -> PathBuf {
        self.root.path().join("onecoat").join("themes")
    }

    fn install(&self, fixture: &str, path: &Path) -> PathBuf {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::copy(fixture_path(fixture), path).unwrap();
        path.to_path_buf()
    }

    fn install_theme(&self, fixture: &str, name: &str) -> PathBuf {
        self.install(fixture, &self.themes().join(name))
    }

    fn install_fragment(&self, fixture: &str) -> PathBuf {
        self.install(fixture, &self.fragment())
    }

    fn install_settings(&self, fixture: &str) -> PathBuf {
        self.install(fixture, &self.settings())
    }
}

fn append_to_file_name(path: &Path, suffix: &str) -> PathBuf {
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

const PIN_NOTE: &str = "profile \"PowerShell\" pins \"One Half Dark\"";
const REPOINTED_NOTE: &str = "repointed profile \"PowerShell\" from \"One Half Dark\"";
const REPOINTED_PAIR_NOTE: &str =
    "repointed profile \"PowerShell\" from \"onecoat-dark/One Half Dark\"";

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap()
}

fn stamp(path: &Path) -> (u64, SystemTime) {
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.len(), metadata.modified().unwrap())
}

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
    sandbox.install_settings("wt/settings.json");

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
fn use_writes_the_fragment_and_the_settings_file() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    let fragment = sandbox.fragment();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "wrote {}\nwrote {}\n{PIN_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/nord-dark-fragment.json")
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-spliced.json")
    );

    assert!(!append_to_file_name(&fragment, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
}

#[test]
fn a_missing_settings_file_fails_without_writing_the_fragment() {
    let sandbox = Sandbox::new();
    let settings = sandbox.settings();
    for args in [vec!["use", "nord"], vec!["use", "nord", "--dry-run"]] {
        let output = sandbox.run(&args);
        assert_eq!(code(&output), 3, "args {args:?}");
        assert!(stdout(&output).is_empty(), "{}", stdout(&output));
        let message = stderr(&output);

        assert!(
            message.contains(&settings.display().to_string()),
            "{message}"
        );

        assert!(message.contains("retry"), "{message}");
    }

    assert!(!sandbox.fragment().exists());
    assert!(!sandbox.settings().exists());
}

#[test]
fn a_broken_settings_file_is_reported_and_left_alone() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings-broken.json");

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
    let message = stderr(&output);

    assert!(
        message.contains(&settings.display().to_string()),
        "{message}"
    );

    assert!(message.contains("line 9"), "{message}");

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-broken.json")
    );

    assert!(!sandbox.fragment().exists());
    assert!(!append_to_file_name(&settings, ".onecoat.bak").exists());
}

#[test]
fn a_second_use_neither_rewrites_nor_rebacks_up() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let settings = sandbox.install_settings("wt/settings.json");
    let fragment_backup = append_to_file_name(&fragment, ".onecoat.bak");
    let settings_backup = append_to_file_name(&settings, ".onecoat.bak");

    let first = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));

    let stamps = [
        stamp(&fragment),
        stamp(&settings),
        stamp(&fragment_backup),
        stamp(&settings_backup),
    ];

    let second = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));

    assert_eq!(
        stdout(&second),
        format!(
            "unchanged {}\nunchanged {}\n{PIN_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert_eq!(
        std::fs::read(&fragment).unwrap(),
        fixture_bytes("wt/nord-dark-fragment.json")
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-spliced.json")
    );

    assert_eq!(
        std::fs::read(&fragment_backup).unwrap(),
        fixture_bytes("wt/previous-fragment.json")
    );

    assert_eq!(
        std::fs::read(&settings_backup).unwrap(),
        fixture_bytes("wt/settings.json")
    );

    assert_eq!(
        [
            stamp(&fragment),
            stamp(&settings),
            stamp(&fragment_backup),
            stamp(&settings_backup),
        ],
        stamps
    );
}

#[test]
fn use_backs_up_both_files_it_replaces() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let settings = sandbox.install_settings("wt/settings.json");

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    assert_eq!(
        std::fs::read(append_to_file_name(&fragment, ".onecoat.bak")).unwrap(),
        fixture_bytes("wt/previous-fragment.json")
    );

    assert_eq!(
        std::fs::read(append_to_file_name(&settings, ".onecoat.bak")).unwrap(),
        fixture_bytes("wt/settings.json")
    );

    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
}

#[test]
fn a_failed_backup_leaves_the_settings_file_intact() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let settings = sandbox.install_settings("wt/settings.json");
    let backup = append_to_file_name(&fragment, ".onecoat.bak");
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

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings.json"),
        "the settings file is not written when an earlier write fails"
    );

    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
}

#[test]
fn dry_run_names_the_planned_files_and_the_changed_keys() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.fragment();
    let settings = sandbox.install_settings("wt/settings.json");
    let stamp_before = stamp(&settings);

    let output = sandbox.run(&["use", "nord", "--dry-run"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "would write {}\n  schemes\nwould write {}\n  theme\n  themes\n  profiles.defaults.colorScheme\n{PIN_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert!(!fragment.exists(), "dry-run creates nothing");

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings.json")
    );

    assert_eq!(stamp(&settings), stamp_before);
    assert!(!append_to_file_name(&settings, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
}

#[test]
fn dry_run_after_an_apply_reports_nothing_to_do() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/nord-dark-fragment.json");
    let settings = sandbox.install_settings("wt/settings-spliced.json");

    let output = sandbox.run(&["use", "nord", "--dry-run"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "unchanged {}\nunchanged {}\n{PIN_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-spliced.json")
    );

    assert!(!append_to_file_name(&settings, ".onecoat.bak").exists());
}

#[test]
fn targets_limit_the_apply() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    for (value, expected) in [
        ("omp", "no writer for omp\n"),
        ("herdr,omp", "no writer for herdr\nno writer for omp\n"),
    ] {
        let output = sandbox.run(&["use", "nord", "--targets", value]);
        assert_eq!(code(&output), 0, "--targets {value}: {}", stderr(&output));
        assert_eq!(stdout(&output), expected, "--targets {value}");
        assert!(!sandbox.fragment().exists(), "--targets {value}");

        assert_eq!(
            std::fs::read(&settings).unwrap(),
            fixture_bytes("wt/settings.json"),
            "--targets {value}"
        );
    }

    let limited = sandbox.run(&["use", "nord", "--targets", "wt"]);
    assert_eq!(code(&limited), 0, "{}", stderr(&limited));

    assert_eq!(
        stdout(&limited),
        format!(
            "wrote {}\nwrote {}\n{PIN_NOTE}\n",
            sandbox.fragment().display(),
            settings.display()
        )
    );
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
    for args in [
        Vec::new(),
        vec!["use"],
        vec!["bogus"],
        vec!["use", "nord", "--targets", "bogus"],
        vec!["use", "nord", "--unknown-flag"],
    ] {
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
fn writes_stay_inside_the_two_windows_terminal_files() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    let fragment = sandbox.fragment();
    let settings = sandbox.settings();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let allowed = |path: &Path| {
        path.starts_with(fragment.parent().unwrap())
            || path == settings
            || path == append_to_file_name(&settings, ".onecoat.bak")
    };

    let unexpected: Vec<String> = tree(sandbox.root())
        .into_iter()
        .map(|entry| sandbox.root().join(entry))
        .filter(|path| !path.is_dir() && !allowed(path))
        .map(|path| path.display().to_string())
        .collect();

    assert!(unexpected.is_empty(), "unexpected files: {unexpected:?}");
    assert!(fragment.exists());

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-spliced.json")
    );
}

#[test]
fn profile_color_scheme_all_repoints_every_pin() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    let fragment = sandbox.fragment();

    let output = sandbox.run(&["use", "nord", "--profile-color-scheme", "all"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "wrote {}\nwrote {}\n{REPOINTED_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-repointed.json")
    );

    let again = sandbox.run(&["use", "nord", "--profile-color-scheme", "all"]);
    assert_eq!(code(&again), 0, "{}", stderr(&again));

    assert_eq!(
        stdout(&again),
        format!(
            "unchanged {}\nunchanged {}\n{REPOINTED_PAIR_NOTE}\n",
            fragment.display(),
            settings.display()
        )
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-repointed.json")
    );
}
