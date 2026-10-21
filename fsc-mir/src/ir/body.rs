use super::{BasicBlock, BasicBlockId, LocalId, SourceInfo, VariableDebugInfo};
use cranelift_entity::{EntityRef, PrimaryMap};
use fsc_sema::types::Ty;

#[derive(Debug)]
pub struct Body {
    pub argument_count: usize,
    // first element is the return value, followed by argument_count of parameters, afterward
    // are the remaining locals. Also function parameters are ordered after the function signature
    // so it's "implicitly" possible to fetch the needed local based on it.
    // Personally I wanted to avoid this, but it seems to be something usually done
    // 1..=argument_count are parameters in signature order
    pub locals: PrimaryMap<LocalId, LocalDecl>,
    pub variable_debug_info: Vec<VariableDebugInfo>,
    // START_BLOCK is the entry block into the function, so every body needs to contain it
    pub blocks: PrimaryMap<BasicBlockId, BasicBlock>,
}

impl Body {
    pub fn local_kind(&self, local: LocalId) -> LocalKind {
        let index = local.index();
        if local == super::RETURN_PLACE {
            LocalKind::Return
        } else if index <= self.argument_count {
            LocalKind::Parameter { index: index - 1 }
        } else {
            LocalKind::Local
        }
    }
}

#[derive(Debug)]
pub struct LocalDecl {
    pub ty: Ty,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    Return,
    Parameter { index: usize },
    Local,
}
