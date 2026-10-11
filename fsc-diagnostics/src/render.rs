use crate::{Diagnostic, LabelStyle, Severity, Span};
use annotate_snippets::{AnnotationKind, Group, Level, Renderer, Snippet};

#[must_use]
pub fn render_diagnostics(diagnostics: &[Diagnostic], source_name: &str, source: &str) -> String {
    let renderer = Renderer::plain();
    let mut output = diagnostics
        .iter()
        .map(|diagnostic| render_diagnostic(&renderer, diagnostic, source_name, source))
        .collect::<Vec<_>>()
        .join("\n");

    if !output.is_empty() {
        output.push('\n');
    }

    output
}

fn render_diagnostic(
    renderer: &Renderer,
    diagnostic: &Diagnostic,
    source_name: &str,
    source: &str,
) -> String {
    let level = match diagnostic.severity() {
        Severity::Error => Level::ERROR,
        Severity::Warning => Level::WARNING,
    };
    let mut group = Group::with_title(level.primary_title(diagnostic.message()));

    if !diagnostic.labels().is_empty() {
        let mut snippet = Snippet::source(source).path(source_name);
        for label in diagnostic.labels() {
            let span = sanitize_span(label.span(), source);
            let annotation = match label.style() {
                LabelStyle::Primary => AnnotationKind::Primary,
                LabelStyle::Secondary => AnnotationKind::Context,
            }
            .span(span.range())
            .label(label.message());
            snippet = snippet.annotation(annotation);
        }
        group = group.element(snippet);
    }

    renderer.render(&[group])
}

fn sanitize_span(span: Span, source: &str) -> Span {
    let mut start = span.start().min(source.len());
    while start > 0 && !source.is_char_boundary(start) {
        start -= 1;
    }

    let mut end = span.end().max(start).min(source.len());
    while end > start && !source.is_char_boundary(end) {
        end -= 1;
    }
    Span::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Diagnostic, Label, Stage};

    #[test]
    fn renders_primary_and_secondary_labels_in_one_snippet() {
        let source = "int value = true;";
        let diagnostics = [Diagnostic::error(Stage::Semantic, "type mismatch")
            .with_label(Label::primary(Span::new(12, 16), "has type `Bool`"))
            .with_label(Label::secondary(Span::new(0, 3), "expected `Int`"))];

        let rendered = render_diagnostics(&diagnostics, "test.fsc", source);

        assert_eq!(
            rendered,
            "error: type mismatch\n \
             --> test.fsc:1:13\n  \
             |\n1 \
             | int value = true;\n  \
             | ---         ^^^^ has type `Bool`\n  \
             | |\n  \
             | expected `Int`\n"
        );
    }

    #[test]
    fn handles_unicode_tabs_multiline_and_control_characters() {
        let source = "\té\u{1b}[31m\nnext";
        let diagnostics = [Diagnostic::error(Stage::Parse, "example")
            .with_label(Label::primary(Span::new(1, source.len()), "location"))];

        let rendered = render_diagnostics(&diagnostics, "test.fsc", source);

        assert!(rendered.contains("location"));
        assert!(rendered.contains("next"));
        assert!(!rendered.contains('\u{1b}'));
    }

    #[test]
    fn malformed_and_empty_spans_do_not_panic() {
        let source = "é\r\nnext";
        let diagnostics = [
            Diagnostic::error(Stage::Parse, "malformed")
                .with_label(Label::primary(Span::new(1, usize::MAX), "location")),
            Diagnostic::error(Stage::Parse, "eof")
                .with_label(Label::primary(Span::new(source.len(), source.len()), "end")),
        ];

        let rendered = render_diagnostics(&diagnostics, "test.fsc", source);
        assert!(rendered.contains("location"));
        assert!(rendered.contains("end"));
    }

    #[test]
    fn renders_unlabelled_diagnostic() {
        let diagnostics = [Diagnostic::error(Stage::Assembly, "binary input failed")];
        let rendered = render_diagnostics(&diagnostics, "test.fsc", "");
        assert!(rendered.contains("error: binary input failed"));
    }
}
