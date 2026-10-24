use crate::{ConfigValues, ExternalSymbolTable, PatchFailure};
use fsb_assembler::binary::{FscriptBinary, CODE_SECTION_FILE_OFFSET};
use fsb_assembler::{
    assemble_program, AssembledProgram, AssemblyUnit, ExternalDataOffsets, ExternalFunctionOffsets,
};
use fsb_ir::{FunctionId, Program};
use fsc_diagnostics::{Diagnostic, Stage};
use fsc_parse::ast::{Item, Script};

struct OriginalProgram {
    script_name: String,
    unit: AssemblyUnit,
    code_len: usize,
}

struct CompiledPatch {
    syntax: Script,
    program: Program,
}

struct PatchPlan {
    replacements: Vec<PlannedReplacement>,
    exports: Vec<PlannedExport>,
    external_functions: ExternalFunctionOffsets,
    external_data: ExternalDataOffsets,
    output_symbols: ExternalSymbolTable,
}

struct PlannedReplacement {
    function: FunctionId,
    entry_offset: u32,
}

struct PlannedExport {
    function: FunctionId,
    name: String,
}

struct AssembledPatch {
    program: AssembledProgram,
    replacements: Vec<AssembledReplacement>,
    exports: Vec<AssembledExport>,
    external_functions: ExternalFunctionOffsets,
    external_data: ExternalDataOffsets,
    output_symbols: ExternalSymbolTable,
}

struct AssembledReplacement {
    entry_offset: u32,
    replacement_offset: u32,
}

struct AssembledExport {
    name: String,
    offset: u32,
}

struct LinkedProgram {
    script_name: String,
    unit: AssemblyUnit,
    external_functions: ExternalFunctionOffsets,
    external_data: ExternalDataOffsets,
    output_symbols: ExternalSymbolTable,
}

fn function_entry_offset(
    function_name: &str,
    address: u32,
    base_address: u32,
    code_len: usize,
) -> Result<u32, PatchFailure> {
    let invalid_address = || PatchFailure::InvalidFunctionAddress {
        function_name: function_name.to_owned(),
        address,
    };
    // it is absolute addresses like in the ghidra addon; TODO: define as explicit behavior
    let file_offset = address
        .checked_sub(base_address)
        .ok_or_else(invalid_address)?;
    let entry_offset = file_offset
        .checked_sub(CODE_SECTION_FILE_OFFSET)
        .ok_or_else(invalid_address)?;
    let entry_index = entry_offset as usize;

    if !entry_offset.is_multiple_of(4)
        || entry_index.checked_add(4).is_none_or(|end| end > code_len)
    {
        return Err(invalid_address());
    }

    Ok(entry_offset)
}

fn data_offset(
    data_name: &str,
    address: u32,
    base_address: u32,
    code_len: usize,
) -> Result<u32, PatchFailure> {
    let invalid_address = || PatchFailure::InvalidDataAddress {
        data_name: data_name.to_owned(),
        address,
    };
    let file_offset = address
        .checked_sub(base_address)
        .ok_or_else(invalid_address)?;
    let offset = file_offset
        .checked_sub(CODE_SECTION_FILE_OFFSET)
        .ok_or_else(invalid_address)?;
    let index = offset as usize;

    if !offset.is_multiple_of(4) || index.checked_add(4).is_none_or(|end| end > code_len) {
        return Err(invalid_address());
    }

    Ok(offset)
}

fn validate_external_declarations(
    script: &Script,
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<(), PatchFailure> {
    for declaration in script.items.iter().filter_map(|item| match item {
        Item::FuncDef(_) => None,
        Item::ExternFuncDecl(declaration) => Some(declaration),
        Item::StaticDecl(_) | Item::ConstDecl(_) | Item::ConfigDecl(_) => None,
    }) {
        let address = symbols
            .get_function(&declaration.header.name)
            .ok_or_else(|| PatchFailure::MissingExternalSymbol {
                function_name: declaration.header.name.clone(),
            })?;
        function_entry_offset(
            &declaration.header.name,
            address,
            symbols.base_address(),
            code_len,
        )?;
    }

    for declaration in script.items.iter().filter_map(|item| match item {
        Item::StaticDecl(declaration)
            if matches!(declaration.kind, fsc_parse::ast::StaticKind::External) =>
        {
            Some(declaration)
        }
        Item::FuncDef(_)
        | Item::ExternFuncDecl(_)
        | Item::StaticDecl(_)
        | Item::ConstDecl(_)
        | Item::ConfigDecl(_) => None,
    }) {
        let address = symbols.get_data(&declaration.name).ok_or_else(|| {
            PatchFailure::MissingExternalDataSymbol {
                data_name: declaration.name.clone(),
            }
        })?;
        data_offset(&declaration.name, address, symbols.base_address(), code_len)?;
    }

    Ok(())
}

pub struct PatchRequest<'a> {
    pub patch_source: &'a str,
    pub original_binary: &'a [u8],
    // keeping symbols as typed input so it's independent of the used file format, so we're not
    // too strongly bound to one specific format
    pub symbols: &'a ExternalSymbolTable,
    pub config_values: &'a ConfigValues,
}

impl<'a> PatchRequest<'a> {
    pub const fn new(
        patch_source: &'a str,
        original_binary: &'a [u8],
        symbols: &'a ExternalSymbolTable,
        config_values: &'a ConfigValues,
    ) -> Self {
        Self {
            patch_source,
            original_binary,
            symbols,
            config_values,
        }
    }
}

pub struct PatchArtifact {
    binary: Vec<u8>,
    symbols: ExternalSymbolTable,
}

impl PatchArtifact {
    pub fn binary(&self) -> &[u8] {
        &self.binary
    }

    pub const fn symbols(&self) -> &ExternalSymbolTable {
        &self.symbols
    }

    pub fn into_parts(self) -> (Vec<u8>, ExternalSymbolTable) {
        (self.binary, self.symbols)
    }
}

pub fn patch(request: PatchRequest<'_>) -> Result<PatchArtifact, PatchFailure> {
    let original = load_original(request.original_binary)?;
    let compiled = compile_patch(request.patch_source, request.config_values)?;
    let plan = plan_patch(&original, &compiled, request.symbols)?;
    let assembled = assemble_patch(&compiled.program, plan)?;
    let linked = link_patch(original, assembled)?;
    finalize_patch(linked)
}

fn load_original(binary: &[u8]) -> Result<OriginalProgram, PatchFailure> {
    let binary = FscriptBinary::deserialize(binary)?;
    let code_len = binary.code_len();
    let (script_name, unit) = AssemblyUnit::from_binary(binary)?;

    Ok(OriginalProgram {
        script_name,
        unit,
        code_len,
    })
}

fn compile_patch(
    source: &str,
    config_values: &ConfigValues,
) -> Result<CompiledPatch, PatchFailure> {
    let syntax = fsc_parse::parse(source)
        .map_err(|error| PatchFailure::InvalidPatchSource(error.into_diagnostics()))?;
    let analysis = fsc_sema::analyze(&syntax, config_values)
        .map_err(|diagnostics| PatchFailure::InvalidPatchSource(diagnostics.entries().to_vec()))?;
    let mir = fsc_mir::lower(&analysis.thir);
    let program = fsc_codegen::lower(&mir).map_err(|error| {
        PatchFailure::InvalidPatchSource(vec![Diagnostic::error(Stage::Codegen, error.to_string())])
    })?;

    Ok(CompiledPatch { syntax, program })
}

fn plan_patch(
    original: &OriginalProgram,
    compiled: &CompiledPatch,
    symbols: &ExternalSymbolTable,
) -> Result<PatchPlan, PatchFailure> {
    validate_external_declarations(&compiled.syntax, symbols, original.code_len)?;
    let external_functions =
        build_external_function_offsets(&original.unit, symbols, original.code_len)?;
    let external_data = build_external_data_offsets(symbols, original.code_len)?;
    let mut replacements = Vec::new();
    let mut exports = Vec::new();

    for (function, definition) in compiled.program.functions() {
        if let Some(address) = symbols.get_function(definition.name()) {
            replacements.push(PlannedReplacement {
                function,
                entry_offset: function_entry_offset(
                    definition.name(),
                    address,
                    symbols.base_address(),
                    original.code_len,
                )?,
            });
        } else if definition.linkage() == fsb_ir::Linkage::Exported {
            exports.push(PlannedExport {
                function,
                name: definition.name().to_owned(),
            });
        }
    }

    Ok(PatchPlan {
        replacements,
        exports,
        external_functions,
        external_data,
        output_symbols: symbols.clone(),
    })
}

fn assemble_patch(program: &Program, plan: PatchPlan) -> Result<AssembledPatch, PatchFailure> {
    let mut assembled = assemble_program(program)?;
    let mut replacements = Vec::with_capacity(plan.replacements.len());
    let mut exports = Vec::with_capacity(plan.exports.len());

    for replacement in plan.replacements {
        let replacement_offset =
            assembled
                .function_offset(replacement.function)
                .ok_or_else(|| {
                    fsb_assembler::error::AssemblerError::UndefinedSymbol(
                        replacement.function.to_string(),
                    )
                })?;
        assembled.make_function_private(replacement.function)?;
        replacements.push(AssembledReplacement {
            entry_offset: replacement.entry_offset,
            replacement_offset,
        });
    }

    for export in plan.exports {
        let offset = assembled.function_offset(export.function).ok_or_else(|| {
            fsb_assembler::error::AssemblerError::UndefinedSymbol(export.function.to_string())
        })?;
        exports.push(AssembledExport {
            name: export.name,
            offset,
        });
    }

    Ok(AssembledPatch {
        program: assembled,
        replacements,
        exports,
        external_functions: plan.external_functions,
        external_data: plan.external_data,
        output_symbols: plan.output_symbols,
    })
}

fn link_patch(
    mut original: OriginalProgram,
    assembled: AssembledPatch,
) -> Result<LinkedProgram, PatchFailure> {
    let patch_base = original.unit.append(assembled.program.into_unit())?;
    install_redirects(&mut original.unit, &assembled.replacements, patch_base)?;
    let mut output_symbols = assembled.output_symbols;

    for export in assembled.exports {
        let offset = patch_base
            .checked_add(export.offset)
            .ok_or(fsb_assembler::error::AssemblerError::AddressOverflow)?;
        let address = output_symbols
            .base_address()
            .checked_add(CODE_SECTION_FILE_OFFSET)
            .and_then(|code_address| code_address.checked_add(offset))
            .ok_or(fsb_assembler::error::AssemblerError::AddressOverflow)?;
        output_symbols.insert_function(export.name, address);
    }

    Ok(LinkedProgram {
        script_name: original.script_name,
        unit: original.unit,
        external_functions: assembled.external_functions,
        external_data: assembled.external_data,
        output_symbols,
    })
}

fn finalize_patch(linked: LinkedProgram) -> Result<PatchArtifact, PatchFailure> {
    let binary = linked
        .unit
        .into_binary_with_externals(
            linked.script_name,
            &linked.external_functions,
            &linked.external_data,
        )?
        .serialize()?;

    Ok(PatchArtifact {
        binary,
        symbols: linked.output_symbols,
    })
}

fn build_external_data_offsets(
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<ExternalDataOffsets, PatchFailure> {
    let mut external_data = ExternalDataOffsets::new();

    for (name, address) in symbols.data() {
        let offset = data_offset(name, address, symbols.base_address(), code_len)?;
        external_data.define(name, offset)?;
    }

    Ok(external_data)
}

fn build_external_function_offsets(
    original_unit: &AssemblyUnit,
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<ExternalFunctionOffsets, PatchFailure> {
    let mut external_functions = ExternalFunctionOffsets::new();

    for (name, address) in symbols.functions() {
        let external_offset =
            function_entry_offset(name, address, symbols.base_address(), code_len)?;
        if let Some(embedded_offset) = original_unit.function_offset(name)
            && embedded_offset != external_offset
        {
            return Err(PatchFailure::ConflictingExternalSymbol {
                function_name: name.to_owned(),
                embedded_offset,
                external_offset,
            });
        }
        external_functions.define(name, external_offset)?;
    }

    Ok(external_functions)
}

fn install_redirects(
    output_unit: &mut AssemblyUnit,
    replacements: &[AssembledReplacement],
    patch_base: u32,
) -> Result<(), PatchFailure> {
    for replacement in replacements {
        let target_offset = patch_base
            .checked_add(replacement.replacement_offset)
            .ok_or(fsb_assembler::error::AssemblerError::AddressOverflow)?;
        output_unit.redirect(replacement.entry_offset, target_offset)?;
    }

    Ok(())
}
