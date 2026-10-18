use crate::hir;
use cranelift_entity::SecondaryMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedSelectors {
    pub(crate) page: u8,
    pub(crate) function: u16,
}

#[derive(Debug, Default)]
pub(crate) struct RawSyscallResults {
    selectors: SecondaryMap<hir::ExprId, Option<ResolvedSelectors>>,
}

impl RawSyscallResults {
    pub(super) fn record(&mut self, expression: hir::ExprId, selectors: ResolvedSelectors) {
        let previous = self.selectors.insert(expression, Some(selectors));
        debug_assert!(
            previous.flatten().is_none(),
            "raw syscall selectors for {expression:?} were recorded more than once"
        );
    }

    pub(crate) fn selectors(&self, expression: hir::ExprId) -> Option<ResolvedSelectors> {
        self.selectors.get(expression).and_then(|value| *value)
    }
}
