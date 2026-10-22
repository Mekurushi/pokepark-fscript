use super::error::TypeckError;
use super::results::{ExprType, ExprTypeResult, TypeckResults};
use super::rules::{self, Compatibility, TypeContext};
use crate::hir;
use crate::types::{self, Ty};
use fsc_diagnostics::{Diagnostic, Diagnostics, ErrorGuaranteed, Span};

pub(crate) fn check(script: &hir::Script, diagnostics: &mut Diagnostics) -> Option<TypeckResults> {
    let mut checker = TypeChecker::new(script, diagnostics);
    checker.check_script();
    if checker.failed {
        None
    } else {
        Some(checker.results)
    }
}

struct TypeChecker<'hir, 'diagnostics> {
    script: &'hir hir::Script,
    diagnostics: &'diagnostics mut Diagnostics,
    failed: bool,
    results: TypeckResults,
}

impl<'hir, 'diagnostics> TypeChecker<'hir, 'diagnostics> {
    fn new(script: &'hir hir::Script, diagnostics: &'diagnostics mut Diagnostics) -> Self {
        Self {
            script,
            diagnostics,
            failed: false,
            results: TypeckResults::default(),
        }
    }

    fn check_script(&mut self) {
        for (_, item) in &self.script.items {
            self.check_item(item);
        }
    }

    fn check_item(&mut self, item: &hir::Item) {
        match &item.kind {
            hir::ItemKind::Function(function) => self.check_function(function),
            hir::ItemKind::Constant(constant) => self.check_constant(constant),
            hir::ItemKind::Config(config) => self.check_config(config),
        }
    }

    fn check_function(&mut self, function: &hir::Function) {
        self.check_signature(&function.signature);
        let hir::FunctionKind::Defined { body, .. } = &function.kind else {
            return;
        };

        for (_, local) in &body.locals {
            let context = match local.kind {
                hir::LocalKind::Parameter { .. } => continue,
                hir::LocalKind::Variable => TypeContext::Local,
                hir::LocalKind::Constant => TypeContext::Constant,
            };
            self.check_declared_type(&local.ty, local.type_span, context);
        }

        self.check_block(body, &body.block, &function.signature.return_type);
    }

    fn check_signature(&mut self, signature: &hir::FunctionSignature) {
        for parameter in &signature.parameters {
            self.check_declared_type(&parameter.ty, parameter.type_span, TypeContext::Parameter);
        }
        self.check_declared_type(
            &signature.return_type,
            signature.return_type_span,
            TypeContext::Return,
        );
    }

    fn check_constant(&mut self, constant: &hir::Constant) {
        self.check_declared_type(&constant.ty, constant.type_span, TypeContext::Constant);
        let actual = self.check_value_expression(None, &constant.initializer);
        self.expect_type(actual, &constant.ty, constant.initializer.span);
    }

    fn check_config(&mut self, config: &hir::Config) {
        if !rules::type_allowed(&config.ty, TypeContext::Config) {
            self.emit_error(TypeckError::UnsupportedConfigType {
                ty: config.ty.clone(),
                span: config.type_span,
            });
        }
    }

    fn check_declared_type(&mut self, ty: &Ty, span: Span, context: TypeContext) {
        if !rules::type_allowed(ty, context) {
            self.emit_error(TypeckError::InvalidDeclaredType {
                ty: ty.clone(),
                span,
                declaration: context.description(),
            });
        }
    }

    fn check_block(&mut self, body: &hir::Body, block: &hir::Block, return_type: &Ty) {
        for statement in &block.statements {
            self.check_statement(body, statement, return_type);
        }
    }

    fn check_statement(&mut self, body: &hir::Body, statement: &hir::Stmt, return_type: &Ty) {
        match &statement.kind {
            hir::StmtKind::Return(value) => {
                self.check_return(body, value.as_ref(), return_type, statement.span);
            }
            hir::StmtKind::Break => {}
            hir::StmtKind::VariableDeclaration { local, initializer } => {
                if let Some(initializer) = initializer {
                    let actual = self.check_value_expression(Some(body), initializer);
                    self.expect_type(actual, &body.locals[*local].ty, initializer.span);
                }
            }
            hir::StmtKind::ConstantDeclaration { local, initializer } => {
                let actual = self.check_value_expression(Some(body), initializer);
                self.expect_type(actual, &body.locals[*local].ty, initializer.span);
            }
            hir::StmtKind::Assign { target, value } => {
                let target_type = self.check_assignment_target(body, target);
                let value_type = self.check_value_expression(Some(body), value);
                if let Ok(expected) = target_type {
                    self.expect_type(value_type, &expected, value.span);
                }
            }
            hir::StmtKind::Expression(expression) => {
                let _ = self.check_value_expression(Some(body), expression);
            }
            hir::StmtKind::Pause(duration) => {
                let actual = self.check_value_expression(Some(body), duration);
                self.expect_type(actual, &Ty::Int, duration.span);
            }
            hir::StmtKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.check_condition(body, condition);
                self.check_block(body, then_block, return_type);
                if let Some(else_block) = else_block {
                    self.check_block(body, else_block, return_type);
                }
            }
            hir::StmtKind::While {
                condition,
                body: loop_body,
            } => {
                self.check_condition(body, condition);
                self.check_block(body, loop_body, return_type);
            }
        }
    }

    fn check_return(
        &mut self,
        body: &hir::Body,
        value: Option<&hir::Expr>,
        return_type: &Ty,
        span: Span,
    ) {
        match value {
            Some(value) => {
                let actual = self.check_value_expression(Some(body), value);
                self.expect_type(actual, return_type, value.span);
            }
            None => self.expect_type(Ok(Ty::Void), return_type, span),
        }
    }

    fn check_condition(&mut self, body: &hir::Body, condition: &hir::Expr) {
        let actual = self.check_value_expression(Some(body), condition);
        self.expect_type(actual, &Ty::Bool, condition.span);
    }

    fn check_assignment_target(
        &mut self,
        body: &hir::Body,
        target: &hir::Expr,
    ) -> Result<Ty, ErrorGuaranteed> {
        let expression_type = self.check_expression(Some(body), target);
        let writable = self.check_writable_place(body, target);

        match (expression_type, writable) {
            (_, Err(error)) | (Err(error), Ok(())) => Err(error),
            (Ok(ExprType::Value(ty)), Ok(())) => Ok(ty),
            (Ok(ExprType::Function(_)), Ok(())) => {
                Err(self.emit_error(TypeckError::ExpectedValue { span: target.span }))
            }
        }
    }

    fn check_writable_place(
        &mut self,
        body: &hir::Body,
        expression: &hir::Expr,
    ) -> Result<(), ErrorGuaranteed> {
        match &expression.kind {
            hir::ExprKind::Error(error) => Err(*error),
            hir::ExprKind::Local(local) => match body.locals[*local].kind {
                hir::LocalKind::Parameter { .. } | hir::LocalKind::Variable => Ok(()),
                hir::LocalKind::Constant => {
                    Err(self.emit_error(TypeckError::ImmutableLocalConstant {
                        span: expression.span,
                    }))
                }
            },
            hir::ExprKind::Member { base, .. } | hir::ExprKind::Index { base, .. } => {
                self.check_writable_place(body, base)
            }
            hir::ExprKind::ItemRef(_) => Err(self.emit_error(TypeckError::ImmutableItem {
                span: expression.span,
            })),
            _ => Err(self.emit_error(TypeckError::NotAssignable {
                span: expression.span,
            })),
        }
    }

    fn check_value_expression(
        &mut self,
        body: Option<&hir::Body>,
        expression: &hir::Expr,
    ) -> Result<Ty, ErrorGuaranteed> {
        match self.check_expression(body, expression) {
            Ok(ExprType::Value(ty)) => Ok(ty),
            Ok(ExprType::Function(_)) => Err(self.emit_error(TypeckError::ExpectedValue {
                span: expression.span,
            })),
            Err(error) => Err(error),
        }
    }

    fn check_expression(
        &mut self,
        body: Option<&hir::Body>,
        expression: &hir::Expr,
    ) -> ExprTypeResult {
        let result = self.check_expression_kind(body, expression);
        if let Ok(expression_type) = &result {
            self.results
                .record_expression(expression.id, expression_type.clone());
        }
        result
    }

    fn check_expression_kind(
        &mut self,
        body: Option<&hir::Body>,
        expression: &hir::Expr,
    ) -> ExprTypeResult {
        match &expression.kind {
            hir::ExprKind::Error(error) => Err(*error),
            hir::ExprKind::Int(_) => Ok(ExprType::Value(Ty::Int)),
            hir::ExprKind::Float(_) => Ok(ExprType::Value(Ty::Float)),
            hir::ExprKind::Bool(_) => Ok(ExprType::Value(Ty::Bool)),
            hir::ExprKind::String(_) => Ok(ExprType::Value(Ty::Str)),
            hir::ExprKind::Array { elements } => self
                .check_array_literal(body, elements, expression.span)
                .map(ExprType::Value),
            hir::ExprKind::Index { base, index } => {
                let base_type = self.check_value_expression(body, base);
                let index_type = self.check_value_expression(body, index);
                self.expect_type(index_type, &Ty::Int, index.span);

                match base_type? {
                    Ty::Array { element, .. } => Ok(ExprType::Value(*element)),
                    actual => Err(self.emit_error(TypeckError::InvalidIndexBase {
                        actual,
                        span: base.span,
                    })),
                }
            }
            hir::ExprKind::Local(local) => {
                let Some(body) = body else {
                    return Err(self.emit_error(TypeckError::ExpectedValue {
                        span: expression.span,
                    }));
                };
                let ty = match &body.locals[*local].ty {
                    Ty::Reference(referent) => (**referent).clone(),
                    ty => ty.clone(),
                };
                Ok(ExprType::Value(ty))
            }
            hir::ExprKind::ItemRef(item) => match &self.script.items[*item].kind {
                hir::ItemKind::Function(_) => Ok(ExprType::Function(*item)),
                hir::ItemKind::Constant(constant) => Ok(ExprType::Value(constant.ty.clone())),
                hir::ItemKind::Config(config) => Ok(ExprType::Value(config.ty.clone())),
            },
            hir::ExprKind::Member {
                base,
                member,
                member_span,
            } => {
                let base_type = self.check_value_expression(body, base)?;
                self.check_member(expression.id, &base_type, member, *member_span)
                    .map(ExprType::Value)
            }
            hir::ExprKind::Unary { op, expr } => {
                if *op == hir::UnaryOp::AddressOf {
                    let Some(body) = body else {
                        return Err(self.emit_error(TypeckError::NotAssignable { span: expr.span }));
                    };
                    self.check_writable_place(body, expr)?;
                    let referent = self.check_value_expression(Some(body), expr)?;
                    return Ok(ExprType::Value(Ty::Reference(Box::new(referent))));
                }
                let operand = self.check_value_expression(body, expr)?;
                self.check_unary(*op, &operand, expression.span)
                    .map(ExprType::Value)
            }
            hir::ExprKind::Binary { op, lhs, rhs } => {
                let lhs_type = self.check_value_expression(body, lhs);
                let rhs_type = self.check_value_expression(body, rhs);
                match (lhs_type, rhs_type) {
                    (Ok(lhs_type), Ok(rhs_type)) => self
                        .check_binary(*op, &lhs_type, &rhs_type, expression.span)
                        .map(ExprType::Value),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            hir::ExprKind::Call { callee, arguments } => self
                .check_call(body, callee, arguments, expression.span)
                .map(ExprType::Value),
            hir::ExprKind::RawSyscall { arguments } => {
                self.check_raw_syscall(body, arguments, expression.span);
                Ok(ExprType::Value(Ty::Int))
            }
        }
    }

    fn check_array_literal(
        &mut self,
        body: Option<&hir::Body>,
        elements: &[hir::Expr],
        span: Span,
    ) -> Result<Ty, ErrorGuaranteed> {
        let Some((first, remaining)) = elements.split_first() else {
            return Err(self.emit_error(TypeckError::CannotInferArrayElementType { span }));
        };

        let element_type = self.check_value_expression(body, first)?;
        for element in remaining {
            let actual = self.check_value_expression(body, element);
            self.expect_type(actual, &element_type, element.span);
        }

        Ok(Ty::Array {
            element: Box::new(element_type),
            length: elements.len(),
        })
    }

    fn check_call(
        &mut self,
        body: Option<&hir::Body>,
        callee: &hir::Expr,
        arguments: &[hir::Expr],
        call_span: Span,
    ) -> Result<Ty, ErrorGuaranteed> {
        let callee_type = self.check_expression(body, callee);
        let function_id = match callee_type {
            Ok(ExprType::Function(function)) => Ok(function),
            Ok(ExprType::Value(_)) => {
                Err(self.emit_error(TypeckError::NotCallable { span: callee.span }))
            }
            Err(error) => Err(error),
        };

        let argument_types = arguments
            .iter()
            .map(|argument| self.check_value_expression(body, argument))
            .collect::<Vec<_>>();

        let function_id = function_id?;
        let hir::ItemKind::Function(function) = &self.script.items[function_id].kind else {
            return Err(self.emit_error(TypeckError::NotCallable { span: call_span }));
        };

        if arguments.len() != function.signature.parameters.len() {
            self.emit_error(TypeckError::ArgumentCount {
                expected: function.signature.parameters.len(),
                actual: arguments.len(),
                span: call_span,
            });
        }
        for ((argument, actual), parameter) in arguments
            .iter()
            .zip(argument_types)
            .zip(&function.signature.parameters)
        {
            self.expect_type(actual, &parameter.ty, argument.span);
        }
        Ok(function.signature.return_type.clone())
    }

    fn check_raw_syscall(&mut self, body: Option<&hir::Body>, arguments: &[hir::Expr], span: Span) {
        const SELECTOR_COUNT: usize = 2;

        if arguments.len() < SELECTOR_COUNT {
            self.emit_error(TypeckError::MissingRawSyscallSelectors {
                actual: arguments.len(),
                span,
            });
        }

        for (index, argument) in arguments.iter().enumerate() {
            let actual = self.check_value_expression(body, argument);
            if index < SELECTOR_COUNT {
                self.expect_type(actual, &Ty::Int, argument.span);
            } else if let Ok(actual) = actual
                && !rules::supports_raw_syscall_argument(&actual)
            {
                self.emit_error(TypeckError::UnsupportedRawSyscallArgument {
                    actual,
                    span: argument.span,
                });
            }
        }
    }

    fn check_member(
        &mut self,
        expression: hir::ExprId,
        base: &Ty,
        member: &str,
        member_span: Span,
    ) -> Result<Ty, ErrorGuaranteed> {
        let field = match types::aggregate_definition(base) {
            Some(aggregate) => aggregate.field(member).ok_or_else(|| {
                self.emit_error(TypeckError::UnknownMember {
                    member: member.to_owned(),
                    span: member_span,
                })
            }),
            None => Err(self.emit_error(TypeckError::InvalidMemberBase {
                actual: base.clone(),
                span: member_span,
            })),
        };
        if let Ok(field) = &field {
            self.results.record_field(expression, field.index());
        }
        field.map(|field| field.ty().clone())
    }

    fn check_unary(
        &mut self,
        op: hir::UnaryOp,
        operand: &Ty,
        span: Span,
    ) -> Result<Ty, ErrorGuaranteed> {
        let result = match op {
            hir::UnaryOp::Not if *operand == Ty::Bool => Some(Ty::Bool),
            hir::UnaryOp::Neg if rules::supports_negation(operand) => Some(operand.clone()),
            hir::UnaryOp::AddressOf => unreachable!("address-of is checked as a place operation"),
            _ => None,
        };
        result.ok_or_else(|| {
            self.emit_error(TypeckError::InvalidUnaryOperand {
                op,
                actual: operand.clone(),
                span,
            })
        })
    }

    fn check_binary(
        &mut self,
        op: hir::BinaryOp,
        lhs: &Ty,
        rhs: &Ty,
        span: Span,
    ) -> Result<Ty, ErrorGuaranteed> {
        use hir::BinaryOp::{
            Add, And, Div, Eq, Greater, GreaterEq, Less, LessEq, Mul, NotEq, Or, Sub,
        };

        let result = match op {
            Add | Sub | Mul | Div if lhs == rhs && rules::supports_arithmetic(lhs) => {
                Some(lhs.clone())
            }
            Eq | NotEq if lhs == rhs && rules::supports_equality(lhs) => Some(Ty::Bool),
            Less | Greater | LessEq | GreaterEq if lhs == rhs && rules::supports_ordering(lhs) => {
                Some(Ty::Bool)
            }
            And | Or if *lhs == Ty::Bool && *rhs == Ty::Bool => Some(Ty::Bool),
            _ => None,
        };
        result.ok_or_else(|| {
            self.emit_error(TypeckError::InvalidBinaryOperands {
                op,
                lhs: lhs.clone(),
                rhs: rhs.clone(),
                span,
            })
        })
    }

    fn expect_type(&mut self, actual: Result<Ty, ErrorGuaranteed>, expected: &Ty, span: Span) {
        if let Ok(actual) = actual
            && rules::compatibility(&actual, expected) == Compatibility::Incompatible
        {
            self.emit_error(TypeckError::TypeMismatch {
                expected: expected.clone(),
                actual,
                span,
            });
        }
    }

    fn emit_error(&mut self, error: TypeckError) -> ErrorGuaranteed {
        self.failed = true;
        self.diagnostics.emit_error(Diagnostic::from(error))
    }
}
