use crate::scan::{Kind, scan};
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
            for comment in scan(&source.text).comments {
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
