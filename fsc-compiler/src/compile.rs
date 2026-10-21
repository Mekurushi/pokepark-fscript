use crate::CheckMode;
use crate::ConfigRequirement;
use crate::ConfigValues;
use crate::diagnostic::CompileFailure;
use fsc_diagnostics::{Diagnostic, Stage};

#[derive(Debug, Clone, Copy)]
pub struct CompileRequest<'src> {
    pub source: &'src str,
    pub script_name: &'src str,
    pub config_values: &'src ConfigValues,
}

impl<'src> CompileRequest<'src> {
    #[must_use]
    pub const fn new(
        source: &'src str,
        script_name: &'src str,
        config_values: &'src ConfigValues,
    ) -> Self {
        Self {
            source,
            script_name,
            config_values,
        }
    }
}

#[derive(Debug)]
pub struct CompileArtifact {
    bytes: Vec<u8>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub struct CheckOutput {
    diagnostics: Vec<Diagnostic>,
}

impl CheckOutput {
    fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self { diagnostics }
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity() == fsc_diagnostics::Severity::Error)
    }

    #[must_use]
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }
}

impl CompileArtifact {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

pub fn compile(request: CompileRequest<'_>) -> Result<CompileArtifact, CompileFailure> {
    let script = parse(request.source)?;
    let analysis = fsc_sema::analyze(&script, request.config_values)
        .map_err(|diagnostics| CompileFailure::from_diagnostics(diagnostics.entries().to_vec()))?;
    let mir = fsc_mir::lower(&analysis.thir);
    let program = fsc_codegen::lower(&mir).map_err(codegen_failure)?;
    let assembly = fsb_assembler::assemble_program(&program)
        .map_err(assembly_failure)?
        .into_unit();
    let binary = assembly
        .into_binary(request.script_name.to_owned())
        .map_err(assembly_failure)?;
    let bytes = binary.serialize().map_err(assembly_failure)?;

    Ok(CompileArtifact {
        bytes,
        diagnostics: analysis.diagnostics.entries().to_vec(),
    })
}

pub fn check(source: &str, mode: CheckMode<'_>) -> CheckOutput {
    match fsc_parse::parse(source) {
        Ok(script) => CheckOutput::new(fsc_sema::check(&script, mode).entries().to_vec()),
        Err(error) => CheckOutput::new(error.into_diagnostics()),
    }
}

pub fn required_configs(source: &str) -> Result<Vec<ConfigRequirement>, CompileFailure> {
    let script = parse(source)?;
    fsc_sema::required_configs(&script)
        .map_err(|diagnostics| CompileFailure::from_diagnostics(diagnostics.entries().to_vec()))
}

fn parse(source: &str) -> Result<fsc_parse::ast::Script, CompileFailure> {
    fsc_parse::parse(source)
        .map_err(|error| CompileFailure::from_diagnostics(error.into_diagnostics()))
}

fn codegen_failure(error: impl std::fmt::Display) -> CompileFailure {
    CompileFailure::from_diagnostic(Diagnostic::error(Stage::Codegen, error.to_string()))
}

fn assembly_failure(error: impl std::fmt::Display) -> CompileFailure {
    CompileFailure::from_diagnostic(Diagnostic::error(Stage::Assembly, error.to_string()))
}
