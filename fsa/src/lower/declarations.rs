use std::collections::HashMap;

use fsc_vm_ir::{DataRef, ExternalData, ExternalFunction, FunctionRef, Program, Word};

use super::error::{LowerError, LowerResult};
use crate::ast::{self, Item, Script};

pub(super) struct DeclaredProgram {
    pub(super) program: Program,
    pub(super) symbols: Symbols,
}

#[derive(Default)]
pub(super) struct Symbols {
    pub(super) functions: HashMap<String, FunctionRef>,
    pub(super) data: HashMap<String, DataRef>,
}

pub(super) fn collect(script: &Script) -> LowerResult<DeclaredProgram> {
    let mut declared = DeclaredProgram {
        program: Program::new(),
        symbols: Symbols::default(),
    };

    for item in &script.items {
        match item {
            Item::Function(function) => declared.declare_function(function)?,
            Item::ExternalFunction(name) => declared.declare_external_function(name)?,
            Item::Data(data) => declared.declare_data(data)?,
            Item::ExternalData(name) => declared.declare_external_data(name)?,
        }
    }

    Ok(declared)
}

impl DeclaredProgram {
    fn declare_function(&mut self, function: &ast::Function) -> LowerResult<()> {
        if self.symbols.functions.contains_key(&function.name) {
            return Err(LowerError::DuplicateFunction(function.name.clone()));
        }

        let linkage = match function.linkage {
            ast::Linkage::Internal => fsc_vm_ir::Linkage::Internal,
            ast::Linkage::Exported => fsc_vm_ir::Linkage::Exported,
        };
        let id = self.program.create_function(function.name.clone(), linkage);

        self.symbols
            .functions
            .insert(function.name.clone(), FunctionRef::Internal(id));
        Ok(())
    }

    fn declare_external_function(&mut self, name: &str) -> LowerResult<()> {
        if self.symbols.functions.contains_key(name) {
            return Err(LowerError::DuplicateFunction(name.to_owned()));
        }

        self.symbols.functions.insert(
            name.to_owned(),
            FunctionRef::External(ExternalFunction::new(name)),
        );
        Ok(())
    }

    fn declare_data(&mut self, data: &ast::Data) -> LowerResult<()> {
        if self.symbols.data.contains_key(&data.name) {
            return Err(LowerError::DuplicateData(data.name.clone()));
        }

        let words = data.words.iter().copied().map(Word::from_bits).collect();
        let id = self.program.create_data(Some(data.name.clone()), words);
        self.symbols
            .data
            .insert(data.name.clone(), DataRef::Internal(id));
        Ok(())
    }

    fn declare_external_data(&mut self, name: &str) -> LowerResult<()> {
        if self.symbols.data.contains_key(name) {
            return Err(LowerError::DuplicateData(name.to_owned()));
        }

        self.symbols
            .data
            .insert(name.to_owned(), DataRef::External(ExternalData::new(name)));
        Ok(())
    }
}
