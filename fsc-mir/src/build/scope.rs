use crate::BasicBlockId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LoopScope {
    break_target: BasicBlockId,
    continue_target: BasicBlockId,
}

#[derive(Default)]
pub(super) struct ControlFlowScopes {
    loops: Vec<LoopScope>,
}

impl ControlFlowScopes {
    pub(super) fn enter_loop(
        &mut self,
        break_target: BasicBlockId,
        continue_target: BasicBlockId,
    ) {
        self.loops.push(LoopScope {
            break_target,
            continue_target,
        });
    }

    pub(super) fn exit_loop(
        &mut self,
        expected_break_target: BasicBlockId,
        expected_continue_target: BasicBlockId,
    ) {
        let scope = self.loops.pop();
        debug_assert_eq!(
            scope,
            Some(LoopScope {
                break_target: expected_break_target,
                continue_target: expected_continue_target,
            })
        );
    }

    pub(super) fn break_target(&self) -> Option<BasicBlockId> {
        self.loops.last().map(|scope| scope.break_target)
    }

    pub(super) fn continue_target(&self) -> Option<BasicBlockId> {
        self.loops.last().map(|scope| scope.continue_target)
    }
}
