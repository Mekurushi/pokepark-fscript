use fsc_mir::BasicBlockId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatementLocation {
    pub(crate) block: BasicBlockId,
    pub(crate) statement: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsumerLocation {
    Statement(StatementLocation),
    Terminator(BasicBlockId),
}

impl ConsumerLocation {
    pub(crate) fn block(self) -> BasicBlockId {
        match self {
            Self::Statement(location) => location.block,
            Self::Terminator(block) => block,
        }
    }
}
