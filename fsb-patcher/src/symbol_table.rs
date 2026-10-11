use crate::{SymbolTableParseError, SymbolTableSerializeError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalSymbolTable {
    base_address: u32,
    functions: BTreeMap<String, u32>,
}

impl ExternalSymbolTable {
    pub fn new(base_address: u32) -> Self {
        Self {
            base_address,
            functions: BTreeMap::new(),
        }
    }

    pub const fn base_address(&self) -> u32 {
        self.base_address
    }

    pub fn insert_function(&mut self, name: String, address: u32) -> Option<u32> {
        self.functions.insert(name, address)
    }

    pub fn get_function(&self, name: &str) -> Option<u32> {
        self.functions.get(name).copied()
    }

    pub fn functions(&self) -> impl Iterator<Item = (&str, u32)> {
        self.functions
            .iter()
            .map(|(name, address)| (name.as_str(), *address))
    }

    pub fn len(&self) -> usize {
        self.functions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

pub fn parse_symbol_table(input: &str) -> Result<ExternalSymbolTable, SymbolTableParseError> {
    toml::from_str(input).map_err(|source| SymbolTableParseError { source })
}

pub fn serialize_symbol_table(
    symbols: &ExternalSymbolTable,
) -> Result<String, SymbolTableSerializeError> {
    toml::to_string(symbols).map_err(|source| SymbolTableSerializeError { source })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_hexadecimal_symbols_with_comments_and_whitespace() {
        let symbols = parse_symbol_table(
            r#"
                base_address = 0x50000

                [functions]
                # Exported functions
                MAIN = 0x50020
                UPDATE = 0x5005c
            "#,
        )
        .unwrap();

        assert_eq!(symbols.base_address(), 0x50000);
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols.get_function("MAIN"), Some(0x50020));
        assert_eq!(symbols.get_function("UPDATE"), Some(0x5005c));
    }

    #[test]
    fn rejects_duplicate_symbols() {
        assert!(
            parse_symbol_table(
                "base_address = 0x50000\n[functions]\nMAIN = 0x50020\nMAIN = 0x50024"
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_negative_addresses() {
        assert!(parse_symbol_table("base_address = 0x50000\n[functions]\nMAIN = -1").is_err());
    }

    #[test]
    fn rejects_addresses_larger_than_u32() {
        assert!(
            parse_symbol_table("base_address = 0x50000\n[functions]\nMAIN = 0x100000000").is_err()
        );
    }

    #[test]
    fn rejects_non_integer_addresses() {
        assert!(
            parse_symbol_table("base_address = 0x50000\n[functions]\nMAIN = \"0x50020\"").is_err()
        );
    }

    #[test]
    fn rejects_malformed_toml() {
        assert!(parse_symbol_table("MAIN: 0x50020").is_err());
    }

    #[test]
    fn rejects_missing_base_address() {
        assert!(parse_symbol_table("[functions]\nMAIN = 0x50020").is_err());
    }

    #[test]
    fn rejects_missing_functions_table() {
        assert!(parse_symbol_table("base_address = 0x50000").is_err());
    }

    #[test]
    fn serialized_symbols_round_trip() {
        let mut symbols = ExternalSymbolTable::new(0x50000);
        symbols.insert_function("UPDATE".to_owned(), 0x5005c);
        symbols.insert_function("MAIN".to_owned(), 0x50020);

        let serialized = serialize_symbol_table(&symbols).unwrap();
        let reparsed = parse_symbol_table(&serialized).unwrap();

        assert_eq!(reparsed, symbols);
        assert!(serialized.find("MAIN").unwrap() < serialized.find("UPDATE").unwrap());
    }
}
