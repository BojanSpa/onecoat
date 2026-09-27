use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let root = std::env::args_os().nth(1).unwrap_or_else(|| ".".into());
    match tidy::run(Path::new(&root)) {
        Ok(diagnostics) if diagnostics.is_empty() => {
            println!("tidy: {} checks clean", tidy::checks::CHECKS.len());
            ExitCode::SUCCESS
        }
        Ok(diagnostics) => {
            for diagnostic in &diagnostics {
                eprintln!("{diagnostic}");
            }
            eprintln!("tidy: {} findings", diagnostics.len());
            ExitCode::from(1)
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(3)
        }
    }
}
