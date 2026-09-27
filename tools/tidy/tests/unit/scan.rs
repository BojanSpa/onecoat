use super::{Kind, scan};

#[test]
fn a_line_comment_is_found_at_its_position() {
    let found = scan("fn main() {\n    let x = 1; // trailing\n}\n").comments;
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (2, 16));
    assert_eq!(found[0].kind, Kind::Line);
}

#[test]
fn the_three_doc_forms_are_told_apart() {
    let found = scan("//! inner\n/// outer\n//// plain\n// plain\n").comments;
    let kinds = found.iter().map(|comment| comment.kind).collect::<Vec<_>>();

    assert_eq!(
        kinds,
        [Kind::InnerDoc, Kind::OuterDoc, Kind::Line, Kind::Line]
    );
}

#[test]
fn slashes_inside_literals_are_not_comments() {
    let source = "let a = \"// not a comment\";\nlet b = r\"// nor this\";\nlet c = r#\"// nor this\"#;\nlet d = '/';\nlet e: &'static str = \"x\";\nlet f = '\\\\';\n";
    assert!(scan(source).comments.is_empty());
}

#[test]
fn a_string_that_spans_lines_hides_the_comment_inside_it() {
    let source = "let a = \"one\\\ntwo\";\n// real\n";
    let found = scan(source).comments;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 3);
}

#[test]
fn nested_block_comments_end_where_they_close() {
    let found = scan("/* outer /* inner */ still outer */ let x = 1;\n").comments;
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (1, 1));
    assert_eq!(found[0].kind, Kind::Block);
}

#[test]
fn a_lifetime_is_not_a_char_literal() {
    let found = scan("fn f<'a>(s: &'a str) -> &'a str { s } // tail\n").comments;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].column, 39);
}

#[test]
fn hidden_lines_cover_a_multi_line_literal_but_not_its_first_line() {
    let source = "let a = 1;\nlet b = \"one\ntwo\nthree\";\nlet c = 2;\n";

    assert_eq!(
        scan(source).hidden,
        [false, false, true, true, false],
        "the line holding the opening quote stays visible"
    );
}

#[test]
fn hidden_lines_cover_raw_strings_and_block_comments() {
    let source = "let a = r#\"\n}\nfn x() {}\n\"#;\n/*\n}\nfn y() {}\n*/\n";

    assert_eq!(
        scan(source).hidden,
        [false, true, true, true, false, true, true, true]
    );
}
