use crate::error::AssemblerResult;

pub struct BinaryStringTable {
    string_table: Vec<u8>,
}

impl BinaryStringTable {
    pub fn new(buffer: Vec<u8>) -> Self {
        Self {
            string_table: buffer,
        }
    }

    pub fn serialize(&self) -> AssemblerResult<Vec<u8>> {
        //TODO: script name as first string
        let mut buf = Vec::new();
        buf.extend_from_slice(self.string_table.as_slice());
        Ok(buf)
    }

    pub(crate) fn into_buffer(self) -> Vec<u8> {
        self.string_table
    }
}
