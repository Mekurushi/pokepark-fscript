use fsc_diagnostics::Diagnostic;

#[derive(Debug)]
pub struct CompileFailure {
    diagnostics: Vec<Diagnostic>,
}

impl CompileFailure {
    #[must_use]
    pub fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    #[must_use]
    pub fn from_diagnostics(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
