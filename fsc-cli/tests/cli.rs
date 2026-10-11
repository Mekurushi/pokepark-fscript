#![allow(clippy::expect_used)]

use fsc_compiler::{CompileRequest, ConfigValues, compile};
use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn build_writes_the_compiler_output() {
    let temp = tempfile::tempdir().expect("temporary directory should be creatable");
    let input = temp.path().join("sample.fsc");
    let output = input.with_extension("fsb");
    let source = "int answer() { return 42; }";
    fs::write(&input, source).expect("test input should be writable");

    let result = Command::new(env!("CARGO_BIN_EXE_fsc"))
        .args(["build", path_str(&input)])
        .output()
        .expect("CLI should run");

    assert!(result.status.success(), "{}", stderr(&result));
    let expected = compile(CompileRequest::new(source, "sample", &ConfigValues::new()))
        .expect("facade compilation should succeed")
        .into_bytes();
    assert_eq!(fs::read(output).expect("output should exist"), expected);
}

#[test]
fn build_renders_compiler_failures() {
    let temp = tempfile::tempdir().expect("temporary directory should be creatable");
    let invalid_input = temp.path().join("invalid.fsc");
    fs::write(&invalid_input, "}").expect("test input should be writable");
    let failure = Command::new(env!("CARGO_BIN_EXE_fsc"))
        .args(["build", path_str(&invalid_input)])
        .output()
        .expect("CLI should run");
    let error = stderr(&failure);

    assert!(!failure.status.success());
    assert!(error.contains("invalid.fsc"), "{error}");
    assert!(!invalid_input.with_extension("fsb").exists());
}

#[test]
fn check_validates_without_writing_an_artifact() {
    let temp = tempfile::tempdir().expect("temporary directory should be creatable");
    let input = temp.path().join("checked.fsc");
    fs::write(&input, "void main() { return; }").expect("test input should be writable");

    let result = Command::new(env!("CARGO_BIN_EXE_fsc"))
        .args(["check", path_str(&input)])
        .output()
        .expect("CLI should run");

    assert!(result.status.success(), "{}", stderr(&result));
    assert!(!input.with_extension("fsb").exists());
}

#[test]
fn check_renders_frontend_failures() {
    let temp = tempfile::tempdir().expect("temporary directory should be creatable");
    let input = temp.path().join("invalid.fsc");
    fs::write(&input, "void main() { return true; }").expect("test input should be writable");

    let result = Command::new(env!("CARGO_BIN_EXE_fsc"))
        .args(["check", path_str(&input)])
        .output()
        .expect("CLI should run");
    let error = stderr(&result);

    assert!(!result.status.success());
    assert!(error.contains("invalid.fsc"), "{error}");
    assert!(error.contains("return type mismatch"), "{error}");
    assert!(!input.with_extension("fsb").exists());
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path should be UTF-8")
}

fn stderr(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
