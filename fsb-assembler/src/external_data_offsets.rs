use std::collections::HashMap;

use crate::error::{AssemblerError, AssemblerResult};

#[derive(Default)]
pub struct ExternalDataOffsets {
    offsets: HashMap<String, u32>,
}

impl ExternalDataOffsets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn define(&mut self, name: impl Into<String>, offset: u32) -> AssemblerResult<()> {
        let name = name.into();
        if self.offsets.contains_key(&name) {
            return Err(AssemblerError::DuplicateSymbol(name));
        }
        self.offsets.insert(name, offset);
        Ok(())
    }

    pub(crate) fn resolve(&self, name: &str) -> Option<u32> {
        self.offsets.get(name).copied()
    }
}
