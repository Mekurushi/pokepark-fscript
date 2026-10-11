mod assembler;
pub mod assembly_unit;
pub mod binary;
mod emission;
pub mod encoding;
pub mod error;
mod external_data_offsets;
mod external_function_offsets;
mod function_symbols;
pub mod string_table;

pub use assembler::assemble_program;
pub use assembly_unit::AssemblyUnit;
pub use external_data_offsets::ExternalDataOffsets;
pub use external_function_offsets::ExternalFunctionOffsets;
