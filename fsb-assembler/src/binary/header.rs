use crate::binary::b40string::{decode_b40, encode_b40};
use crate::error::{AssemblerResult, BinaryReadError, BinaryReadResult};

// Header
pub const HEADER_SIZE: u32 = 0x20;

pub struct FscriptHeader {
    // plain string, encoded to B40 string on serialization
    pub script_name: String,
    pub instructions_ptr: u32,
    pub symbol_table_ptr: u32,
    pub string_table_ptr: u32,
}

impl FscriptHeader {
    pub fn new(
        script_name: String,
        instructions_ptr: u32,
        symbol_table_ptr: u32,
        string_table_ptr: u32,
    ) -> Self {
        Self {
            script_name,
            instructions_ptr,
            symbol_table_ptr,
            string_table_ptr,
        }
    }

    pub fn serialize(&self) -> AssemblerResult<[u8; 32]> {
        let mut buf = [0u8; 32];
        buf[0x00..0x04].copy_from_slice(&self.instructions_ptr.to_be_bytes());
        buf[0x04..0x08].copy_from_slice(&self.symbol_table_ptr.to_be_bytes());
        buf[0x08..0x0c].copy_from_slice(&self.string_table_ptr.to_be_bytes()); // unused, mirrors
        buf[0x0c..0x10].copy_from_slice(&self.string_table_ptr.to_be_bytes());
        buf[0x10..0x14].copy_from_slice(&0u32.to_be_bytes()); // unused2
        buf[0x14..0x1c].copy_from_slice(&encode_b40(&self.script_name)?);
        buf[0x1c..0x20].copy_from_slice(&0x0100_0000_u32.to_be_bytes()); // unused3
        Ok(buf)
    }

    #[allow(unused)]
    pub fn deserialize(data: &[u8]) -> BinaryReadResult<FscriptHeader> {
        let header = data.first_chunk::<{ HEADER_SIZE as usize }>().ok_or(
            BinaryReadError::FileTooShort {
                minimum: HEADER_SIZE as usize,
                actual: data.len(),
            },
        )?;

        let (words, _) = header.as_chunks::<4>();

        let instructions_ptr = u32::from_be_bytes(words[0]);
        let symbol_table_ptr = u32::from_be_bytes(words[1]);
        let string_table_ptr = u32::from_be_bytes(words[3]);

        let mut script_name_bytes = [0; 8];
        script_name_bytes.copy_from_slice(&header[0x14..0x1c]);
        let script_name = decode_b40(script_name_bytes)?;

        Ok(Self {
            instructions_ptr,
            symbol_table_ptr,
            string_table_ptr,
            script_name,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn header_round_trips() {
        let header = FscriptHeader::new("EVAR01ZN01_N".to_owned(), 0x20, 0x80, 0xa0);
        let serialized = header.serialize().unwrap();
        let deserialized = FscriptHeader::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.script_name, header.script_name);
        assert_eq!(deserialized.instructions_ptr, header.instructions_ptr);
        assert_eq!(deserialized.symbol_table_ptr, header.symbol_table_ptr);
        assert_eq!(deserialized.string_table_ptr, header.string_table_ptr);
    }

    #[test]
    fn deserialize_rejects_truncated_header() {
        let data = [0u8; HEADER_SIZE as usize - 1];

        assert!(matches!(
            FscriptHeader::deserialize(&data),
            Err(BinaryReadError::FileTooShort {
                minimum,
                actual,
            })
            if minimum == HEADER_SIZE as usize && actual == data.len()
        ));
    }
}
