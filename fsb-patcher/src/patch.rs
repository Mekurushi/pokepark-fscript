use crate::{ConfigValues, ExternalSymbolTable, PatchFailure};
use fsb_assembler::binary::{CODE_SECTION_FILE_OFFSET, FscriptBinary};
use fsb_assembler::{Assembler, AssemblyUnit};
use fsc_diagnostics::{Diagnostic, Stage};
use fsc_parse::ast::{Item, Script};

struct ClassifiedFunction {
    function_name: String,
    patch_kind: FunctionPatchKind,
}

enum FunctionPatchKind {
    Replacement {
        entry_offset: u32,
        generated_name: String,
    },
    Custom,
}

fn classify_functions(
    script: &Script,
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<Vec<ClassifiedFunction>, PatchFailure> {
    script
        .items
        .iter()
        .filter_map(|item| match item {
            Item::FuncDef(function) => Some(function),
            Item::FuncDecl(_) | Item::ConstDecl(_) | Item::ConfigDecl(_) => None,
        })
        .map(|function| {
            let patch_kind = match symbols.get_function(&function.header.name) {
                // current behavior sets all functions that are listed in the original_symbols as
                // replacement targets
                Some(original_address) => {
                    let entry_offset = function_entry_offset(
                        &function.header.name,
                        original_address,
                        symbols.base_address(),
                        code_len,
                    )?;
                    FunctionPatchKind::Replacement {
                        entry_offset,
                        // using $ to ensure uniqueness, because it can't be used by the normal flow
                        generated_name: format!("$patch$replacement${}", function.header.name),
                    }
                }
                None => FunctionPatchKind::Custom,
            };

            Ok(ClassifiedFunction {
                function_name: function.header.name.clone(),
                patch_kind,
            })
        })
        .collect()
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

fn validate_external_declarations(
    script: &Script,
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<(), PatchFailure> {
    for declaration in script.items.iter().filter_map(|item| match item {
        Item::FuncDef(_) => None,
        Item::FuncDecl(declaration) => Some(declaration),
        Item::ConstDecl(_) | Item::ConfigDecl(_) => None,
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
    let binary = FscriptBinary::deserialize(request.original_binary)?;
    let original_code_len = binary.code_len();
    let script = fsc_parse::parse(request.patch_source)
        .map_err(|error| PatchFailure::InvalidPatchSource(error.into_diagnostics()))?;
    let functions = classify_functions(&script, request.symbols, original_code_len)?;
    let checked = fsc_sema::check(script)
        .map_err(|error| PatchFailure::InvalidPatchSource(vec![error.into()]))?;
    validate_external_declarations(checked.ast(), request.symbols, original_code_len)?;
    let bound = fsc_sema::bind_configs(checked, request.config_values)
        .map_err(|errors| PatchFailure::InvalidPatchSource(errors.into_diagnostics()))?;
    let hir = fsc_sema::lower(bound)
        .map_err(|error| PatchFailure::InvalidPatchSource(vec![error.into()]))?;

    let (script_name, mut original_unit) = AssemblyUnit::from_binary(binary)?;
    let mut assembler = Assembler::new();
    fsc_codegen::compile(&hir, &mut assembler).map_err(|error| {
        PatchFailure::InvalidPatchSource(vec![Diagnostic::error(Stage::Codegen, error.to_string())])
    })?;
    let mut patch_unit = assembler.into_unit();

    prepare_replacements(&functions, &mut patch_unit)?;
    install_external_symbols(&mut original_unit, request.symbols, original_code_len)?;

    let mut merged_unit = original_unit.merge(patch_unit)?;
    install_redirects(&mut merged_unit, &functions)?;
    let output_symbols = build_output_symbols(request.symbols, &functions, &merged_unit)?;
    let binary = merged_unit.into_binary(script_name)?.serialize()?;

    Ok(PatchArtifact {
        binary,
        symbols: output_symbols,
    })
}

fn prepare_replacements(
    functions: &[ClassifiedFunction],
    patch_unit: &mut AssemblyUnit,
) -> Result<(), PatchFailure> {
    for classified in functions {
        let FunctionPatchKind::Replacement { generated_name, .. } = &classified.patch_kind else {
            continue;
        };
        let original_name = &classified.function_name;
        patch_unit.rename_function(original_name, generated_name)?;
        patch_unit.make_function_private(generated_name)?;
    }

    Ok(())
}

fn install_external_symbols(
    original_unit: &mut AssemblyUnit,
    symbols: &ExternalSymbolTable,
    code_len: usize,
) -> Result<(), PatchFailure> {
    for (name, address) in symbols.functions() {
        let external_offset =
            function_entry_offset(name, address, symbols.base_address(), code_len)?;
        if let Some(embedded_offset) = original_unit.function_offset(name) {
            if embedded_offset != external_offset {
                return Err(PatchFailure::ConflictingExternalSymbol {
                    function_name: name.to_owned(),
                    embedded_offset,
                    external_offset,
                });
            }
        } else {
            original_unit.define_private_function(name, external_offset)?;
        }
    }

    Ok(())
}

fn install_redirects(
    merged_unit: &mut AssemblyUnit,
    functions: &[ClassifiedFunction],
) -> Result<(), PatchFailure> {
    for classified in functions {
        let FunctionPatchKind::Replacement {
            entry_offset,
            generated_name,
        } = &classified.patch_kind
        else {
            continue;
        };
        let target_offset = merged_unit.function_offset(generated_name).ok_or_else(|| {
            fsb_assembler::error::AssemblerError::UndefinedSymbol(generated_name.clone())
        })?;
        merged_unit.redirect(*entry_offset, target_offset)?;
    }

    Ok(())
}

fn build_output_symbols(
    input_symbols: &ExternalSymbolTable,
    functions: &[ClassifiedFunction],
    merged_unit: &AssemblyUnit,
) -> Result<ExternalSymbolTable, PatchFailure> {
    // TODO: rethink symbol output
    let mut output = input_symbols.clone();

    for classified in functions {
        if !matches!(classified.patch_kind, FunctionPatchKind::Custom) {
            continue;
        }
        let name = &classified.function_name;
        let offset = merged_unit
            .function_offset(name)
            .ok_or_else(|| fsb_assembler::error::AssemblerError::UndefinedSymbol(name.clone()))?;
        let address = input_symbols
            .base_address()
            .checked_add(CODE_SECTION_FILE_OFFSET)
            .and_then(|code_address| code_address.checked_add(offset))
            .ok_or(fsb_assembler::error::AssemblerError::AddressOverflow)?;
        output.insert_function(name.clone(), address);
    }

    Ok(output)
}
