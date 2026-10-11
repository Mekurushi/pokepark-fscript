use crate::error::CodegenResult;
use crate::lower::lower_func;
use fsb_assembler::Assembler;
use fsc_sema::thir::{FuncDef, Item, Script};

mod error;
mod frame;
mod lower;

pub fn compile(script: &Script, asm: &mut Assembler) -> CodegenResult<()> {
    for item in &script.items {
        compile_item(item, asm)?;
    }
    Ok(())
}

pub fn compile_func(func: &FuncDef, asm: &mut Assembler) -> CodegenResult<()> {
    lower_func(func, asm)
}

fn compile_item(item: &Item, asm: &mut Assembler) -> CodegenResult<()> {
    match item {
        Item::FuncDef(func) => compile_func(func, asm),
    }
}
