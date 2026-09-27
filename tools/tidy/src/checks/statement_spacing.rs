use crate::scan::scan;
use crate::{Check, Diagnostic, Error, Repo};

const SOURCES: &[&str] = &["src", "tests", "tools/tidy/src", "tools/tidy/tests"];
const BODIES: [&str; 6] = ["else", "for", "fn", "if", "loop", "while"];
const BLOCK_STARTS: [&str; 7] = ["else", "fn", "for", "if", "loop", "match", "while"];
const MODIFIERS: [&str; 5] = ["async", "const", "extern", "pub", "unsafe"];
const BACK_LIMIT: usize = 40;
const MISSING: &str =
    "separate statements with one blank line when either one spans lines (AGENTS.md)";
const MISSING_TAIL: &str =
    "a tail expression needs one blank line above a statement that spans lines (AGENTS.md)";
const GLUED: &str =
    "a single-line `let` stays glued to the block below it; remove this blank line (AGENTS.md)";

pub struct StatementSpacing;

impl Check for StatementSpacing {
    fn id(&self) -> &'static str {
        "statement-spacing"
    }

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error> {
        let mut found = Vec::new();
        for source in repo.sources(SOURCES, "rs")? {
            let text = scan(&source.text);
            let lines: Vec<&str> = text.code.lines().collect();
            for note in notes(&lines) {
                found.push(Diagnostic {
                    path: source.path.clone(),
                    line: note.line,
                    column: note.column,
                    check: self.id(),
                    message: note.message.to_owned(),
                });
            }
        }

        Ok(found)
    }
}

struct Note {
    line: usize,
    column: usize,
    message: &'static str,
}

#[derive(Clone, Copy)]
struct Block {
    open: usize,
    close: usize,
    depth: usize,
    holds_statements: bool,
}

fn notes(code: &[&str]) -> Vec<Note> {
    let delta = deltas(code);
    let depths = depths(&delta);
    let mut found = Vec::new();
    for block in blocks(code) {
        if !block.holds_statements {
            continue;
        }

        let body = statements(code, &block, &depths, &delta);
        for position in 1..body.len() {
            let (previous_open, previous_close) = body[position - 1];
            let (open, close) = body[position];
            let gap = open - previous_close;
            if glued(code, previous_open, previous_close, open) {
                if gap > 1 {
                    found.push(Note {
                        line: previous_close + 2,
                        column: 1,
                        message: GLUED,
                    });
                }

                continue;
            }

            if gap == 1 && (previous_close > previous_open || close > open) {
                let tail = position + 1 == body.len() && !ends_a_statement(code[close]);

                found.push(Note {
                    line: open + 1,
                    column: indentation(code[open]) + 1,
                    message: if tail { MISSING_TAIL } else { MISSING },
                });
            }
        }
    }

    sorted(found)
}

fn sorted(mut found: Vec<Note>) -> Vec<Note> {
    found.sort_by_key(|note| (note.line, note.column));
    found
}

fn glued(code: &[&str], previous_open: usize, previous_close: usize, open: usize) -> bool {
    previous_open == previous_close
        && code[previous_open].trim_start().starts_with("let ")
        && head_word(code[open]).is_some_and(|head| BLOCK_STARTS.contains(&head))
}

fn blocks(code: &[&str]) -> Vec<Block> {
    let mut open: Vec<Block> = Vec::new();
    let mut found = Vec::new();
    let mut depth = 0usize;
    for (index, line) in code.iter().enumerate() {
        for (column, ch) in line.char_indices() {
            match ch {
                '{' => {
                    open.push(Block {
                        open: index,
                        close: index,
                        depth,
                        holds_statements: opens_statements(code, index, column),
                    });
                    depth += 1;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    if let Some(mut block) = open.pop_if(|block| block.depth == depth) {
                        block.close = index;
                        found.push(block);
                    }
                }
                _ => {}
            }
        }
    }

    found
}

fn statements(
    code: &[&str],
    block: &Block,
    depths: &[usize],
    delta: &[isize],
) -> Vec<(usize, usize)> {
    let level = block.depth + 1;
    let mut body = Vec::new();
    let mut current = None;
    for index in (block.open + 1)..block.close {
        let trimmed = code[index].trim();
        if current.is_none() && depths[index] == level && starts_a_statement(trimmed) {
            current = Some(index);
        }

        if let Some(open) = current
            && depth_after(depths[index], delta[index]) == level
            && ends_a_statement(trimmed)
        {
            body.push((open, index));
            current = None;
        }
    }

    if let Some(open) = current {
        body.push((open, block.close - 1));
    }

    body
}

fn opens_statements(code: &[&str], line: usize, column: usize) -> bool {
    construct_head(code, line, column).is_some_and(|head| BODIES.contains(&head))
}

fn construct_head<'a>(code: &[&'a str], line: usize, column: usize) -> Option<&'a str> {
    let before = code[line][..column].trim();
    if let Some(head) = head_word(before).filter(|head| BODIES.contains(head)) {
        return Some(head);
    }

    if before.is_empty() || continues(before) {
        return walk_back(code, line);
    }

    None
}

fn walk_back<'a>(code: &[&'a str], line: usize) -> Option<&'a str> {
    let mut back = line;
    while back > 0 && line - back < BACK_LIMIT {
        back -= 1;
        let previous = code[back].trim();
        if let Some(head) = head_word(previous).filter(|head| BODIES.contains(head)) {
            return Some(head);
        }

        if previous.is_empty() {
            continue;
        }

        if previous.ends_with(['{', '}', ';']) {
            break;
        }
    }

    None
}

fn head_word(text: &str) -> Option<&str> {
    let mut words = attributes(text)?.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if word == "}" {
            continue;
        }

        if MODIFIERS.contains(&word) || word.starts_with("pub(") {
            if word == "extern" && words.peek().is_some_and(|next| next.starts_with('"')) {
                words.next();
            }

            continue;
        }

        return Some(word.split_once('<').map_or(word, |(head, _)| head));
    }

    None
}

fn attributes(text: &str) -> Option<&str> {
    let mut rest = text.trim();
    while rest.starts_with("#[") || rest.starts_with("#!") {
        rest = rest[rest.find(']')? + 1..].trim();
    }

    Some(rest)
}

fn continues(before: &str) -> bool {
    bracket_balance(before) < 0
        || before.starts_with([')', ','])
        || before.ends_with("->")
        || before.ends_with("where")
        || before.ends_with(',')
}

fn bracket_balance(text: &str) -> isize {
    text.chars().fold(0, |balance, ch| match ch {
        '(' | '[' => balance + 1,
        ')' | ']' => balance - 1,
        _ => balance,
    })
}

fn deltas(code: &[&str]) -> Vec<isize> {
    code.iter()
        .map(|line| line.matches('{').count() as isize - line.matches('}').count() as isize)
        .collect()
}

fn depths(delta: &[isize]) -> Vec<usize> {
    let mut found = Vec::with_capacity(delta.len());
    let mut depth = 0isize;
    for change in delta {
        found.push(depth.max(0) as usize);
        depth += change;
    }

    found
}

fn depth_after(depth: usize, change: isize) -> usize {
    (depth as isize + change).max(0) as usize
}

fn starts_a_statement(trimmed: &str) -> bool {
    !trimmed.is_empty() && !trimmed.starts_with(['}', ')', ']'])
}

fn ends_a_statement(line: &str) -> bool {
    line.trim_end().ends_with([';', '}'])
}

fn indentation(line: &str) -> usize {
    line.len() - line.trim_start().len()
}
