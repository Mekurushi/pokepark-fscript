use crate::binary::string_table::BinaryStringTable;
use crate::error::{AssemblerError, AssemblerResult};
use std::collections::HashMap;
const MAX_STRING_TABLE_SIZE: u32 = u16::MAX as u32;

pub struct StringTable {
    buffer: Vec<u8>,
    index: HashMap<String, u32>,
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

impl StringTable {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn from_binary(table: BinaryStringTable) -> AssemblerResult<Self> {
        let buffer = table.into_buffer();
        if !buffer.is_empty() && !buffer.ends_with(&[0]) {
            return Err(AssemblerError::InvalidStringTable);
        }

        let mut index = HashMap::new();
        let mut start = 0;
        for (end, byte) in buffer.iter().enumerate() {
            if *byte != 0 {
                continue;
            }

            let value = std::str::from_utf8(&buffer[start..end])
                .map_err(|_error| AssemblerError::InvalidStringTable)?;
            let offset =
                u32::try_from(start).map_err(|_error| AssemblerError::InvalidStringTable)?;
            index.entry(value.to_owned()).or_insert(offset);
            start = end + 1;
        }

        Ok(Self { buffer, index })
    }

    pub fn intern(&mut self, s: &str) -> AssemblerResult<u32> {
        if let Some(&offset) = self.index.get(s) {
            return Ok(offset);
        }
        let offset =
            u32::try_from(self.buffer.len()).map_err(|_foo| AssemblerError::StringTableFull)?;

        if offset > MAX_STRING_TABLE_SIZE {
            return Err(AssemblerError::StringTableFull);
        }
        self.buffer.extend_from_slice(s.as_bytes());
        self.buffer.push(0);
        self.index.insert(s.to_string(), offset);
        Ok(offset)
    }

    pub fn lookup(&self, s: &str) -> Option<u32> {
        self.index.get(s).copied()
    }

    pub(crate) fn merge(&mut self, other: &Self) -> AssemblerResult<()> {
        let mut start = 0;
        for (end, byte) in other.buffer.iter().enumerate() {
            if *byte != 0 {
                continue;
            }

            let value = std::str::from_utf8(&other.buffer[start..end])
                .map_err(|_error| AssemblerError::InvalidStringTable)?;
            self.intern(value)?;
            start = end + 1;
        }
        Ok(())
    }

    pub fn into_binary(self) -> BinaryStringTable {
        BinaryStringTable::new(self.buffer)
    }
}
