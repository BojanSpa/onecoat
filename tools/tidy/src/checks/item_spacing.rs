use crate::scan::scan;
use crate::{Check, Diagnostic, Error, Repo};

const SOURCES: &[&str] = &["src", "tests", "tools/tidy/src", "tools/tidy/tests"];
const ITEMS: [&str; 12] = [
    "const",
    "enum",
    "fn",
    "impl",
    "macro_rules!",
    "mod",
    "static",
    "struct",
    "trait",
    "type",
    "union",
    "use",
];

pub struct ItemSpacing;

impl Check for ItemSpacing {
    fn id(&self) -> &'static str {
        "item-spacing"
    }

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error> {
        let mut found = Vec::new();
        for source in repo.sources(SOURCES, "rs")? {
            let lines: Vec<&str> = source.text.lines().collect();
            let hidden = scan(&source.text).hidden;
            for (index, line) in lines.iter().enumerate() {
                let Some(next) = lines.get(index + 1) else {
                    continue;
                };
                if line.trim() != "}" || hidden.get(index + 1).copied().unwrap_or(true) {
                    continue;
                }
                if next.trim().is_empty() {
                    continue;
                }
                let indent = indentation(line);
                if !opens_an_item(&lines, index + 1, indent) {
                    continue;
                }
                found.push(Diagnostic {
                    path: source.path.clone(),
                    line: index + 2,
                    column: indent + 1,
                    check: self.id(),
                    message: "an item starts right after a closing brace; separate items with one blank line (AGENTS.md)".to_owned(),
                });
            }
        }
        Ok(found)
    }
}

fn opens_an_item(lines: &[&str], index: usize, indent: usize) -> bool {
    let Some(line) = lines.get(index) else {
        return false;
    };
    if indentation(line) != indent {
        return false;
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with("#[") {
        return opens_an_item(lines, index + 1, indent);
    }
    starts_item(trimmed)
}

fn starts_item(line: &str) -> bool {
    let mut words = line.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if ITEMS.contains(&word) {
            return true;
        }
        if !is_modifier(word) {
            return false;
        }
        if word == "extern" && words.peek().is_some_and(|next| next.starts_with('"')) {
            words.next();
        }
    }
    false
}

fn is_modifier(word: &str) -> bool {
    matches!(word, "async" | "extern" | "pub" | "unsafe") || word.starts_with("pub(")
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
