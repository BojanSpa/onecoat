use super::{Kind, comments};

#[test]
fn a_line_comment_is_found_at_its_position() {
    let found = comments("fn main() {\n    let x = 1; // trailing\n}\n");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (2, 16));
    assert_eq!(found[0].kind, Kind::Line);
}

#[test]
fn the_three_doc_forms_are_told_apart() {
    let found = comments("//! inner\n/// outer\n//// plain\n// plain\n");
    let kinds = found.iter().map(|comment| comment.kind).collect::<Vec<_>>();
    assert_eq!(
        kinds,
        [Kind::InnerDoc, Kind::OuterDoc, Kind::Line, Kind::Line]
    );
}

#[test]
fn slashes_inside_literals_are_not_comments() {
    let source = "let a = \"// not a comment\";\nlet b = r\"// nor this\";\nlet c = r#\"// nor this\"#;\nlet d = '/';\nlet e: &'static str = \"x\";\nlet f = '\\\\';\n";
    assert!(comments(source).is_empty());
}

#[test]
fn a_string_that_spans_lines_hides_the_comment_inside_it() {
    let source = "let a = \"one\\\ntwo\";\n// real\n";
    let found = comments(source);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].line, 3);
}

#[test]
fn nested_block_comments_end_where_they_close() {
    let found = comments("/* outer /* inner */ still outer */ let x = 1;\n");
    assert_eq!(found.len(), 1);
    assert_eq!((found[0].line, found[0].column), (1, 1));
    assert_eq!(found[0].kind, Kind::Block);
}

#[test]
fn a_lifetime_is_not_a_char_literal() {
    let found = comments("fn f<'a>(s: &'a str) -> &'a str { s } // tail\n");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].column, 39);
}
