use crate::ConfigRequirement;
use crate::ConfigValues;
use crate::diagnostic::CompileFailure;
use fsb_assembler::Assembler;
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
    let checked = check_source(request.source)?;
    let bound = fsc_sema::bind_configs(checked, request.config_values)
        .map_err(|errors| CompileFailure::from_diagnostics(errors.into_diagnostics()))?;
    let hir = fsc_sema::lower(bound)
        .map_err(|error| CompileFailure::from_diagnostic(Diagnostic::from(error)))?;

    let mut assembler = Assembler::new();
    fsc_codegen::compile(&hir, &mut assembler).map_err(|error| {
        // TODO: attach source labels when HIR has spans
        CompileFailure::from_diagnostic(Diagnostic::error(Stage::Codegen, error.to_string()))
    })?;

    let binary = assembler
        .finalize(request.script_name.to_owned())
        .map_err(assembly_failure)?;
    let bytes = binary.serialize().map_err(assembly_failure)?;

    Ok(CompileArtifact {
        bytes,
        // TODO: collect successful-stage warnings
        diagnostics: Vec::new(),
    })
}

pub fn check(source: &str) -> Result<(), CompileFailure> {
    check_source(source)?;
    Ok(())
}

pub fn required_configs(source: &str) -> Result<Vec<ConfigRequirement>, CompileFailure> {
    Ok(check_source(source)?.config_requirements())
}

fn check_source(source: &str) -> Result<fsc_sema::CheckedScript, CompileFailure> {
    let script = fsc_parse::parse(source)
        .map_err(|error| CompileFailure::from_diagnostics(error.into_diagnostics()))?;

    fsc_sema::check(script)
        .map_err(|error| CompileFailure::from_diagnostic(Diagnostic::from(error)))
}

fn assembly_failure(error: impl std::fmt::Display) -> CompileFailure {
    // TODO: attach source labels on assembly error
    CompileFailure::from_diagnostic(Diagnostic::error(Stage::Assembly, error.to_string()))
}
