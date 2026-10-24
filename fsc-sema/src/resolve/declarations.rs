use std::collections::{HashMap, hash_map::Entry};

use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics, Span};
use fsc_parse::ast;

use super::error::ResolveError;
use crate::hir::ItemId;

#[derive(Debug, Clone, Copy)]
struct Declaration {
    source_index: usize,
    span: Span,
}

#[derive(Debug, Default)]
pub(super) struct Declarations {
    items: PrimaryMap<ItemId, Declaration>,
    by_name: HashMap<String, ItemId>,
}

impl Declarations {
    pub(super) fn resolve(&self, name: &str) -> Option<ItemId> {
        self.by_name.get(name).copied()
    }

    pub(super) fn source_items(&self) -> impl Iterator<Item = (ItemId, usize)> + '_ {
        self.items
            .iter()
            .map(|(id, declaration)| (id, declaration.source_index))
    }
}

pub(super) fn collect(script: &ast::Script, diagnostics: &mut Diagnostics) -> Declarations {
    let mut declarations = Declarations::default();

    for (source_index, item) in script.items.iter().enumerate() {
        let (name, span) = match item {
            ast::Item::FuncDef(function) => (&function.header.name, function.header.name_span),
            ast::Item::ExternFuncDecl(function) => {
                (&function.header.name, function.header.name_span)
            }
            ast::Item::StaticDecl(static_var) => (&static_var.name, static_var.name_span),
            ast::Item::ConstDecl(constant) => (&constant.name, constant.name_span),
            ast::Item::ConfigDecl(config) => (&config.name, config.name_span),
        };
        let id = declarations.items.push(Declaration { source_index, span });
        match declarations.by_name.entry(name.to_owned()) {
            Entry::Vacant(entry) => {
                entry.insert(id);
            }
            Entry::Occupied(entry) => {
                let original = &declarations.items[*entry.get()];
                diagnostics.emit_error(Diagnostic::from(ResolveError::DuplicateDeclaration {
                    name: name.to_owned(),
                    duplicate_span: span,
                    original_span: original.span,
                }));
            }
        }
    }

    declarations
}
