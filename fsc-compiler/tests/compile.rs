#![allow(clippy::expect_used)]

use fsc_compiler::{CompileRequest, ConfigValues, check, compile};
use fsc_diagnostics::{LabelStyle, Severity, Span, Stage, render_diagnostics};

const VALID_SOURCE: &str = "void main() { return; }";

#[test]
fn checks_source_without_assembling_it() {
    let source = "extern void unavailable(); void main() { unavailable(); return; }";

    check(source).expect("frontend-valid source should pass checking");
    compile(CompileRequest::new(source, "main", &ConfigValues::new()))
        .expect_err("building should still report the unresolved external symbol");
}

#[test]
fn compiles_source_to_an_fsb_artifact() {
    let artifact = compile(CompileRequest::new(
        VALID_SOURCE,
        "main",
        &ConfigValues::new(),
    ))
    .expect("valid source should compile");

    assert!(artifact.bytes().len() > 32);
    assert_eq!(&artifact.bytes()[0..4], &32_u32.to_be_bytes());
    assert!(artifact.diagnostics().is_empty());
}

#[test]
fn parse_failure_retains_source_location() {
    let source = "void main() { return; }\n}\n";
    let failure = compile(CompileRequest::new(source, "broken", &ConfigValues::new()))
        .expect_err("an unmatched closing brace should fail");
    let diagnostic = &failure.diagnostics()[0];

    assert_eq!(diagnostic.stage(), Stage::Parse);
    assert_eq!(diagnostic.severity(), Severity::Error);
    assert_eq!(diagnostic.labels().len(), 1);

    let rendered = render_diagnostics(failure.diagnostics(), "broken.fsc", source);
    assert!(rendered.contains("broken.fsc:2:1"));
    assert!(rendered.contains('^'));
}

#[test]
fn semantic_failure_is_normalized() {
    let source = "void main() { missing(); return; }";
    let failure = compile(CompileRequest::new(source, "main", &ConfigValues::new()))
        .expect_err("undeclared call should fail");
    let diagnostic = &failure.diagnostics()[0];

    assert_eq!(diagnostic.stage(), Stage::Semantic);
    assert!(diagnostic.message().contains("undeclared name `missing`"));
    assert_eq!(diagnostic.labels().len(), 1);
}

#[test]
fn invalid_script_name_is_an_assembly_failure() {
    let failure = compile(CompileRequest::new(
        VALID_SOURCE,
        "main!",
        &ConfigValues::new(),
    ))
    .expect_err("invalid B40 name should fail during serialization");
    let diagnostic = &failure.diagnostics()[0];

    assert_eq!(diagnostic.stage(), Stage::Assembly);
    assert!(diagnostic.message().contains("invalid character"));
}

#[test]
fn all_lexer_failures_survive_in_source_order() {
    let source = "void main() { @ # return; }";
    let failure = compile(CompileRequest::new(source, "broken", &ConfigValues::new()))
        .expect_err("unknown characters should fail lexing");
    let diagnostics = failure.diagnostics();

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].message().contains("`@`"));
    assert!(diagnostics[1].message().contains("`#`"));
    assert!(diagnostics.iter().all(|item| item.stage() == Stage::Parse));
}

#[test]
fn duplicate_declaration_labels_duplicate_before_original() {
    let source = "void main(int value, int value) { return; }";
    let failure = compile(CompileRequest::new(source, "main", &ConfigValues::new()))
        .expect_err("duplicate parameters should fail");
    let labels = failure.diagnostics()[0].labels();

    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].style(), LabelStyle::Primary);
    assert_eq!(labels[0].span(), Span::new(25, 30));
    assert_eq!(labels[1].style(), LabelStyle::Secondary);
    assert_eq!(labels[1].span(), Span::new(14, 19));
}

#[test]
fn type_mismatch_points_to_expression_and_declared_type() {
    let source = "void main() { int value = true; return; }";
    let failure = compile(CompileRequest::new(source, "main", &ConfigValues::new()))
        .expect_err("invalid initializer should fail checking");
    let labels = failure.diagnostics()[0].labels();

    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].style(), LabelStyle::Primary);
    assert_eq!(&source[labels[0].span().range()], "true");
    assert_eq!(labels[1].style(), LabelStyle::Secondary);
    assert_eq!(&source[labels[1].span().range()], "int");
}

#[test]
fn unexpected_eof_uses_an_empty_span_at_source_end() {
    let source = "void main(";
    let failure = compile(CompileRequest::new(source, "broken", &ConfigValues::new()))
        .expect_err("unfinished parameter list should fail parsing");
    let label = &failure.diagnostics()[0].labels()[0];

    assert_eq!(label.span(), Span::new(source.len(), source.len()));
    let rendered = render_diagnostics(failure.diagnostics(), "broken.fsc", source);
    assert!(rendered.contains("broken.fsc"));
}
