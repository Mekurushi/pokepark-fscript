use std::collections::{HashMap, hash_map::Entry};

use super::super::declarations::Declarations;
use crate::hir;
use fsc_diagnostics::Span;

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalBinding {
    id: hir::LocalId,
    pub(super) declaration_span: Span,
}

#[derive(Debug, Default)]
pub(super) struct LocalScopeStack {
    function: HashMap<String, LocalBinding>,
    blocks: Vec<HashMap<String, LocalBinding>>,
}

impl LocalScopeStack {
    pub(super) fn enter_block(&mut self) {
        self.blocks.push(HashMap::new());
    }

    pub(super) fn exit_block(&mut self) {
        self.blocks.pop();
    }

    pub(super) fn try_declare(
        &mut self,
        name: &str,
        id: hir::LocalId,
        span: Span,
    ) -> Result<(), LocalBinding> {
        let scope = match self.blocks.last_mut() {
            Some(block) => block,
            None => &mut self.function,
        };
        let binding = LocalBinding {
            id,
            declaration_span: span,
        };
        match scope.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(binding);
                Ok(())
            }
            Entry::Occupied(entry) => Err(*entry.get()),
        }
    }

    fn resolve(&self, name: &str) -> Option<hir::LocalId> {
        self.blocks
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).map(|binding| binding.id))
            .or_else(|| self.function.get(name).map(|binding| binding.id))
    }
}

#[derive(Clone, Copy)]
pub(super) enum NameResolver<'a> {
    Global(&'a Declarations),
    Function {
        declarations: &'a Declarations,
        locals: &'a LocalScopeStack,
    },
}

impl<'a> NameResolver<'a> {
    pub(super) const fn global(declarations: &'a Declarations) -> Self {
        Self::Global(declarations)
    }

    pub(super) const fn function(
        declarations: &'a Declarations,
        locals: &'a LocalScopeStack,
    ) -> Self {
        Self::Function {
            declarations,
            locals,
        }
    }

    pub(super) fn resolve(self, name: &str) -> Option<hir::ExprKind> {
        match self {
            Self::Global(declarations) => declarations.resolve(name).map(hir::ExprKind::ItemRef),
            Self::Function {
                declarations,
                locals,
            } => {
                if let Some(local) = locals.resolve(name) {
                    return Some(hir::ExprKind::Local(local));
                }
                declarations.resolve(name).map(hir::ExprKind::ItemRef)
            }
        }
    }
}
