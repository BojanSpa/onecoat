use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use serde::Serialize;

use onecoat::Error;
use onecoat::doctor;
use onecoat::exec::{self, WriteOutcome};
use onecoat::model::ids::{Appearance, Origin, Slot, Target, ThemeId};
use onecoat::model::theme::{Theme, Validated};
use onecoat::plan::Plan;
use onecoat::render::wt::ProfileScheme;
use onecoat::state::State;
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
    #[command(about = "Print the assigned theme per slot and the targets last applied")]
    Current(ReadArgs),
    #[command(
        about = "Report the resolved paths, the herdr program, the socket, the profile pins, and the state age"
    )]
    Doctor(ReadArgs),
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
    #[arg(
        long,
        value_enum,
        help = "Assign the theme to this slot instead of its declared appearance"
    )]
    slot: Option<Slot>,
    #[arg(
        long,
        help = "Print the planned writes and the keys they change, without writing"
    )]
    dry_run: bool,
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        help = "Apply only to these targets: wt, herdr, omp (default: all)"
    )]
    targets: Vec<Target>,
    #[arg(
        long,
        value_enum,
        default_value = "report",
        help = "Report profiles that pin their own scheme, or `all` to repoint them to the scheme pair"
    )]
    profile_color_scheme: ProfileScheme,
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
        Command::Use(args) => apply(&paths, &args),
        Command::Current(args) => current(&paths, args),
        Command::Doctor(args) => doctor(&paths, args),
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

fn apply(paths: &Paths, args: &UseArgs) -> Result<(), Error> {
    let themes = ThemeSet::load(&paths.user_themes)?;

    let theme = themes
        .get(args.id.as_str())
        .ok_or_else(|| Error::ThemeNotFound {
            id: args.id.clone(),
        })?;

    let slot = args.slot.unwrap_or_else(|| Slot::from(theme.appearance));

    let targets = if args.targets.is_empty() {
        Target::ALL.to_vec()
    } else {
        args.targets.clone()
    };

    let plan = Plan::build(paths, theme, slot, args.profile_color_scheme)?.limited_to(&targets);
    let mut state = exec::read_state(&paths.state)?;
    if args.dry_run {
        let lines = exec::preview(&plan)?;

        println!("would assign {} = {}", slot.name(), theme.id);

        for line in lines {
            println!("{line}");
        }
    } else {
        for report in exec::execute(paths, &plan)? {
            let verb = match report.outcome {
                WriteOutcome::Written => "wrote",
                WriteOutcome::Unchanged => "unchanged",
            };

            println!("{verb} {}", report.path.display());

            for note in &report.pin_notes {
                println!("{note}");
            }
        }

        state.assign(slot, theme.id.clone());
        state.record(plan.targets());
        exec::write_state(&paths.state, &state)?;
    }

    Ok(())
}

fn current(paths: &Paths, args: ReadArgs) -> Result<(), Error> {
    let state = exec::read_state(&paths.state)?;
    if args.json {
        print!("{}", state.render()?);
    } else {
        print_current(&state);
    }

    Ok(())
}

fn doctor(paths: &Paths, args: ReadArgs) -> Result<(), Error> {
    let report = doctor::probe(paths)?;
    if args.json {
        println!("{}", report.json()?);
    } else {
        println!("{}", report.table());
    }

    Ok(())
}

struct CurrentRow {
    slot: &'static str,
    theme: String,
    targets: String,
}

impl CurrentRow {
    fn of(slot: Slot, state: &State, applied: &str) -> Self {
        Self {
            slot: slot.name(),
            theme: state
                .assigned(slot)
                .map_or_else(|| "none".to_owned(), ThemeId::to_string),
            targets: applied.to_owned(),
        }
    }

    fn header() -> Self {
        Self {
            slot: "slot",
            theme: "theme".to_owned(),
            targets: "targets".to_owned(),
        }
    }
}

fn print_current(state: &State) {
    let applied = if state.targets().is_empty() {
        "-".to_owned()
    } else {
        state
            .targets()
            .iter()
            .map(Target::to_string)
            .collect::<Vec<_>>()
            .join(",")
    };

    let rows: Vec<CurrentRow> = std::iter::once(CurrentRow::header())
        .chain(
            [Slot::Dark, Slot::Light]
                .into_iter()
                .map(|slot| CurrentRow::of(slot, state, &applied)),
        )
        .collect();

    let widest = |cell: fn(&CurrentRow) -> &str| {
        rows.iter()
            .map(cell)
            .map(|text| text.chars().count())
            .max()
            .unwrap_or(0)
    };

    let (slot_width, theme_width) = (widest(|row| row.slot), widest(|row| &row.theme));
    for row in &rows {
        let line = format!(
            "{:<slot$} {:<theme$} {}",
            row.slot,
            row.theme,
            row.targets,
            slot = slot_width,
            theme = theme_width
        );

        println!("{}", line.trim_end());
    }
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

struct TableRow {
    id: String,
    appearance: &'static str,
    origin: &'static str,
    name: String,
}

impl TableRow {
    fn of(theme: &Theme<Validated>) -> Self {
        Self {
            id: theme.id.to_string(),
            appearance: Slot::from(theme.appearance).name(),
            origin: origin_name(theme.origin),
            name: theme.name.clone(),
        }
    }

    fn header() -> Self {
        Self {
            id: "id".to_owned(),
            appearance: "appearance",
            origin: "origin",
            name: "name".to_owned(),
        }
    }
}

fn print_table(themes: &ThemeSet) {
    let rows: Vec<TableRow> = std::iter::once(TableRow::header())
        .chain(themes.iter().map(TableRow::of))
        .collect();

    let widest = |cell: fn(&TableRow) -> &str| {
        rows.iter()
            .map(cell)
            .map(|text| text.chars().count())
            .max()
            .unwrap_or(0)
    };

    let (id_width, appearance_width, origin_width) = (
        widest(|row| &row.id),
        widest(|row| row.appearance),
        widest(|row| row.origin),
    );

    for row in &rows {
        let line = format!(
            "{:<id$} {:<appearance$} {:<origin$} {}",
            row.id,
            row.appearance,
            row.origin,
            row.name,
            id = id_width,
            appearance = appearance_width,
            origin = origin_width
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
