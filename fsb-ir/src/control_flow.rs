use crate::{BlockId, Instruction};

#[derive(Debug, Default)]
pub struct BasicBlock {
    instructions: Vec<Instruction>,
    terminator: Option<Terminator>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Terminator {
    Jump(BlockId),
    Branch {
        kind: BranchKind,
        taken: BlockId,
        not_taken: BlockId,
    },
    Return {
        kind: ReturnKind,
        frame_slots: u16,
    },
    Exit(ExitKind),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchKind {
    IfZero(BranchMode),
    IfNonZero(BranchMode),
    IfEqual,
    IfEqualImmediate(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchMode {
    Normal,
    Pause,
    Set,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnKind {
    None,
    Value,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitKind {
    // TODO: semantic names are not safe
    Abort,
    Reset,
}

impl BasicBlock {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    pub fn instructions_mut(&mut self) -> &mut [Instruction] {
        &mut self.instructions
    }

    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn terminator(&self) -> Option<&Terminator> {
        self.terminator.as_ref()
    }

    pub fn terminator_mut(&mut self) -> Option<&mut Terminator> {
        self.terminator.as_mut()
    }

    pub fn set_terminator(&mut self, terminator: Terminator) {
        self.terminator = Some(terminator);
    }

    pub fn take_terminator(&mut self) -> Option<Terminator> {
        self.terminator.take()
    }
}
