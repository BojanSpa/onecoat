use std::path::PathBuf;

use tidy::{Check, Diagnostic, Error, Repo, checks};

fn repo(case: &str) -> Repo {
    Repo::new(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(case),
    )
}

fn check(id: &str) -> &'static dyn Check {
    *checks::CHECKS
        .iter()
        .find(|check| check.id() == id)
        .expect("a check with that id")
}

fn findings(id: &str, case: &str) -> Vec<Diagnostic> {
    check(id).run(&repo(case)).expect("the fixture runs")
}

fn positions(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}:{}", diagnostic.path, diagnostic.line))
        .collect()
}

#[test]
fn the_purity_check_flags_every_way_a_pure_module_can_reach_out() {
    let diagnostics = findings("purity", "purity/fails");

    assert_eq!(
        positions(&diagnostics),
        ["src/jsonc.rs:2", "src/plan.rs:2", "src/render/wt.rs:1"]
    );

    assert!(
        diagnostics[2].message.contains("std::env"),
        "a use tree is still a reach out: {}",
        diagnostics[2].message
    );
}

#[test]
fn the_purity_check_passes_a_pure_module() {
    assert!(findings("purity", "purity/passes").is_empty());
}

#[test]
fn the_purity_check_stops_when_an_expected_file_is_gone() {
    let error = check("purity")
        .run(&repo("purity/missing"))
        .expect_err("an edge module the check expects is missing");

    assert!(matches!(error, Error::CheckTargetMissing { .. }), "{error}");
}

#[test]
fn the_comment_check_flags_line_doc_and_block_comments() {
    assert_eq!(
        positions(&findings("no-comments", "no-comments/fails")),
        [
            "src/main.rs:1",
            "src/render/wt.rs:2",
            "src/render/wt.rs:5",
            "src/render/wt.rs:8",
            "tools/tidy/src/main.rs:2",
        ]
    );
}

#[test]
fn the_comment_check_lets_the_crate_doc_through() {
    assert!(findings("no-comments", "no-comments/passes").is_empty());
}

#[test]
fn the_spacing_check_flags_items_that_start_right_after_a_closing_brace() {
    assert_eq!(
        positions(&findings("item-spacing", "item-spacing/fails")),
        [
            "src/lib.rs:5",
            "src/lib.rs:12",
            "src/lib.rs:17",
            "src/lib.rs:20"
        ]
    );
}

#[test]
fn the_spacing_check_leaves_bodies_uses_and_literals_alone() {
    assert!(findings("item-spacing", "item-spacing/passes").is_empty());
}

#[test]
fn the_statement_check_flags_bodies_without_a_blank_line() {
    let diagnostics = findings("statement-spacing", "statement-spacing/fails");

    assert_eq!(
        positions(&diagnostics),
        [
            "src/lib.rs:5",
            "src/lib.rs:13",
            "src/lib.rs:24",
            "src/lib.rs:32",
            "src/lib.rs:34"
        ]
    );

    assert!(
        diagnostics[0].message.contains("tail expression"),
        "the tail case reads differently: {}",
        diagnostics[0].message
    );
}

#[test]
fn the_statement_check_flags_a_blank_line_that_unglues_a_single_line_let() {
    assert_eq!(
        positions(&findings("statement-spacing", "statement-spacing/glued")),
        ["src/lib.rs:3"]
    );
}

#[test]
fn the_statement_check_leaves_compliant_bodies_alone() {
    assert!(findings("statement-spacing", "statement-spacing/passes").is_empty());
}

#[test]
fn the_plan_check_flags_a_long_sentence_and_a_missing_break() {
    assert_eq!(
        positions(&findings("plan-style", "plan-style/fails")),
        ["work/plans/vs4-thing.md:5", "work/plans/vs4-thing.md:6"]
    );
}

#[test]
fn the_plan_check_ignores_comments_and_code_blocks() {
    assert!(findings("plan-style", "plan-style/passes").is_empty());
}

#[test]
fn the_plan_check_leaves_a_table_row_alone() {
    let table = repo("plan-style/passes")
        .read("work/plans/vs3-thing.md")
        .expect("the table plan is there");

    assert!(table.contains("| A table row without a break"));
    assert!(findings("plan-style", "plan-style/passes").is_empty());
}

#[test]
fn the_plan_check_skips_a_merged_plan() {
    let exempt = repo("plan-style/passes")
        .read("work/plans/vs1-wt-scheme-fragment.md")
        .expect("the merged plan is there");

    assert!(exempt.contains("A line with no br at the end."));
    assert!(findings("plan-style", "plan-style/passes").is_empty());
}
