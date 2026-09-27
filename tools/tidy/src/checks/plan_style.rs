use crate::{Check, Diagnostic, Error, Repo, Source};

const PLANS: &[&str] = &["work/plans"];
const EXEMPT: &[&str] = &["work/plans/vs1-wt-scheme-fragment.md"];
const MAX_WORDS: usize = 20;

pub struct PlanStyle;

impl Check for PlanStyle {
    fn id(&self) -> &'static str {
        "plan-style"
    }

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error> {
        let mut found = Vec::new();
        for source in repo.sources(PLANS, "md")? {
            if EXEMPT.contains(&source.path.as_str()) {
                continue;
            }
            found.extend(violations(self.id(), &source));
        }
        Ok(found)
    }
}

fn violations(check: &'static str, source: &Source) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    let mut fenced = false;
    let mut commented = false;
    for (index, line) in source.text.lines().enumerate() {
        let (visible, inside) = strip_comments(line, commented);
        commented = inside;
        let trimmed = visible.trim();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if visible.trim_end().ends_with("<br>") {
            found.extend(long_sentences(check, &source.path, index + 1, &visible));
        } else {
            found.push(Diagnostic {
                path: source.path.clone(),
                line: index + 1,
                column: visible.trim_end().chars().count() + 1,
                check,
                message: "a plan line ends with an explicit <br> (AGENTS.md)".to_owned(),
            });
        }
    }
    found
}

fn long_sentences(check: &'static str, path: &str, line: usize, text: &str) -> Vec<Diagnostic> {
    let mut found = Vec::new();
    let mut column = 1;
    for sentence in text.split("<br>") {
        let words = count_words(sentence);
        if words > MAX_WORDS {
            found.push(Diagnostic {
                path: path.to_owned(),
                line,
                column,
                check,
                message: format!(
                    "a plan sentence runs past {MAX_WORDS} words, and this one has {words} (AGENTS.md); split it"
                ),
            });
        }
        column += sentence.chars().count() + "<br>".len();
    }
    found
}

fn count_words(sentence: &str) -> usize {
    sentence
        .split_whitespace()
        .filter(|word| word.chars().any(char::is_alphanumeric))
        .count()
}

fn strip_comments(line: &str, mut inside: bool) -> (String, bool) {
    let mut visible = String::new();
    let mut rest = line;
    loop {
        if inside {
            match rest.find("-->") {
                Some(at) => {
                    visible.push_str(&blank(&rest[..at + 3]));
                    rest = &rest[at + 3..];
                    inside = false;
                }
                None => {
                    visible.push_str(&blank(rest));
                    return (visible, true);
                }
            }
        } else {
            match rest.find("<!--") {
                Some(at) => {
                    visible.push_str(&rest[..at]);
                    visible.push_str(&blank(&rest[at..at + 4]));
                    rest = &rest[at + 4..];
                    inside = true;
                }
                None => {
                    visible.push_str(rest);
                    return (visible, false);
                }
            }
        }
    }
}

fn blank(text: &str) -> String {
    " ".repeat(text.chars().count())
}
