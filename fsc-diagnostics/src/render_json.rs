use crate::{Diagnostic, Label, LabelStyle, Severity, Stage};
use serde::Serialize;

#[derive(Serialize)]
struct JsonDiagnostic<'a> {
    file: &'a str,
    stage: &'static str,
    severity: &'static str,
    message: &'a str,
    labels: Vec<JsonLabel<'a>>,
}

#[derive(Serialize)]
struct JsonLabel<'a> {
    style: &'static str,
    message: &'a str,
    start: usize,
    end: usize,
}

pub fn render_diagnostics_json(
    diagnostics: &[Diagnostic],
    source_name: &str,
) -> Result<String, serde_json::Error> {
    let diagnostics = diagnostics
        .iter()
        .map(|diagnostic| JsonDiagnostic {
            file: source_name,
            stage: stage_name(diagnostic.stage()),
            severity: severity_name(diagnostic.severity()),
            message: diagnostic.message(),
            labels: diagnostic.labels().iter().map(json_label).collect(),
        })
        .collect::<Vec<_>>();

    let mut output = serde_json::to_string(&diagnostics)?;
    output.push('\n');
    Ok(output)
}

fn json_label(label: &Label) -> JsonLabel<'_> {
    JsonLabel {
        style: match label.style() {
            LabelStyle::Primary => "primary",
            LabelStyle::Secondary => "secondary",
        },
        message: label.message(),
        start: label.span().start(),
        end: label.span().end(),
    }
}

const fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Parse => "parse",
        Stage::Semantic => "semantic",
        Stage::Codegen => "codegen",
        Stage::Assembly => "assembly",
    }
}

const fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    }
}
