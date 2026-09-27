use std::path::PathBuf;

#[test]
fn the_repository_is_tidy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");

    let diagnostics = tidy::run(&root).expect("the checks run");

    let reported = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect::<Vec<_>>();

    assert!(reported.is_empty(), "{reported:#?}");
}

#[test]
fn a_root_that_is_not_a_directory_is_an_error() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("no-such-root");
    let error = tidy::run(&root).expect_err("a missing root cannot be checked");
    assert!(matches!(error, tidy::Error::RootMissing { .. }), "{error}");
}
