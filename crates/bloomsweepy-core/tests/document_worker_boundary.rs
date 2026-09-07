use std::process::Command;

#[test]
fn isolated_helper_rejects_excess_heap_without_allocating_it() {
    let output = Command::new(env!("CARGO_BIN_EXE_bloomsweepy-document-worker"))
        .arg("--check-heap-budget")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "heap-budget-rejection-ok"
    );
}

#[test]
fn isolated_helper_rejects_invalid_arguments_without_reading_files() {
    let output = Command::new(env!("CARGO_BIN_EXE_bloomsweepy-document-worker"))
        .args(["--extract", "not-a-real-file", "9999999999", "100", "pdf"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
