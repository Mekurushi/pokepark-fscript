use crate::{
    BasicBlock, BasicBlockId, START_BLOCK, SourceInfo, Statement, Terminator, TerminatorKind,
};
use cranelift_entity::PrimaryMap;

struct PendingBlock {
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

pub(super) struct CfgBuilder {
    blocks: PrimaryMap<BasicBlockId, PendingBlock>,
    current: Option<BasicBlockId>,
}

impl CfgBuilder {
    pub(super) fn new() -> Self {
        let mut blocks = PrimaryMap::new();
        let entry = blocks.push(PendingBlock {
            statements: Vec::new(),
            terminator: None,
        });
        debug_assert_eq!(entry, START_BLOCK);

        Self {
            blocks,
            current: Some(entry),
        }
    }

    pub(super) fn new_block(&mut self) -> BasicBlockId {
        self.blocks.push(PendingBlock {
            statements: Vec::new(),
            terminator: None,
        })
    }

    pub(super) fn switch_to(&mut self, block: BasicBlockId) {
        debug_assert!(
            self.current.is_none(),
            "current MIR block is not terminated"
        );
        debug_assert!(
            self.blocks[block].terminator.is_none(),
            "cannot reopen a terminated MIR block"
        );
        self.current = Some(block);
    }

    pub(super) const fn has_current_block(&self) -> bool {
        self.current.is_some()
    }

    pub(super) fn push_statement(&mut self, statement: Statement) {
        let Some(current) = self.current else {
            unreachable!("cannot emit a statement without a current MIR block");
        };
        self.blocks[current].statements.push(statement);
    }

    pub(super) fn terminate(&mut self, terminator: Terminator) {
        let Some(current) = self.current.take() else {
            unreachable!("cannot terminate without a current MIR block");
        };
        debug_assert!(self.blocks[current].terminator.is_none());
        self.blocks[current].terminator = Some(terminator);
    }

    pub(super) fn goto(&mut self, target: BasicBlockId, source: SourceInfo) {
        self.terminate(Terminator {
            kind: TerminatorKind::Goto { target },
            source,
        });
    }

    pub(super) fn finish(mut self, source: SourceInfo) -> PrimaryMap<BasicBlockId, BasicBlock> {
        if self.has_current_block() {
            self.terminate(Terminator {
                kind: TerminatorKind::Return,
                source,
            });
        }

        let mut blocks = PrimaryMap::new();
        for (expected, block) in self.blocks {
            let Some(terminator) = block.terminator else {
                unreachable!("unfinished MIR block reached CFG finalization");
            };
            let actual = blocks.push(BasicBlock {
                statements: block.statements,
                terminator,
            });
            debug_assert_eq!(actual, expected);
        }
        blocks
    }
}
