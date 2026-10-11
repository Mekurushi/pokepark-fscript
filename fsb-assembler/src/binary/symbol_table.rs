use crate::binary::b40string::{decode_b40, encode_b40};
use crate::error::{AssemblerError, AssemblerResult, BinaryReadError, BinaryReadResult};
const SYMBOL_ENTRY_SIZE: usize = 12;

pub struct BinarySymbolTable {
    entries: Vec<SymbolEntry>,
}

struct SymbolEntry {
    pub name: String, // plain string, encoded to B40 string on serialization
    pub offset: u32,  // file_offset; modified in serialization
}

impl SymbolEntry {
    pub fn serialize(&self, code_ptr: u32) -> AssemblerResult<[u8; SYMBOL_ENTRY_SIZE]> {
        let mut buf = [0u8; SYMBOL_ENTRY_SIZE];
        buf[0x00..0x08].copy_from_slice(&encode_b40(&self.name)?);
        // TODO: replace hardcoded HEADER_SIZE with other method to get file offset
        buf[0x08..0x0c].copy_from_slice(&((self.offset + code_ptr) / 4).to_be_bytes());

        Ok(buf)
    }

    pub fn deserialize(entry: &[u8; SYMBOL_ENTRY_SIZE], code_ptr: u32) -> BinaryReadResult<Self> {
        let mut encoded_name = [0; 8];
        encoded_name.copy_from_slice(&entry[..8]);

        let absolute_word_offset = u32::from_be_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let absolute_offset =
            absolute_word_offset
                .checked_mul(4)
                .ok_or(BinaryReadError::InvalidSymbolOffset {
                    offset: absolute_word_offset,
                })?;
        let offset =
            absolute_offset
                .checked_sub(code_ptr)
                .ok_or(BinaryReadError::InvalidSymbolOffset {
                    offset: absolute_offset,
                })?;

        Ok(Self {
            name: decode_b40(encoded_name)?,
            offset,
        })
    }
}

impl Default for BinarySymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl BinarySymbolTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, name: String, offset: u32) {
        self.entries.push(SymbolEntry { name, offset });
    }

    pub fn serialize(&self, code_ptr: u32) -> AssemblerResult<Vec<u8>> {
        let mut buf = Vec::new();
        for entry in &self.entries {
            buf.extend_from_slice(&entry.serialize(code_ptr)?);
        }
        Ok(buf)
    }

    pub fn deserialize(data: &[u8], code_ptr: u32) -> BinaryReadResult<Self> {
        if !data.len().is_multiple_of(SYMBOL_ENTRY_SIZE) {
            return Err(BinaryReadError::InvalidSymbolTableSize { size: data.len() });
        }

        let mut table = Self::new();
        let (entries, _) = data.as_chunks::<SYMBOL_ENTRY_SIZE>();
        for entry in entries {
            table
                .entries
                .push(SymbolEntry::deserialize(entry, code_ptr)?);
        }
        Ok(table)
    }

    pub fn byte_len(&self) -> AssemblerResult<u32> {
        Ok(u32::try_from(self.entries.len())
            .map_err(|_err| AssemblerError::SectionTooLarge("Symbol_Table"))?
            * SYMBOL_ENTRY_SIZE as u32)
    }

    pub(crate) fn into_entries(self) -> impl Iterator<Item = (String, u32)> {
        self.entries
            .into_iter()
            .map(|entry| (entry.name, entry.offset))
    }
}
