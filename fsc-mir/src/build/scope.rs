use crate::BasicBlockId;

#[derive(Default)]
pub(super) struct ControlFlowScopes {
    break_targets: Vec<BasicBlockId>,
}

impl ControlFlowScopes {
    pub(super) fn enter_loop(&mut self, break_target: BasicBlockId) {
        self.break_targets.push(break_target);
    }

    pub(super) fn exit_loop(&mut self, expected_target: BasicBlockId) {
        let target = self.break_targets.pop();
        debug_assert_eq!(target, Some(expected_target));
    }

    pub(super) fn break_target(&self) -> Option<BasicBlockId> {
        self.break_targets.last().copied()
    }
}
