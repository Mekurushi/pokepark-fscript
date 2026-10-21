use super::{BasicBlockId, FunctionId, Operand, Place, Rvalue, SourceInfo};

#[derive(Debug, Clone, PartialEq)]
pub struct BasicBlock {
    pub statements: Vec<Statement>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: StatementKind,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    Assign {
        target: Place,
        value: Rvalue,
    },
    Call {
        destination: Option<Place>,
        callee: FunctionId,
        arguments: Box<[CallArgument]>,
    },
    RawSyscall {
        destination: Option<Place>,
        page: u8,
        function: u16,
        arguments: Box<[CallArgument]>,
    },
    Pause {
        duration: Operand,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CallArgument {
    Value(Operand),
    Reference(Place),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub source: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Goto {
        target: BasicBlockId,
    },
    Branch {
        condition: Operand,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    },
    // it returns the value stored in RETURN_PLACE so we can save ourselves the
    // reference here; one of the advantages of the decoupling
    Return,
    // it seems to be a core part of CFG; tbh no idea if that matches with this language, but I'm
    // keeping it in for now so it is easier to check whether we're blocking ourselves out from smth
    Unreachable,
}
