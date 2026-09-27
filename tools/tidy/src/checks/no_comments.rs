use crate::{Check, Diagnostic, Error, Repo, Source};

const SOURCES: &[&str] = &["src", "tests", "tools/tidy/src", "tools/tidy/tests"];
const CRATE_DOC: &str = "src/lib.rs";

pub struct NoComments;

impl Check for NoComments {
    fn id(&self) -> &'static str {
        "no-comments"
    }

    fn run(&self, repo: &Repo) -> Result<Vec<Diagnostic>, Error> {
        let mut found = Vec::new();
        for source in repo.sources(SOURCES, "rs")? {
            let crate_doc = crate_doc_lines(&source);
            for comment in comments(&source.text) {
                if source.path == CRATE_DOC
                    && comment.kind == Kind::InnerDoc
                    && comment.line <= crate_doc
                {
                    continue;
                }
                found.push(Diagnostic {
                    path: source.path.clone(),
                    line: comment.line,
                    column: comment.column,
                    check: self.id(),
                    message: "the source carries no comments; write self-evident code, or move the contract to docs/architecture.md".to_owned(),
                });
            }
        }
        Ok(found)
    }
}

fn crate_doc_lines(source: &Source) -> usize {
    if source.path != CRATE_DOC {
        return 0;
    }
    source
        .text
        .lines()
        .take_while(|line| line.trim_start().starts_with("//!"))
        .count()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Line,
    Block,
    OuterDoc,
    InnerDoc,
}

struct Comment {
    line: usize,
    column: usize,
    kind: Kind,
}

fn comments(text: &str) -> Vec<Comment> {
    let mut scanner = Scanner::new(text);
    let mut found = Vec::new();
    while let Some(ch) = scanner.peek(0) {
        match ch {
            '"' => scanner.skip_string(),
            'r' if scanner.raw_hashes().is_some() => scanner.skip_raw_string(),
            '\'' if scanner.char_literal() => scanner.skip_char(),
            '/' => match scanner.peek(1) {
                Some('/') => found.push(scanner.skip_line_comment()),
                Some('*') => found.push(scanner.skip_block_comment()),
                _ => scanner.advance(),
            },
            _ => scanner.advance(),
        }
    }
    found
}

struct Scanner {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
}

impl Scanner {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn advance(&mut self) {
        match self.chars.get(self.index) {
            Some('\n') => {
                self.line += 1;
                self.column = 1;
            }
            Some(_) => self.column += 1,
            None => {}
        }
        self.index += 1;
    }

    fn push_past(&mut self, offset: usize) {
        for _ in 0..offset {
            self.advance();
        }
    }

    fn skip_string(&mut self) {
        self.advance();
        while let Some(ch) = self.peek(0) {
            match ch {
                '\\' => self.push_past(2),
                '"' => {
                    self.advance();
                    break;
                }
                _ => self.advance(),
            }
        }
    }

    fn raw_hashes(&self) -> Option<usize> {
        let mut offset = 1;
        while self.peek(offset) == Some('#') {
            offset += 1;
        }
        (self.peek(offset) == Some('"')).then_some(offset - 1)
    }

    fn skip_raw_string(&mut self) {
        let hashes = self.raw_hashes().unwrap_or(0);
        self.push_past(hashes + 2);
        while self.peek(0).is_some() {
            if self.peek(0) == Some('"')
                && (1..=hashes).all(|offset| self.peek(offset) == Some('#'))
            {
                self.push_past(hashes + 1);
                break;
            }
            self.advance();
        }
    }

    fn char_literal(&self) -> bool {
        match self.peek(1) {
            Some('\\') => true,
            Some(_) => self.peek(2) == Some('\''),
            None => false,
        }
    }

    fn skip_char(&mut self) {
        self.advance();
        if self.peek(0) == Some('\\') {
            self.advance();
        }
        while let Some(ch) = self.peek(0) {
            self.advance();
            if ch == '\'' {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) -> Comment {
        let kind = match self.peek(2) {
            Some('!') => Kind::InnerDoc,
            Some('/') if self.peek(3) != Some('/') => Kind::OuterDoc,
            _ => Kind::Line,
        };
        let comment = Comment {
            line: self.line,
            column: self.column,
            kind,
        };
        while let Some(ch) = self.peek(0) {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
        comment
    }

    fn skip_block_comment(&mut self) -> Comment {
        let comment = Comment {
            line: self.line,
            column: self.column,
            kind: Kind::Block,
        };
        let mut depth = 0usize;
        while let Some(ch) = self.peek(0) {
            if ch == '/' && self.peek(1) == Some('*') {
                depth += 1;
                self.push_past(2);
                continue;
            }
            if ch == '*' && self.peek(1) == Some('/') {
                depth = depth.saturating_sub(1);
                self.push_past(2);
                if depth == 0 {
                    break;
                }
                continue;
            }
            self.advance();
        }
        comment
    }
}

#[cfg(test)]
#[path = "../../tests/unit/checks/no_comments.rs"]
mod tests;
