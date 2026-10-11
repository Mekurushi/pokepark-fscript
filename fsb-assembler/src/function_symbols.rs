use crate::error::{AssemblerError, AssemblerResult};
use std::collections::HashMap;

pub(crate) struct FunctionSymbol {
    offset: u32,
}

impl FunctionSymbol {
    pub const fn offset(&self) -> u32 {
        self.offset
    }
}

pub(crate) struct FunctionSymbols {
    functions: HashMap<String, FunctionSymbol>,
}

impl Default for FunctionSymbols {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionSymbols {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
        }
    }

    pub fn define_function(&mut self, name: String, offset: u32) -> AssemblerResult<()> {
        if self.functions.contains_key(&name) {
            return Err(AssemblerError::DuplicateSymbol(name));
        }
        self.functions.insert(name, FunctionSymbol { offset });
        Ok(())
    }

    pub fn resolve_function_offset(&self, name: &str) -> AssemblerResult<u32> {
        match self.functions.get(name) {
            Some(FunctionSymbol { offset }) => Ok(*offset),
            None => Err(AssemblerError::UndefinedSymbol(name.to_owned())),
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &FunctionSymbol)> {
        self.functions
            .iter()
            .map(|(name, symbol)| (name.as_str(), symbol))
    }

    pub(crate) fn rebase(&mut self, offset: u32) -> AssemblerResult<()> {
        if self
            .functions
            .values()
            .map(|symbol| symbol.offset)
            .any(|symbol_offset| symbol_offset.checked_add(offset).is_none())
        {
            return Err(AssemblerError::AddressOverflow);
        }

        for symbol in self.functions.values_mut() {
            symbol.offset += offset;
        }
        Ok(())
    }

    pub(crate) fn merge(&mut self, other: Self) -> AssemblerResult<()> {
        for (name, symbol) in other.functions {
            if self.functions.contains_key(&name) {
                return Err(AssemblerError::DuplicateSymbol(name));
            }
            self.functions.insert(name, symbol);
        }
        Ok(())
    }

    pub(crate) fn rename_function(
        &mut self,
        current_name: &str,
        new_name: &str,
    ) -> AssemblerResult<()> {
        let Some(_function) = self.functions.get(current_name) else {
            return Err(AssemblerError::UndefinedSymbol(current_name.to_owned()));
        };
        if self.functions.contains_key(new_name) {
            return Err(AssemblerError::DuplicateSymbol(new_name.to_owned()));
        }

        let function = self
            .functions
            .remove(current_name)
            .ok_or_else(|| AssemblerError::UndefinedSymbol(current_name.to_owned()))?;
        self.functions.insert(new_name.to_owned(), function);
        Ok(())
    }

    pub(crate) fn remove(&mut self, name: &str) -> AssemblerResult<u32> {
        self.functions
            .remove(name)
            .map(|symbol| symbol.offset)
            .ok_or_else(|| AssemblerError::UndefinedSymbol(name.to_owned()))
    }
}
