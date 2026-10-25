use super::context::LoweringContext;
use crate::hir;
use crate::thir;
use cranelift_entity::{PrimaryMap, SecondaryMap};

pub(super) struct BodyLowerer<'context, 'analysis> {
    pub(super) context: &'context LoweringContext<'analysis>,
    pub(super) owner: hir::ItemId,
    pub(super) body: &'analysis hir::Body,
    locals: PrimaryMap<thir::LocalId, thir::Local>,
    pub(super) local_mapping: SecondaryMap<hir::LocalId, Option<thir::LocalId>>,
    parameter_locals: Box<[thir::LocalId]>,
    pub(super) expressions: PrimaryMap<thir::ExprId, thir::Expr>,
}

impl<'context, 'analysis> BodyLowerer<'context, 'analysis> {
    pub(super) fn new(
        context: &'context LoweringContext<'analysis>,
        owner: hir::ItemId,
        body: &'analysis hir::Body,
    ) -> Self {
        let mut locals = PrimaryMap::new();
        let mut local_mapping = SecondaryMap::new();

        for (local, declaration) in &body.locals {
            let kind = match declaration.kind {
                hir::LocalKind::Parameter { index } => thir::LocalKind::Parameter { index },
                hir::LocalKind::Variable => thir::LocalKind::Variable,
                hir::LocalKind::Constant => continue,
            };
            let lowered = locals.push(thir::Local {
                name: declaration.name.clone(),
                ty: declaration.ty.clone(),
                kind,
                source: thir::SourceInfo {
                    span: declaration.name_span,
                },
            });
            local_mapping.insert(local, Some(lowered));
        }

        let parameter_locals = body
            .parameter_locals
            .iter()
            .map(|local| Self::mapped_local(&local_mapping, *local))
            .collect();

        Self {
            context,
            owner,
            body,
            locals,
            local_mapping,
            parameter_locals,
            expressions: PrimaryMap::new(),
        }
    }

    pub(super) fn mapped_local(
        local_mapping: &SecondaryMap<hir::LocalId, Option<thir::LocalId>>,
        local: hir::LocalId,
    ) -> thir::LocalId {
        let Some(local) = local_mapping.get(local).and_then(|local| *local) else {
            unreachable!("non-runtime local reached THIR local lowering");
        };
        local
    }

    pub(super) fn lower_block(&mut self, block: &hir::Block) -> thir::Block {
        thir::Block {
            statements: block
                .statements
                .iter()
                .filter_map(|statement| self.lower_statement(statement))
                .collect(),
        }
    }

    fn lower_statement(&mut self, statement: &hir::Stmt) -> Option<thir::Stmt> {
        let kind = match &statement.kind {
            hir::StmtKind::Return(value) => thir::StmtKind::Return {
                value: value
                    .as_ref()
                    .map(|expression| self.lower_expression(expression)),
            },
            hir::StmtKind::Break => thir::StmtKind::Break,
            hir::StmtKind::Continue => thir::StmtKind::Continue,
            hir::StmtKind::VariableDeclaration { local, initializer } => {
                thir::StmtKind::LocalDeclaration {
                    local: Self::mapped_local(&self.local_mapping, *local),
                    initializer: initializer
                        .as_ref()
                        .map(|expression| self.lower_expression(expression)),
                }
            }
            hir::StmtKind::ConstantDeclaration { .. } => return None,
            hir::StmtKind::Assign { target, value } => thir::StmtKind::Assign {
                target: self.lower_expression(target),
                value: self.lower_expression(value),
            },
            hir::StmtKind::Expression(expression) => thir::StmtKind::Expression {
                expression: self.lower_expression(expression),
            },
            hir::StmtKind::Pause(duration) => thir::StmtKind::Pause {
                duration: self.lower_expression(duration),
            },
            hir::StmtKind::If {
                condition,
                then_block,
                else_block,
            } => thir::StmtKind::If {
                condition: self.lower_expression(condition),
                then_block: self.lower_block(then_block),
                else_block: else_block.as_ref().map(|block| self.lower_block(block)),
            },
            hir::StmtKind::While { condition, body } => thir::StmtKind::While {
                condition: self.lower_expression(condition),
                body: self.lower_block(body),
            },
            hir::StmtKind::For {
                initializer,
                condition,
                step,
                body,
            } => thir::StmtKind::For {
                initializer: initializer
                    .as_deref()
                    .and_then(|statement| self.lower_statement(statement))
                    .map(Box::new),
                condition: condition
                    .as_ref()
                    .map(|expression| self.lower_expression(expression)),
                step: step
                    .as_deref()
                    .and_then(|statement| self.lower_statement(statement))
                    .map(Box::new),
                body: self.lower_block(body),
            },
        };

        Some(thir::Stmt {
            kind,
            source: thir::SourceInfo {
                span: statement.span,
            },
        })
    }

    pub(super) fn finish(self, block: thir::Block) -> thir::Body {
        thir::Body {
            parameter_locals: self.parameter_locals,
            locals: self.locals,
            expressions: self.expressions,
            block,
        }
    }
}
