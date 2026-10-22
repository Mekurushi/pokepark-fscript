use std::collections::HashMap;

use super::super::declarations::Declarations;
use super::super::error::ResolveError;
use crate::hir;
use crate::types::Ty;
use cranelift_entity::PrimaryMap;
use fsc_diagnostics::{Diagnostic, Diagnostics};
use fsc_parse::ast;

use super::body::BodyLowerer;
use super::expression::{ExprFieldIdAllocator, ExprIdAllocator, ExpressionLowerer};
use super::names::NameResolver;

pub(in crate::resolve) fn lower_script(
    script: &ast::Script,
    declarations: &Declarations,
    diagnostics: &mut Diagnostics,
) -> hir::Script {
    LoweringContext::new(declarations, diagnostics).lower_script(script)
}

struct LoweringContext<'declarations, 'diagnostics> {
    declarations: &'declarations Declarations,
    diagnostics: &'diagnostics mut Diagnostics,
    expression_ids: ExprIdAllocator,
    expr_field_ids: ExprFieldIdAllocator,
}

impl<'declarations, 'diagnostics> LoweringContext<'declarations, 'diagnostics> {
    fn new(
        declarations: &'declarations Declarations,
        diagnostics: &'diagnostics mut Diagnostics,
    ) -> Self {
        Self {
            declarations,
            diagnostics,
            expression_ids: ExprIdAllocator::default(),
            expr_field_ids: ExprFieldIdAllocator::default(),
        }
    }

    fn lower_script(&mut self, script: &ast::Script) -> hir::Script {
        let mut items = PrimaryMap::new();
        let source_items = self.declarations.source_items().collect::<Vec<_>>();
        for (id, source_index) in source_items {
            let lowered_id = items.push(self.lower_item(&script.items[source_index]));
            debug_assert_eq!(lowered_id, id);
        }
        hir::Script { items }
    }

    fn lower_item(&mut self, item: &ast::Item) -> hir::Item {
        match item {
            ast::Item::FuncDef(function) => hir::Item {
                name: function.header.name.clone(),
                name_span: function.header.name_span,
                kind: hir::ItemKind::Function(hir::Function {
                    signature: self.lower_signature(&function.header),
                    kind: hir::FunctionKind::Defined {
                        exported: function.exported,
                        body: BodyLowerer::new(
                            self.declarations,
                            self.diagnostics,
                            &mut self.expression_ids,
                            &mut self.expr_field_ids,
                        )
                        .lower(&function.header.params, &function.body),
                    },
                }),
            },
            ast::Item::ExternFuncDecl(extern_func) => hir::Item {
                name: extern_func.header.name.clone(),
                name_span: extern_func.header.name_span,
                kind: hir::ItemKind::Function(hir::Function {
                    signature: self.lower_signature(&extern_func.header),
                    kind: hir::FunctionKind::External,
                }),
            },
            ast::Item::ConstDecl(constant) => hir::Item {
                name: constant.name.clone(),
                name_span: constant.name_span,
                kind: hir::ItemKind::Constant(hir::Constant {
                    ty: Ty::from(&constant.ty),
                    type_span: constant.ty_span,
                    initializer: ExpressionLowerer::new(
                        NameResolver::global(self.declarations),
                        self.diagnostics,
                        &mut self.expression_ids,
                        &mut self.expr_field_ids,
                    )
                    .lower(&constant.initializer),
                }),
            },
            ast::Item::ConfigDecl(config) => hir::Item {
                name: config.name.clone(),
                name_span: config.name_span,
                kind: hir::ItemKind::Config(hir::Config {
                    ty: Ty::from(&config.ty),
                    type_span: config.ty_span,
                }),
            },
        }
    }
    fn lower_signature(&mut self, header: &ast::FunctionHeader) -> hir::FunctionSignature {
        let mut parameter_names = HashMap::new();
        let mut parameters = Vec::with_capacity(header.params.len());

        for parameter in &header.params {
            if let Some(&original_span) = parameter_names.get(parameter.name.as_str()) {
                self.diagnostics
                    .emit_error(Diagnostic::from(ResolveError::DuplicateDeclaration {
                        name: parameter.name.clone(),
                        duplicate_span: parameter.name_span,
                        original_span,
                    }));
            } else {
                parameter_names.insert(parameter.name.as_str(), parameter.name_span);
            }
            parameters.push(hir::Parameter {
                ty: Ty::from(&parameter.ty),
                type_span: parameter.ty_span,
            });
        }

        hir::FunctionSignature {
            parameters,
            return_type: Ty::from(&header.ret_ty),
            return_type_span: header.ret_ty_span,
        }
    }
}
