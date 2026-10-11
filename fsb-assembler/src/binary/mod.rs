use crate::binary::header::{FscriptHeader, HEADER_SIZE};
use crate::binary::string_table::BinaryStringTable;
use crate::binary::symbol_table::BinarySymbolTable;
use crate::error::{AssemblerError, AssemblerResult, BinaryReadError, BinaryReadResult};

mod b40string;
mod header;
pub mod string_table;
pub mod symbol_table;

pub const CODE_SECTION_FILE_OFFSET: u32 = HEADER_SIZE;

pub struct FscriptBinary {
    script_name: String,
    pub(crate) code: Vec<u8>,
    symbol_table: BinarySymbolTable,
    string_table: BinaryStringTable,
}

impl FscriptBinary {
    pub fn new(
        script_name: String,
        code: Vec<u8>,
        symbol_table: BinarySymbolTable,
        string_table: BinaryStringTable,
    ) -> Self {
        Self {
            script_name,
            code,
            symbol_table,
            string_table,
        }
    }

    pub fn serialize(&self) -> AssemblerResult<Vec<u8>> {
        let code_ptr = CODE_SECTION_FILE_OFFSET;
        let symbol_table_ptr = code_ptr
            + u32::try_from(self.code.len())
                .map_err(|_err| AssemblerError::SectionTooLarge("code"))?;
        let string_table_ptr = symbol_table_ptr + self.symbol_table.byte_len()?;
        let header = FscriptHeader::new(
            self.script_name.clone(),
            code_ptr,
            symbol_table_ptr,
            string_table_ptr,
        );

        let mut out = Vec::new();
        out.extend_from_slice(&header.serialize()?);
        out.extend_from_slice(&self.code);
        out.extend_from_slice(&self.symbol_table.serialize(code_ptr)?);
        out.extend_from_slice(&self.string_table.serialize()?);
        Ok(out)
    }

    pub fn deserialize(data: &[u8]) -> BinaryReadResult<FscriptBinary> {
        let header = FscriptHeader::deserialize(data)?;
        let instructions_ptr = usize::try_from(header.instructions_ptr)
            .map_err(|_err| BinaryReadError::InvalidSectionLayout)?;
        let symbol_table_ptr = usize::try_from(header.symbol_table_ptr)
            .map_err(|_err| BinaryReadError::InvalidSectionLayout)?;
        let string_table_ptr = usize::try_from(header.string_table_ptr)
            .map_err(|_err| BinaryReadError::InvalidSectionLayout)?;

        if instructions_ptr < HEADER_SIZE as usize
            || instructions_ptr > symbol_table_ptr
            || symbol_table_ptr > string_table_ptr
        {
            return Err(BinaryReadError::InvalidSectionLayout);
        }
        if string_table_ptr > data.len() {
            return Err(BinaryReadError::FileTooShort {
                minimum: string_table_ptr,
                actual: data.len(),
            });
        }

        let code = data[instructions_ptr..symbol_table_ptr].to_vec();
        let symbol_table = BinarySymbolTable::deserialize(
            &data[symbol_table_ptr..string_table_ptr],
            header.instructions_ptr,
        )?;
        let string_table = BinaryStringTable::new(data[string_table_ptr..].to_vec());

        Ok(Self::new(
            header.script_name,
            code,
            symbol_table,
            string_table,
        ))
    }

    pub fn code_len(&self) -> usize {
        self.code.len()
    }

    pub(crate) fn into_parts(self) -> (String, Vec<u8>, BinarySymbolTable, BinaryStringTable) {
        (
            self.script_name,
            self.code,
            self.symbol_table,
            self.string_table,
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn binary_round_trips() {
        let mut symbols = BinarySymbolTable::new();
        symbols.add("MAIN".to_owned(), 0);
        symbols.add("OTHER".to_owned(), 4);
        let binary = FscriptBinary::new(
            "TEST".to_owned(),
            vec![0, 1, 2, 3, 4, 5, 6, 7],
            symbols,
            BinaryStringTable::new(b"hello\0".to_vec()),
        );

        let serialized = binary.serialize().unwrap();
        let deserialized = FscriptBinary::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.serialize().unwrap(), serialized);
    }

    #[test]
    fn existing_binary_round_trips() {
        let data = include_bytes!("../../../scripts/add.fsb");

        let deserialized = FscriptBinary::deserialize(data).unwrap();

        assert_eq!(deserialized.serialize().unwrap(), data);
    }

    #[test]
    fn deserialize_rejects_out_of_order_sections() {
        let mut data = [0u8; HEADER_SIZE as usize];
        data[0..4].copy_from_slice(&HEADER_SIZE.to_be_bytes());
        data[4..8].copy_from_slice(&(HEADER_SIZE - 1).to_be_bytes());
        data[12..16].copy_from_slice(&HEADER_SIZE.to_be_bytes());

        assert_eq!(
            FscriptBinary::deserialize(&data).err(),
            Some(BinaryReadError::InvalidSectionLayout)
        );
    }

    #[test]
    fn deserialize_rejects_truncated_section() {
        let mut data = [0u8; HEADER_SIZE as usize];
        data[0..4].copy_from_slice(&HEADER_SIZE.to_be_bytes());
        data[4..8].copy_from_slice(&HEADER_SIZE.to_be_bytes());
        data[12..16].copy_from_slice(&(HEADER_SIZE + 1).to_be_bytes());

        assert_eq!(
            FscriptBinary::deserialize(&data).err(),
            Some(BinaryReadError::FileTooShort {
                minimum: HEADER_SIZE as usize + 1,
                actual: HEADER_SIZE as usize,
            })
        );
    }
}
