use crate::config::{BoundConfigs, ConfigValue};
use crate::const_eval::{ConstValue, EvaluatedConstants};
use crate::hir;
use crate::thir;
use crate::typeck::{ExprType, TypeckResults};
use crate::types::Ty;
use cranelift_entity::{PrimaryMap, SecondaryMap};

pub(crate) fn lower(
    hir: &hir::Script,
    typeck: &TypeckResults,
    configs: &BoundConfigs,
    constants: &EvaluatedConstants,
) -> thir::Script {
    LoweringContext::new(hir, typeck, configs, constants).lower_script()
}

struct FunctionMap {
    items: PrimaryMap<thir::FunctionId, hir::ItemId>,
    functions: SecondaryMap<hir::ItemId, Option<thir::FunctionId>>,
}

impl FunctionMap {
    fn collect(script: &hir::Script) -> Self {
        let mut items = PrimaryMap::new();
        let mut functions = SecondaryMap::new();

        for (item, declaration) in &script.items {
            if matches!(declaration.kind, hir::ItemKind::Function(_)) {
                let function = items.push(item);
                functions.insert(item, Some(function));
            }
        }

        Self { items, functions }
    }

    fn function(&self, item: hir::ItemId) -> Option<thir::FunctionId> {
        self.functions.get(item).and_then(|function| *function)
    }
}

struct LoweringContext<'analysis> {
    hir: &'analysis hir::Script,
    typeck: &'analysis TypeckResults,
    configs: &'analysis BoundConfigs,
    constants: &'analysis EvaluatedConstants,
    functions: FunctionMap,
}

impl<'analysis> LoweringContext<'analysis> {
    fn new(
        hir: &'analysis hir::Script,
        typeck: &'analysis TypeckResults,
        configs: &'analysis BoundConfigs,
        constants: &'analysis EvaluatedConstants,
    ) -> Self {
        Self {
            hir,
            typeck,
            configs,
            constants,
            functions: FunctionMap::collect(hir),
        }
    }

    fn lower_script(&self) -> thir::Script {
        let mut functions = PrimaryMap::new();
        for (expected, item) in &self.functions.items {
            let actual = functions.push(self.lower_function(*item));
            debug_assert_eq!(actual, expected);
        }
        thir::Script { functions }
    }

    fn lower_function(&self, item: hir::ItemId) -> thir::Function {
        let declaration = &self.hir.items[item];
        let hir::ItemKind::Function(function) = &declaration.kind else {
            unreachable!("non-function item reached THIR function lowering");
        };

        let signature = thir::FunctionSignature {
            parameters: function
                .signature
                .parameters
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            return_type: function.signature.return_type.clone(),
        };
        let kind = match &function.kind {
            hir::FunctionKind::Defined { exported, body } => thir::FunctionKind::Defined {
                linkage: if *exported {
                    thir::Linkage::Exported
                } else {
                    thir::Linkage::Internal
                },
                body: self.lower_body(item, body),
            },
            hir::FunctionKind::External => thir::FunctionKind::External,
        };

        thir::Function {
            name: declaration.name.clone(),
            signature,
            kind,
            source: thir::SourceInfo {
                span: declaration.name_span,
            },
        }
    }

    fn lower_body(&self, owner: hir::ItemId, body: &hir::Body) -> thir::Body {
        let mut lowerer = BodyLowerer::new(self, owner, body);
        let block = lowerer.lower_block(&body.block);
        lowerer.finish(block)
    }
}

struct BodyLowerer<'context, 'analysis> {
    context: &'context LoweringContext<'analysis>,
    owner: hir::ItemId,
    body: &'analysis hir::Body,
    locals: PrimaryMap<thir::LocalId, thir::Local>,
    local_mapping: SecondaryMap<hir::LocalId, Option<thir::LocalId>>,
    parameter_locals: Box<[thir::LocalId]>,
    expressions: PrimaryMap<thir::ExprId, thir::Expr>,
}

impl<'context, 'analysis> BodyLowerer<'context, 'analysis> {
    fn new(
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

    fn mapped_local(
        local_mapping: &SecondaryMap<hir::LocalId, Option<thir::LocalId>>,
        local: hir::LocalId,
    ) -> thir::LocalId {
        let Some(local) = local_mapping.get(local).and_then(|local| *local) else {
            unreachable!("non-runtime local reached THIR local lowering");
        };
        local
    }

    fn lower_expression(&mut self, expression: &hir::Expr) -> thir::ExprId {
        let kind = match &expression.kind {
            hir::ExprKind::Error(_) => {
                unreachable!("error expression reached THIR lowering")
            }
            hir::ExprKind::Int(value) => {
                thir::ExprKind::Constant(thir::ConstValue::Int(*value))
            }
            hir::ExprKind::Float(value) => {
                thir::ExprKind::Constant(thir::ConstValue::Float(*value))
            }
            hir::ExprKind::Bool(value) => {
                thir::ExprKind::Constant(thir::ConstValue::Bool(*value))
            }
            hir::ExprKind::String(value) => {
                thir::ExprKind::Constant(thir::ConstValue::String(value.clone()))
            }
            hir::ExprKind::Local(local) => self.lower_local(*local),
            hir::ExprKind::ItemRef(item) => self.lower_item(*item),
            hir::ExprKind::Member { base, .. } => {
                let base = self.lower_expression(base);
                let Some(field) = self.context.typeck.field_index(expression.id) else {
                    unreachable!("unresolved field reached THIR lowering");
                };
                thir::ExprKind::Field { base, field }
            }
            hir::ExprKind::Unary { op, expr } => {
                let operand = self.lower_expression(expr);
                thir::ExprKind::Unary {
                    op: lower_unary_operator(*op),
                    operand,
                }
            }
            hir::ExprKind::Binary { op, lhs, rhs } => {
                let lhs = self.lower_expression(lhs);
                let rhs = self.lower_expression(rhs);
                thir::ExprKind::Binary {
                    op: lower_binary_operator(*op),
                    lhs,
                    rhs,
                }
            }
            hir::ExprKind::Call { callee, arguments } => {
                let Some(ExprType::Function(callee)) =
                    self.context.typeck.expression_type(callee.id)
                else {
                    unreachable!("unresolved callee reached THIR lowering");
                };
                let Some(callee) = self.context.functions.function(*callee) else {
                    unreachable!("non-function callee reached THIR lowering");
                };
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect();
                thir::ExprKind::Call { callee, arguments }
            }
            hir::ExprKind::RawSyscall { arguments } => {
                let [page, function, arguments @ ..] = arguments.as_slice() else {
                    unreachable!("raw syscall without selectors reached THIR lowering");
                };
                let page = self.lower_expression(page);
                let function = self.lower_expression(function);
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect();
                thir::ExprKind::RawSyscall {
                    page,
                    function,
                    arguments,
                }
            }
        };
        let ty = self.expression_type(expression);

        self.expressions.push(thir::Expr {
            kind,
            ty,
            source: thir::SourceInfo {
                span: expression.span,
            },
        })
    }

    fn lower_local(&self, local: hir::LocalId) -> thir::ExprKind {
        match self.body.locals[local].kind {
            hir::LocalKind::Parameter { .. } | hir::LocalKind::Variable => {
                thir::ExprKind::Local(Self::mapped_local(&self.local_mapping, local))
            }
            hir::LocalKind::Constant => {
                let Some(value) = self.context.constants.local(self.owner, local) else {
                    unreachable!("unevaluated local constant reached THIR lowering");
                };
                thir::ExprKind::Constant(lower_constant(value))
            }
        }
    }

    fn lower_item(&self, item: hir::ItemId) -> thir::ExprKind {
        match &self.context.hir.items[item].kind {
            hir::ItemKind::Constant(_) => {
                let Some(value) = self.context.constants.item(item) else {
                    unreachable!("unevaluated constant reached THIR lowering");
                };
                thir::ExprKind::Constant(lower_constant(value))
            }
            hir::ItemKind::Config(_) => {
                let Some(value) = self.context.configs.value(item) else {
                    unreachable!("unbound config reached THIR lowering");
                };
                thir::ExprKind::Constant(lower_config(value))
            }
            hir::ItemKind::Function(_) => {
                unreachable!("function item reached THIR value lowering")
            }
        }
    }

    fn expression_type(&self, expression: &hir::Expr) -> Ty {
        let Some(ExprType::Value(ty)) = self.context.typeck.expression_type(expression.id) else {
            unreachable!("untyped value expression reached THIR lowering");
        };
        ty.clone()
    }

    fn lower_block(&mut self, block: &hir::Block) -> thir::Block {
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
        };

        Some(thir::Stmt {
            kind,
            source: thir::SourceInfo {
                span: statement.span,
            },
        })
    }

    fn finish(self, block: thir::Block) -> thir::Body {
        thir::Body {
            parameter_locals: self.parameter_locals,
            locals: self.locals,
            expressions: self.expressions,
            block,
        }
    }
}

fn lower_constant(value: &ConstValue) -> thir::ConstValue {
    match value {
        ConstValue::Int(value) => thir::ConstValue::Int(*value),
        ConstValue::Float(value) => thir::ConstValue::Float(*value),
        ConstValue::Bool(value) => thir::ConstValue::Bool(*value),
        ConstValue::String(value) => thir::ConstValue::String(value.clone()),
    }
}

fn lower_config(value: &ConfigValue) -> thir::ConstValue {
    match value {
        ConfigValue::Int(value) => thir::ConstValue::Int(*value),
        ConfigValue::Float(value) => thir::ConstValue::Float(*value),
        ConfigValue::Bool(value) => thir::ConstValue::Bool(*value),
        ConfigValue::String(value) => thir::ConstValue::String(value.clone()),
    }
}

const fn lower_unary_operator(operator: hir::UnaryOp) -> thir::UnaryOp {
    match operator {
        hir::UnaryOp::Not => thir::UnaryOp::Not,
        hir::UnaryOp::Neg => thir::UnaryOp::Negate,
    }
}

const fn lower_binary_operator(operator: hir::BinaryOp) -> thir::BinaryOp {
    match operator {
        hir::BinaryOp::Add => thir::BinaryOp::Add,
        hir::BinaryOp::Sub => thir::BinaryOp::Subtract,
        hir::BinaryOp::Mul => thir::BinaryOp::Multiply,
        hir::BinaryOp::Div => thir::BinaryOp::Divide,
        hir::BinaryOp::Eq => thir::BinaryOp::Equal,
        hir::BinaryOp::NotEq => thir::BinaryOp::NotEqual,
        hir::BinaryOp::Less => thir::BinaryOp::Less,
        hir::BinaryOp::Greater => thir::BinaryOp::Greater,
        hir::BinaryOp::LessEq => thir::BinaryOp::LessEqual,
        hir::BinaryOp::GreaterEq => thir::BinaryOp::GreaterEqual,
        hir::BinaryOp::And => thir::BinaryOp::LogicalAnd,
        hir::BinaryOp::Or => thir::BinaryOp::LogicalOr,
    }
}
