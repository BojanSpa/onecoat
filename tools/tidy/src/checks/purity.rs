use crate::{Check, Diagnostic, Error, Repo};

const IMPURE: &[&str] = &[
    "src/exec.rs",
    "src/main.rs",
    "src/targets.rs",
    "src/themes.rs",
];

const REACHES: &[&str] = &["env", "fs", "process"];

pub struct Purity;

impl Check for Purity {
    fn id(&self) -> &'static str {
        "purity"
    }

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error> {
        for entry in IMPURE {
            if !repo.exists(entry) {
                return Err(Error::CheckTargetMissing { path: entry.into() });
            }
        }

        let mut found = Vec::new();
        for source in repo.sources(&["src"], "rs")? {
            if IMPURE.contains(&source.path.as_str()) {
                continue;
            }
            for (index, line) in source.text.lines().enumerate() {
                let Some(reached) = REACHES.iter().copied().find(|name| reaches(line, name)) else {
                    continue;
                };
                found.push(Diagnostic {
                    path: source.path.clone(),
                    line: index + 1,
                    column: line.find("std::").map_or(1, |at| at + 1),
                    check: self.id(),
                    message: format!(
                        "`std::{reached}` is IO or the environment and only the edge touches those (N-5); move it next to `src/exec.rs` or `src/targets.rs` (docs/architecture.md)"
                    ),
                });
            }
        }
        Ok(found)
    }
}

fn reaches(line: &str, name: &str) -> bool {
    line.contains(&format!("std::{name}")) || in_braces(line, name)
}

fn in_braces(line: &str, name: &str) -> bool {
    let Some(start) = line.find("std::{") else {
        return false;
    };
    let rest = &line[start + "std::{".len()..];
    let group = rest.split('}').next().unwrap_or(rest);
    group
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .any(|token| token == name)
}
