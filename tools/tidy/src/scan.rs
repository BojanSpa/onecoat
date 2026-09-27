#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Line,
    Block,
    OuterDoc,
    InnerDoc,
}

pub struct Comment {
    pub line: usize,
    pub column: usize,
    pub kind: Kind,
}

pub struct Scan {
    pub comments: Vec<Comment>,
    pub hidden: Vec<bool>,
    pub code: String,
}

pub fn scan(text: &str) -> Scan {
    let mut scanner = Scanner::new(text);

    let mut found = Scan {
        comments: Vec::new(),
        hidden: vec![false; text.lines().count()],
        code: String::with_capacity(text.len()),
    };

    while let Some(ch) = scanner.peek(0) {
        let first = scanner.index;

        let hidden = match ch {
            '"' => {
                hide(&mut found.hidden, &mut scanner, Scanner::skip_string);
                true
            }
            'r' if scanner.raw_hashes().is_some() => {
                hide(&mut found.hidden, &mut scanner, Scanner::skip_raw_string);
                true
            }
            '\'' if scanner.char_literal() => {
                scanner.skip_char();
                true
            }
            '/' => match scanner.peek(1) {
                Some('/') => {
                    found.comments.push(scanner.skip_line_comment());
                    true
                }
                Some('*') => {
                    let comment =
                        hide(&mut found.hidden, &mut scanner, Scanner::skip_block_comment);
                    found.comments.push(comment);
                    true
                }
                _ => {
                    scanner.advance();
                    false
                }
            },
            _ => {
                scanner.advance();
                false
            }
        };

        let consumed = &scanner.chars[first..scanner.index];
        if hidden {
            found.code.extend(
                consumed
                    .iter()
                    .map(|ch| if *ch == '\n' { '\n' } else { ' ' }),
            );
        } else {
            found.code.extend(consumed);
        }
    }

    found
}

fn hide<T>(hidden: &mut [bool], scanner: &mut Scanner, skip: impl FnOnce(&mut Scanner) -> T) -> T {
    let first = scanner.line + 1;
    let value = skip(scanner);
    for line in first..=scanner.line {
        if let Some(flag) = hidden.get_mut(line - 1) {
            *flag = true;
        }
    }

    value
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
#[path = "../tests/unit/scan.rs"]
mod tests;
