//! onecoat's command-line surface: `list` and `use`.
//!
//! Data goes to stdout and diagnostics go to stderr, so a caller can consume either
//! stream (R-41). clap exits 2 on a usage error, this binary exits 3 on any failure
//! reported by the library, and exit code 1 is reserved for `verify`.

use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use onecoat::Error;
use onecoat::exec::{self, WriteOutcome};
use onecoat::model::ids::{Appearance, Origin, Slot, ThemeId};
use onecoat::model::theme::{Theme, Validated};
use onecoat::plan::Plan;
use onecoat::render::wt::WtFragment;
use onecoat::targets::Paths;
use onecoat::themes::ThemeSet;

#[derive(Parser)]
#[command(
    name = "onecoat",
    version,
    about = "Apply one canonical theme to Windows Terminal, Herdr, and the omp harness",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "List the available themes and where each one comes from")]
    List(ReadArgs),
    #[command(about = "Apply a theme to the current slot")]
    Use(UseArgs),
}

#[derive(Args)]
struct ReadArgs {
    #[arg(long, help = "Print machine-readable JSON on stdout")]
    json: bool,
}

#[derive(Args)]
struct UseArgs {
    #[arg(help = "Theme id, as printed by `onecoat list`")]
    id: ThemeId,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(3)
        }
    }
}

fn run(cli: Cli) -> Result<(), Error> {
    let paths = Paths::resolve()?;
    match cli.command {
        Command::List(args) => list(&paths, args),
        Command::Use(args) => apply(&paths, &args.id),
    }
}

fn list(paths: &Paths, args: ReadArgs) -> Result<(), Error> {
    let themes = ThemeSet::load(&paths.user_themes)?;
    if args.json {
        print_json(&themes)
    } else {
        print_table(&themes);
        Ok(())
    }
}

fn apply(paths: &Paths, id: &ThemeId) -> Result<(), Error> {
    let themes = ThemeSet::load(&paths.user_themes)?;
    let theme = themes
        .get(id.as_str())
        .ok_or_else(|| Error::ThemeNotFound { id: id.clone() })?;
    let plan = Plan::fragment(&paths.wt_fragment, &WtFragment::for_theme(theme))?;
    for report in exec::execute(&plan)? {
        let verb = match report.outcome {
            WriteOutcome::Written => "wrote",
            WriteOutcome::Unchanged => "unchanged",
        };
        println!("{verb} {}", report.path.display());
    }
    Ok(())
}

#[derive(Serialize)]
struct ListRow {
    id: ThemeId,
    name: String,
    appearance: Appearance,
    origin: Origin,
    derived: bool,
}

impl ListRow {
    fn of(theme: &Theme<Validated>) -> Self {
        Self {
            id: theme.id.clone(),
            name: theme.name.clone(),
            appearance: theme.appearance,
            origin: theme.origin,
            derived: theme.derived,
        }
    }
}

fn print_json(themes: &ThemeSet) -> Result<(), Error> {
    let rows: Vec<ListRow> = themes.iter().map(ListRow::of).collect();
    let document =
        serde_json::to_string_pretty(&rows).map_err(|source| Error::JsonEncodeFailed { source })?;
    println!("{document}");
    Ok(())
}

/// Pads the first three columns to their widest cell; `name` is last and bare, so no
/// line has a trailing space.
fn print_table(themes: &ThemeSet) {
    let mut rows: Vec<[String; 4]> = vec![[
        "id".to_owned(),
        "appearance".to_owned(),
        "origin".to_owned(),
        "name".to_owned(),
    ]];
    for theme in themes.iter() {
        rows.push([
            theme.id.to_string(),
            Slot::from(theme.appearance).name().to_owned(),
            origin_name(theme.origin).to_owned(),
            theme.name.clone(),
        ]);
    }
    let widths =
        [0, 1, 2].map(|column| rows.iter().map(|row| row[column].len()).max().unwrap_or(0));
    for row in &rows {
        let line = format!(
            "{:<id$} {:<appearance$} {:<origin$} {}",
            row[0],
            row[1],
            row[2],
            row[3],
            id = widths[0],
            appearance = widths[1],
            origin = widths[2]
        );
        println!("{}", line.trim_end());
    }
}

fn origin_name(origin: Origin) -> &'static str {
    match origin {
        Origin::Bundled => "bundled",
        Origin::User => "user",
    }
}
