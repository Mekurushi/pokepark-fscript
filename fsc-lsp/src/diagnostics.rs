use crate::documents::Document;
use fsc_diagnostics::{Diagnostic as CompilerDiagnostic, Label, LabelStyle, Severity, Span};
use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location,
    PublishDiagnosticsParams, Range, Uri,
};

pub(crate) fn check_document(
    uri: Uri,
    document: &Document,
    related_information: bool,
) -> PublishDiagnosticsParams {
    let converter = DiagnosticConverter::new(&uri, document, related_information);
    let diagnostics = match fsc_compiler::check(document.text()) {
        Ok(()) => Vec::new(),
        Err(failure) => converter.convert_all(failure.diagnostics()),
    };

    PublishDiagnosticsParams::new(uri, diagnostics, Some(document.version()))
}

struct DiagnosticConverter<'a> {
    uri: &'a Uri,
    document: &'a Document,
    include_related_information: bool,
}

impl<'a> DiagnosticConverter<'a> {
    const fn new(uri: &'a Uri, document: &'a Document, include_related_information: bool) -> Self {
        Self {
            uri,
            document,
            include_related_information,
        }
    }

    fn convert_all(&self, diagnostics: &[CompilerDiagnostic]) -> Vec<Diagnostic> {
        let mut converted = Vec::with_capacity(diagnostics.len());
        for diagnostic in diagnostics {
            if let Some(diagnostic) = self.convert(diagnostic) {
                converted.push(diagnostic);
            }
        }
        converted
    }

    fn convert(&self, diagnostic: &CompilerDiagnostic) -> Option<Diagnostic> {
        let primary = Self::primary_label(diagnostic);
        let range = self.primary_range(primary)?;
        let message = Self::message(diagnostic, primary);
        let severity = Self::severity(diagnostic.severity());
        let related_information = self.related_information(diagnostic);

        Some(Diagnostic::new(
            range,
            Some(severity),
            None,
            Some("fsc".to_owned()),
            message,
            related_information,
            None,
        ))
    }

    fn primary_label(diagnostic: &CompilerDiagnostic) -> Option<&Label> {
        diagnostic
            .labels()
            .iter()
            .find(|label| label.style() == LabelStyle::Primary)
    }

    fn primary_range(&self, primary: Option<&Label>) -> Option<Range> {
        let Some(primary) = primary else {
            return Some(Range::default());
        };

        let span = primary.span();
        let range = self.span_range(span);
        if range.is_none() {
            log::warn!(
                "omitting compiler diagnostic with invalid primary span {}..{}",
                span.start(),
                span.end(),
            );
        }
        range
    }

    fn message(diagnostic: &CompilerDiagnostic, primary: Option<&Label>) -> String {
        match primary {
            Some(label) if !label.message().is_empty() => {
                format!("{}: {}", diagnostic.message(), label.message())
            }
            _ => diagnostic.message().to_owned(),
        }
    }

    const fn severity(severity: Severity) -> DiagnosticSeverity {
        match severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
        }
    }

    fn related_information(
        &self,
        diagnostic: &CompilerDiagnostic,
    ) -> Option<Vec<DiagnosticRelatedInformation>> {
        if !self.include_related_information {
            return None;
        }

        let mut related_information = Vec::new();
        for label in diagnostic.labels() {
            if label.style() != LabelStyle::Secondary {
                continue;
            }

            let span = label.span();
            let Some(range) = self.span_range(span) else {
                log::warn!(
                    "omitting related diagnostic information with invalid span {}..{}",
                    span.start(),
                    span.end(),
                );
                continue;
            };

            related_information.push(DiagnosticRelatedInformation {
                location: Location::new(self.uri.clone(), range),
                message: label.message().to_owned(),
            });
        }

        Some(related_information)
    }

    fn span_range(&self, span: Span) -> Option<Range> {
        Some(Range::new(
            self.document.position(span.start())?,
            self.document.position(span.end())?,
        ))
    }
}
