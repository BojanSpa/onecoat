use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::SystemTime;

use tempfile::TempDir;

struct Sandbox {
    root: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            root: TempDir::new().unwrap(),
        };

        sandbox.install("herdr/config.toml", &sandbox.herdr_config());

        sandbox
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_onecoat"))
            .args(args)
            .env("LOCALAPPDATA", self.root.path())
            .env("APPDATA", self.root.path())
            .env("PI_CODING_AGENT_DIR", self.agent())
            .env("PATH", self.root.path().join("no-programs"))
            .output()
            .unwrap()
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn agent(&self) -> PathBuf {
        self.root.path().join("omp").join("agent")
    }

    fn omp_theme(&self, slot: &str) -> PathBuf {
        self.agent()
            .join("themes")
            .join(format!("onecoat-{slot}.json"))
    }

    fn omp_config(&self) -> PathBuf {
        self.agent().join("config.yml")
    }

    fn herdr_config(&self) -> PathBuf {
        self.root.path().join("herdr").join("config.toml")
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

    fn state(&self) -> PathBuf {
        self.root.path().join("onecoat").join("state.json")
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

    fn install_config(&self, fixture: &str) -> PathBuf {
        self.install(fixture, &self.omp_config())
    }

    fn install_omp_theme(&self, fixture: &str, slot: &str) -> PathBuf {
        self.install(fixture, &self.omp_theme(slot))
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
const HERDR_SKIPPED: &str = "herdr is not on PATH, so the config check was skipped";
const HERDR_INTERACTIVE: &str = "no herdr server socket, so herdr picks this up at its next launch";
const HERDR_KEYS: &str = "  theme.name\n  theme.auto_switch\n  theme.dark_name\n  theme.light_name\n  theme.custom.accent\n  theme.custom.dark.active_row_bg\n  theme.custom.blue\n  theme.custom.green\n  theme.custom.dark.panel_bg\n  theme.custom.red\n  theme.custom.dark.selection_bg\n  theme.custom.dark.sidebar_bg\n  theme.custom.dark.text\n  theme.custom.yellow\n";
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
        "id        appearance origin  name\n\
         nord      dark       bundled Nord\n\
         nord-grey dark       bundled Nord Grey\n"
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
  },
  {
    "id": "nord-grey",
    "name": "Nord Grey",
    "appearance": "dark",
    "origin": "bundled",
    "derived": true
  }
]
"#;

    assert_eq!(stdout(&output), expected);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();

    assert_eq!(
        parsed,
        serde_json::json!([
            {
                "id": "nord",
                "name": "Nord",
                "appearance": "dark",
                "origin": "bundled",
                "derived": false,
            },
            {
                "id": "nord-grey",
                "name": "Nord Grey",
                "appearance": "dark",
                "origin": "bundled",
                "derived": true,
            }
        ])
    );

    assert!(output.stderr.is_empty(), "{}", stderr(&output));
}

#[test]
fn a_user_theme_shadows_the_bundled_one() {
    let sandbox = Sandbox::new();
    sandbox.install_theme("themes/shadow-nord.toml", "shadow-nord.toml");
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");

    let listed = sandbox.run(&["list", "--json"]);
    assert_eq!(code(&listed), 0);
    let rows: serde_json::Value = serde_json::from_str(&stdout(&listed)).unwrap();
    let rows = rows.as_array().unwrap();
    let shadowed: Vec<&serde_json::Value> = rows.iter().filter(|row| row["id"] == "nord").collect();
    assert_eq!(shadowed.len(), 1, "{rows:?}");
    assert_eq!(shadowed[0]["name"], "Shadow Nord");
    assert_eq!(shadowed[0]["origin"], "user");

    let human = sandbox.run(&["list"]);
    let human = stdout(&human);

    let lines: Vec<&str> = human
        .lines()
        .filter(|line| line.split_whitespace().next() == Some("nord"))
        .collect();

    assert_eq!(lines.len(), 1, "{human}");

    assert!(
        lines[0].contains("user") && lines[0].contains("Shadow Nord"),
        "{}",
        lines[0]
    );

    let applied = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&applied), 0, "{}", stderr(&applied));
    let fragment = std::fs::read(sandbox.fragment()).unwrap();
    let fragment: serde_json::Value = serde_json::from_slice(&fragment).unwrap();
    assert_eq!(fragment["schemes"][0]["background"], "#101820");
}

#[test]
fn use_writes_every_target() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");
    let fragment = sandbox.fragment();
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.omp_config();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "wrote {}\nwrote {}\n{PIN_NOTE}\nwrote {}\n{HERDR_SKIPPED}\n{HERDR_INTERACTIVE}\nwrote {}\nwrote {}\n",
            fragment.display(),
            settings.display(),
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
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
        std::fs::read(&omp_theme).unwrap(),
        fixture_bytes("omp/onecoat-dark.json")
    );

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config-spliced.yml")
    );

    assert_eq!(
        std::fs::read(sandbox.herdr_config()).unwrap(),
        fixture_bytes("herdr/config-spliced.toml")
    );

    assert_eq!(
        std::fs::read(append_to_file_name(&omp_config, ".onecoat.bak")).unwrap(),
        fixture_bytes("omp/config.yml")
    );

    assert!(!append_to_file_name(&fragment, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&omp_theme, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&omp_theme, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&omp_config, ".onecoat.tmp").exists());
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
    assert!(!sandbox.omp_theme("dark").exists());
    assert!(!sandbox.omp_config().exists());
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
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.install_config("omp/config.yml");
    let herdr_config = sandbox.herdr_config();
    let state = sandbox.state();
    let fragment_backup = append_to_file_name(&fragment, ".onecoat.bak");
    let settings_backup = append_to_file_name(&settings, ".onecoat.bak");
    let config_backup = append_to_file_name(&omp_config, ".onecoat.bak");
    let herdr_backup = append_to_file_name(&herdr_config, ".onecoat.bak");

    let first = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));

    let stamps = [
        stamp(&fragment),
        stamp(&settings),
        stamp(&herdr_config),
        stamp(&omp_theme),
        stamp(&omp_config),
        stamp(&state),
        stamp(&fragment_backup),
        stamp(&settings_backup),
        stamp(&herdr_backup),
        stamp(&config_backup),
    ];

    let second = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));

    assert_eq!(
        stdout(&second),
        format!(
            "unchanged {}\nunchanged {}\n{PIN_NOTE}\nunchanged {}\nunchanged {}\nunchanged {}\n",
            fragment.display(),
            settings.display(),
            herdr_config.display(),
            omp_theme.display(),
            omp_config.display()
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
        std::fs::read(&omp_theme).unwrap(),
        fixture_bytes("omp/onecoat-dark.json")
    );

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config-spliced.yml")
    );

    assert_eq!(
        std::fs::read(&herdr_config).unwrap(),
        fixture_bytes("herdr/config-spliced.toml")
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
        std::fs::read(&config_backup).unwrap(),
        fixture_bytes("omp/config.yml")
    );

    assert_eq!(
        std::fs::read_to_string(&state).unwrap(),
        "{\n  \"slots\": {\n    \"dark\": \"nord\",\n    \"light\": null\n  },\n  \"targets\": [\n    \"wt\",\n    \"herdr\",\n    \"omp\"\n  ]\n}\n"
    );

    assert_eq!(
        [
            stamp(&fragment),
            stamp(&settings),
            stamp(&herdr_config),
            stamp(&omp_theme),
            stamp(&omp_config),
            stamp(&state),
            stamp(&fragment_backup),
            stamp(&settings_backup),
            stamp(&herdr_backup),
            stamp(&config_backup),
        ],
        stamps
    );
}

#[test]
fn a_theme_rewrite_leaves_the_pin_untouched() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    let omp_config = sandbox.install_config("omp/config.yml");
    let omp_theme = sandbox.omp_theme("dark");

    let first = sandbox.run(&["use", "nord", "--targets", "omp"]);
    assert_eq!(code(&first), 0, "{}", stderr(&first));

    let nord: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&omp_theme).unwrap()).unwrap();

    assert_eq!(nord["colors"]["toolPendingBg"], "#0b1018");

    let theme_stamp = stamp(&omp_theme);
    let config_bytes = std::fs::read(&omp_config).unwrap();
    let config_stamp = stamp(&omp_config);

    sandbox.install_theme("themes/shadow-nord.toml", "shadow-nord.toml");

    let second = sandbox.run(&["use", "nord", "--targets", "omp"]);
    assert_eq!(code(&second), 0, "{}", stderr(&second));

    assert_eq!(
        stdout(&second),
        format!(
            "wrote {}\nunchanged {}\n",
            omp_theme.display(),
            omp_config.display()
        )
    );

    let shadow: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&omp_theme).unwrap()).unwrap();

    assert_eq!(shadow["colors"]["toolPendingBg"], "#101820");
    assert_ne!(stamp(&omp_theme), theme_stamp);
    assert_eq!(std::fs::read(&omp_config).unwrap(), config_bytes);
    assert_eq!(stamp(&omp_config), config_stamp);

    let rewritten = stamp(&omp_theme);

    let third = sandbox.run(&["use", "nord", "--targets", "omp"]);
    assert_eq!(code(&third), 0, "{}", stderr(&third));

    assert_eq!(
        stdout(&third),
        format!(
            "unchanged {}\nunchanged {}\n",
            omp_theme.display(),
            omp_config.display()
        )
    );

    assert_eq!(stamp(&omp_theme), rewritten);
    assert_eq!(stamp(&omp_config), config_stamp);
}

#[test]
fn use_backs_up_every_file_it_replaces() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let settings = sandbox.install_settings("wt/settings.json");
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.install_config("omp/config.yml");

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

    assert_eq!(
        std::fs::read(append_to_file_name(&omp_config, ".onecoat.bak")).unwrap(),
        fixture_bytes("omp/config.yml")
    );

    assert!(!append_to_file_name(&omp_theme, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&omp_config, ".onecoat.tmp").exists());
}

#[test]
fn a_failed_backup_leaves_the_settings_file_intact() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/previous-fragment.json");
    let settings = sandbox.install_settings("wt/settings.json");
    let omp_config = sandbox.install_config("omp/config.yml");
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

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config.yml"),
        "the omp config is not written when an earlier write fails"
    );

    assert!(!sandbox.omp_theme("dark").exists());
    assert!(!append_to_file_name(&fragment, ".onecoat.tmp").exists());
}

#[test]
fn dry_run_names_the_planned_files_and_the_changed_keys() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.fragment();
    let settings = sandbox.install_settings("wt/settings.json");
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.install_config("omp/config.yml");
    let stamp_before = stamp(&settings);

    let output = sandbox.run(&["use", "nord", "--dry-run"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));
    assert!(output.stderr.is_empty(), "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "would assign dark = nord\nwould write {}\n  schemes\nwould write {}\n  theme\n  themes\n  profiles.defaults.colorScheme\n{PIN_NOTE}\nwould write {}\n{HERDR_KEYS}would write {}\nwould write {}\n  theme.dark\n",
            fragment.display(),
            settings.display(),
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
        )
    );

    assert!(!fragment.exists(), "dry-run creates nothing");
    assert!(!sandbox.state().exists(), "dry-run writes no state");
    assert!(!omp_theme.exists(), "dry-run creates no omp theme file");

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config.yml")
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings.json")
    );

    assert_eq!(stamp(&settings), stamp_before);
    assert!(!append_to_file_name(&settings, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&settings, ".onecoat.tmp").exists());
    assert!(!append_to_file_name(&omp_config, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&omp_config, ".onecoat.tmp").exists());
}

#[test]
fn dry_run_after_an_apply_reports_nothing_to_do() {
    let sandbox = Sandbox::new();
    let fragment = sandbox.install_fragment("wt/nord-dark-fragment.json");
    let settings = sandbox.install_settings("wt/settings-spliced.json");
    let omp_theme = sandbox.install_omp_theme("omp/onecoat-dark.json", "dark");
    let omp_config = sandbox.install_config("omp/config-spliced.yml");
    sandbox.install("herdr/config-spliced.toml", &sandbox.herdr_config());

    let output = sandbox.run(&["use", "nord", "--dry-run"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "would assign dark = nord\nunchanged {}\nunchanged {}\n{PIN_NOTE}\nunchanged {}\nunchanged {}\nunchanged {}\n",
            fragment.display(),
            settings.display(),
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
        )
    );

    assert!(!sandbox.state().exists(), "dry-run writes no state");

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-spliced.json")
    );

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config-spliced.yml")
    );

    assert!(!append_to_file_name(&settings, ".onecoat.bak").exists());
    assert!(!append_to_file_name(&omp_config, ".onecoat.bak").exists());
}

#[test]
fn targets_limit_the_apply() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.install_config("omp/config.yml");

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

    assert!(sandbox.fragment().exists());

    assert!(
        !omp_theme.exists(),
        "--targets wt leaves the omp target alone"
    );

    assert_eq!(
        std::fs::read(&omp_config).unwrap(),
        fixture_bytes("omp/config.yml"),
        "--targets wt leaves the omp target alone"
    );

    let omp = sandbox.run(&["use", "nord", "--targets", "omp"]);
    assert_eq!(code(&omp), 0, "{}", stderr(&omp));

    assert_eq!(
        stdout(&omp),
        format!(
            "wrote {}\nwrote {}\n",
            omp_theme.display(),
            omp_config.display()
        )
    );

    assert_eq!(
        std::fs::read(&omp_theme).unwrap(),
        fixture_bytes("omp/onecoat-dark.json")
    );

    assert_eq!(
        std::fs::read_to_string(sandbox.state()).unwrap(),
        "{\n  \"slots\": {\n    \"dark\": \"nord\",\n    \"light\": null\n  },\n  \"targets\": [\n    \"omp\"\n  ]\n}\n"
    );

    let herdr = sandbox.run(&["use", "nord", "--targets", "herdr,omp"]);
    assert_eq!(code(&herdr), 0, "{}", stderr(&herdr));

    assert_eq!(
        stdout(&herdr),
        format!(
            "wrote {}\n{HERDR_SKIPPED}\n{HERDR_INTERACTIVE}\nunchanged {}\nunchanged {}\n",
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
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
fn writes_stay_inside_the_target_files_and_onecoats_own_directory() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");
    let fragment = sandbox.fragment();
    let settings = sandbox.settings();
    let agent = sandbox.agent();
    let herdr_config = sandbox.herdr_config();
    let state = sandbox.state();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let allowed = |path: &Path| {
        path.starts_with(fragment.parent().unwrap())
            || path == settings
            || path == append_to_file_name(&settings, ".onecoat.bak")
            || path == herdr_config
            || path == append_to_file_name(&herdr_config, ".onecoat.bak")
            || path.starts_with(&agent)
            || path == state
    };

    let unexpected: Vec<String> = tree(sandbox.root())
        .into_iter()
        .map(|entry| sandbox.root().join(entry))
        .filter(|path| !path.is_dir() && !allowed(path))
        .map(|path| path.display().to_string())
        .collect();

    assert!(unexpected.is_empty(), "unexpected files: {unexpected:?}");
    assert!(fragment.exists());
    assert!(state.exists());

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
    let omp_theme = sandbox.omp_theme("dark");
    let omp_config = sandbox.install_config("omp/config.yml");

    let output = sandbox.run(&["use", "nord", "--profile-color-scheme", "all"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    assert_eq!(
        stdout(&output),
        format!(
            "wrote {}\nwrote {}\n{REPOINTED_NOTE}\nwrote {}\n{HERDR_SKIPPED}\n{HERDR_INTERACTIVE}\nwrote {}\nwrote {}\n",
            fragment.display(),
            settings.display(),
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
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
            "unchanged {}\nunchanged {}\n{REPOINTED_PAIR_NOTE}\nunchanged {}\nunchanged {}\nunchanged {}\n",
            fragment.display(),
            settings.display(),
            sandbox.herdr_config().display(),
            omp_theme.display(),
            omp_config.display()
        )
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings-repointed.json")
    );
}

#[test]
fn current_reports_no_assignment_before_any_use() {
    let sandbox = Sandbox::new();

    let human = sandbox.run(&["current"]);
    assert_eq!(code(&human), 0, "{}", stderr(&human));

    assert_eq!(
        stdout(&human),
        "slot  theme targets\ndark  none  -\nlight none  -\n"
    );

    assert!(human.stderr.is_empty(), "{}", stderr(&human));

    let machine = sandbox.run(&["current", "--json"]);
    assert_eq!(code(&machine), 0, "{}", stderr(&machine));

    assert_eq!(
        stdout(&machine),
        "{\n  \"slots\": {\n    \"dark\": null,\n    \"light\": null\n  },\n  \"targets\": []\n}\n"
    );
}

#[test]
fn use_slot_records_the_assignment_and_keeps_the_other_slot() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");
    let state = sandbox.state();

    let light = sandbox.run(&["use", "nord", "--slot", "light"]);
    assert_eq!(code(&light), 0, "{}", stderr(&light));
    assert!(light.stderr.is_empty(), "{}", stderr(&light));

    assert_eq!(
        std::fs::read_to_string(&state).unwrap(),
        "{\n  \"slots\": {\n    \"dark\": null,\n    \"light\": \"nord\"\n  },\n  \"targets\": [\n    \"wt\",\n    \"herdr\",\n    \"omp\"\n  ]\n}\n"
    );

    let dark = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&dark), 0, "{}", stderr(&dark));

    assert_eq!(
        std::fs::read_to_string(&state).unwrap(),
        "{\n  \"slots\": {\n    \"dark\": \"nord\",\n    \"light\": \"nord\"\n  },\n  \"targets\": [\n    \"wt\",\n    \"herdr\",\n    \"omp\"\n  ]\n}\n"
    );

    let human = sandbox.run(&["current"]);
    assert_eq!(code(&human), 0, "{}", stderr(&human));

    assert_eq!(
        stdout(&human),
        "slot  theme targets\ndark  nord  wt,herdr,omp\nlight nord  wt,herdr,omp\n"
    );

    let machine = sandbox.run(&["current", "--json"]);
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&machine)).unwrap();

    assert_eq!(
        parsed,
        serde_json::json!({
            "slots": {"dark": "nord", "light": "nord"},
            "targets": ["wt", "herdr", "omp"],
        })
    );
}

#[test]
fn a_malformed_state_file_fails_the_commands_that_read_it() {
    let sandbox = Sandbox::new();
    let settings = sandbox.install_settings("wt/settings.json");
    let state = sandbox.state();
    std::fs::create_dir_all(state.parent().unwrap()).unwrap();
    std::fs::write(&state, "{\"slots\": {\"dark\": 7}, \"targets\": []}\n").unwrap();

    for args in [vec!["current"], vec!["use", "nord"]] {
        let output = sandbox.run(&args);
        assert_eq!(code(&output), 3, "args {args:?}");
        assert!(stdout(&output).is_empty(), "args {args:?}");

        let message = stderr(&output);

        assert!(message.contains(&state.display().to_string()), "{message}");
        assert!(message.contains("slots.dark"), "{message}");
        assert!(message.contains("delete"), "{message}");
    }

    assert!(
        !sandbox.fragment().exists(),
        "the apply fails before it writes"
    );

    assert_eq!(
        std::fs::read(&settings).unwrap(),
        fixture_bytes("wt/settings.json")
    );
}

#[test]
fn use_slot_light_writes_its_own_file_and_pin() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");

    let output = sandbox.run(&["use", "nord", "--slot", "light"]);
    assert_eq!(code(&output), 0, "{}", stderr(&output));

    let theme = sandbox.omp_theme("light");
    let light: serde_json::Value = serde_json::from_slice(&std::fs::read(&theme).unwrap()).unwrap();

    let dark: serde_json::Value =
        serde_json::from_slice(&fixture_bytes("omp/onecoat-dark.json")).unwrap();

    assert_eq!(light["name"], "onecoat-light");
    assert_eq!(light["colors"], dark["colors"]);

    assert_eq!(
        std::fs::read(sandbox.omp_config()).unwrap(),
        fixture_bytes("omp/config-light.yml")
    );

    assert!(!sandbox.omp_theme("dark").exists());
}

#[test]
fn a_missing_config_file_fails_without_writing_the_theme_file() {
    let sandbox = Sandbox::new();
    sandbox.install_settings("wt/settings.json");
    let config = sandbox.omp_config();

    let output = sandbox.run(&["use", "nord"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));

    let message = stderr(&output);
    assert!(message.contains(&config.display().to_string()), "{message}");
    assert!(message.contains("retry"), "{message}");

    assert!(!sandbox.omp_theme("dark").exists());
    assert!(!sandbox.fragment().exists());
}

#[test]
fn an_unknown_omp_token_fails_the_apply() {
    let sandbox = Sandbox::new();
    let theme = sandbox.install_theme("themes/unknown-omp-token.toml", "unknown-omp-token.toml");
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");

    let output = sandbox.run(&["use", "unknown-omp-token"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));

    let message = stderr(&output);
    assert!(message.contains(&theme.display().to_string()), "{message}");
    assert!(message.contains("bogusToken"), "{message}");
    assert!(message.contains("[targets.omp]"), "{message}");

    assert!(
        !sandbox.fragment().exists(),
        "the apply fails before it writes"
    );
}

#[test]
fn a_text_override_on_an_omp_token_fails_the_apply() {
    let sandbox = Sandbox::new();
    let theme = sandbox.install_theme("themes/text-omp-override.toml", "text-omp-override.toml");
    sandbox.install_settings("wt/settings.json");
    sandbox.install_config("omp/config.yml");

    let output = sandbox.run(&["use", "text-omp-override"]);
    assert_eq!(code(&output), 3, "{}", stderr(&output));

    let message = stderr(&output);
    assert!(message.contains(&theme.display().to_string()), "{message}");
    assert!(message.contains("targets.omp.text"), "{message}");
    assert!(message.contains("#rrggbb"), "{message}");
}
