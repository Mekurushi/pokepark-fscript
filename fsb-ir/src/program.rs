use crate::{BasicBlock, BlockId, DataId, FunctionId, Word};
use cranelift_entity::PrimaryMap;

#[derive(Debug, Default)]
pub struct Program {
    functions: PrimaryMap<FunctionId, Function>,
    data: PrimaryMap<DataId, Data>,
}

#[derive(Debug)]
pub struct Function {
    name: String,
    linkage: Linkage,
    blocks: PrimaryMap<BlockId, BasicBlock>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Linkage {
    Internal,
    Exported,
}

// TODO: support for sub-word data and all alignments etc. necessary for that; going for
// simplicity for now with word only to ensure alignments can't be wrong
#[derive(Debug)]
pub struct Data {
    name: Option<String>,
    words: Vec<Word>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_function(&mut self, name: impl Into<String>, linkage: Linkage) -> FunctionId {
        self.functions.push(Function::new(name.into(), linkage))
    }

    pub fn function(&self, id: FunctionId) -> &Function {
        &self.functions[id]
    }

    pub fn function_mut(&mut self, id: FunctionId) -> &mut Function {
        &mut self.functions[id]
    }

    pub fn functions(&self) -> impl Iterator<Item = (FunctionId, &Function)> {
        self.functions.iter()
    }

    pub fn create_data(&mut self, name: Option<String>, words: Vec<Word>) -> DataId {
        self.data.push(Data::new(name, words))
    }

    pub fn data(&self, id: DataId) -> &Data {
        &self.data[id]
    }

    pub fn data_mut(&mut self, id: DataId) -> &mut Data {
        &mut self.data[id]
    }

    pub fn data_objects(&self) -> impl Iterator<Item = (DataId, &Data)> {
        self.data.iter()
    }
}

impl Function {
    fn new(name: String, linkage: Linkage) -> Self {
        Self {
            name,
            linkage,
            blocks: PrimaryMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn linkage(&self) -> Linkage {
        self.linkage
    }

    pub fn create_block(&mut self) -> BlockId {
        self.blocks.push(BasicBlock::new())
    }

    pub fn block(&self, id: BlockId) -> &BasicBlock {
        &self.blocks[id]
    }

    pub fn block_mut(&mut self, id: BlockId) -> &mut BasicBlock {
        &mut self.blocks[id]
    }

    pub fn blocks(&self) -> impl Iterator<Item = (BlockId, &BasicBlock)> {
        self.blocks.iter()
    }
}

impl Data {
    fn new(name: Option<String>, words: Vec<Word>) -> Self {
        Self { name, words }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn words(&self) -> &[Word] {
        &self.words
    }

    pub fn words_mut(&mut self) -> &mut Vec<Word> {
        &mut self.words
    }
}
