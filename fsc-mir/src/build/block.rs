use super::{body::BodyBuilder, context::lower_source};
use crate::{
    Constant, Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use fsc_sema::thir;

enum LogicalOp {
    And,
    Or,
}

impl BodyBuilder<'_> {
    pub(super) fn lower_block(&mut self, block: &thir::Block) {
        for statement in &block.statements {
            if !self.cfg.has_current_block() {
                break;
            }
            self.lower_statement(statement);
        }
    }

    fn lower_statement(&mut self, statement: &thir::Stmt) {
        let source = lower_source(statement.source);
        match &statement.kind {
            thir::StmtKind::Return { value } => {
                if let Some(value) = value {
                    // thanks to the decoupling we can allow returning void calls in void
                    // function and handle them decoupled as effectful statement
                    if self.thir.expressions[*value].ty == fsc_sema::types::Ty::Void {
                        self.lower_expression_statement(*value);
                    } else {
                        self.lower_expression_into(*value, Place::return_place());
                    }
                }
                self.cfg.terminate(Terminator {
                    kind: TerminatorKind::Return,
                    source,
                });
            }
            thir::StmtKind::Break => {
                let Some(target) = self.scopes.break_target() else {
                    unreachable!("break outside a loop reached MIR lowering");
                };
                self.cfg.goto(target, source);
            }
            thir::StmtKind::LocalDeclaration { local, initializer } => {
                if let Some(initializer) = initializer {
                    self.lower_expression_into(*initializer, self.local_place(*local));
                }
            }
            thir::StmtKind::Assign { target, value } => {
                let target = self.lower_place(*target);
                self.lower_expression_into(*value, target);
            }
            thir::StmtKind::Expression { expression } => {
                self.lower_expression_statement(*expression);
            }
            thir::StmtKind::Pause { duration } => {
                let duration = self.lower_operand(*duration);
                self.cfg.push_statement(Statement {
                    kind: StatementKind::Pause { duration },
                    source,
                });
            }
            thir::StmtKind::If {
                condition,
                then_block,
                else_block,
            } => self.lower_if(
                *condition,
                then_block,
                else_block.as_ref(),
                statement.source,
            ),
            thir::StmtKind::While { condition, body } => {
                self.lower_while(*condition, body, statement.source);
            }
        }
    }

    fn lower_if(
        &mut self,
        condition: thir::ExprId,
        then_block: &thir::Block,
        else_block: Option<&thir::Block>,
        source: thir::SourceInfo,
    ) {
        //     if condition {
        //         then_block
        //     } else {
        //         else_block
        //     }
        let then_target = self.cfg.new_block();
        let join_target = self.cfg.new_block();
        let else_target = else_block.map_or(join_target, |_| self.cfg.new_block());

        //     current:
        //         branch condition, then, else
        self.lower_condition_to_targets(condition, then_target, else_target);

        //     then:
        //         ...
        //         goto join
        self.cfg.switch_to(then_target);
        self.lower_block(then_block);
        if self.cfg.has_current_block() {
            self.cfg.goto(join_target, lower_source(source));
        }

        //     else:
        //         ...
        //         goto join
        if let Some(else_block) = else_block {
            self.cfg.switch_to(else_target);
            self.lower_block(else_block);
            if self.cfg.has_current_block() {
                self.cfg.goto(join_target, lower_source(source));
            }
        }
        //     join:
        //         ...
        self.cfg.switch_to(join_target);
    }

    fn lower_while(
        &mut self,
        condition: thir::ExprId,
        body: &thir::Block,
        source: thir::SourceInfo,
    ) {
        //     while condition {
        //         body
        //     }
        let condition_target = self.cfg.new_block();
        let body_target = self.cfg.new_block();
        let exit_target = self.cfg.new_block();

        //     current:
        //         goto condition
        self.cfg.goto(condition_target, lower_source(source));

        //     condition:
        //         branch condition, body, exit
        self.cfg.switch_to(condition_target);
        self.lower_condition_to_targets(condition, body_target, exit_target);

        //     body:
        //         ...
        //         goto condition
        self.cfg.switch_to(body_target);
        self.scopes.enter_loop(exit_target);
        self.lower_block(body);
        self.scopes.exit_loop(exit_target);
        if self.cfg.has_current_block() {
            self.cfg.goto(condition_target, lower_source(source));
        }

        //     exit:
        //         ...
        self.cfg.switch_to(exit_target);
    }

    fn lower_condition_to_targets(
        &mut self,
        expression: thir::ExprId,
        true_target: crate::BasicBlockId,
        false_target: crate::BasicBlockId,
    ) {
        // invariant: expects that the expression was type checked and has the bool type
        let source = self.thir.expressions[expression].source;
        let logical = match &self.thir.expressions[expression].kind {
            thir::ExprKind::Binary {
                op: thir::BinaryOp::LogicalAnd,
                lhs,
                rhs,
            } => Some((LogicalOp::And, *lhs, *rhs)),
            thir::ExprKind::Binary {
                op: thir::BinaryOp::LogicalOr,
                lhs,
                rhs,
            } => Some((LogicalOp::Or, *lhs, *rhs)),
            _ => None,
        };

        match logical {
            Some((LogicalOp::And, lhs, rhs)) => {
                // lhs && rhs
                let rhs_target = self.cfg.new_block();
                //     current:
                //         branch lhs, rhs_block, false_target
                self.lower_condition_to_targets(lhs, rhs_target, false_target);
                //     rhs_block:
                //         branch rhs, true_target, false_target
                self.cfg.switch_to(rhs_target);
                self.lower_condition_to_targets(rhs, true_target, false_target);
            }
            Some((LogicalOp::Or, lhs, rhs)) => {
                // lhs || rhs
                let rhs_target = self.cfg.new_block();
                //     current:
                //         branch lhs, true_target, rhs_block
                self.lower_condition_to_targets(lhs, true_target, rhs_target);
                //     rhs_block:
                //         branch rhs, true_target, false_target
                self.cfg.switch_to(rhs_target);
                self.lower_condition_to_targets(rhs, true_target, false_target);
            }
            None => {
                //     current:
                //         branch value, true_target, false_target
                let condition = self.lower_operand(expression);
                self.cfg.terminate(Terminator {
                    kind: TerminatorKind::Branch {
                        condition,
                        then_target: true_target,
                        else_target: false_target,
                    },
                    source: lower_source(source),
                });
            }
        }
    }

    pub(super) fn lower_condition_to_operand(&mut self, expression: thir::ExprId) -> Operand {
        //     let result = lhs && rhs;
        //     let result = lhs || rhs;
        let expression_data = self.thir.expressions[expression].clone();
        let destination =
            Place::from_local(self.allocate_temporary(&expression_data.ty, expression_data.source));

        self.lower_condition_into(expression, destination.clone());
        Operand::Place(destination)
    }

    pub(super) fn lower_condition_into(&mut self, expression: thir::ExprId, destination: Place) {
        //     let result = lhs && rhs;
        //     let result = lhs || rhs;
        let source = self.thir.expressions[expression].source;
        let true_target = self.cfg.new_block();
        let false_target = self.cfg.new_block();
        let join_target = self.cfg.new_block();

        //     current:
        //         lower_condition expression, true_block, false_block
        self.lower_condition_to_targets(expression, true_target, false_target);

        //     true_block:
        //         destination = true
        //         goto join
        self.cfg.switch_to(true_target);
        self.assign_bool(destination.clone(), true, source);
        self.cfg.goto(join_target, lower_source(source));

        //     false_block:
        //         destination = false
        //         goto join
        self.cfg.switch_to(false_target);
        self.assign_bool(destination, false, source);
        self.cfg.goto(join_target, lower_source(source));

        //     join:
        //         use destination
        self.cfg.switch_to(join_target);
    }

    fn assign_bool(&mut self, destination: Place, value: bool, source: thir::SourceInfo) {
        self.cfg.push_statement(Statement {
            kind: StatementKind::Assign {
                target: destination,
                value: Rvalue::Use(Operand::Constant(Constant {
                    value: crate::ConstValue::Bool(value),
                    ty: fsc_sema::types::Ty::Bool,
                })),
            },
            source: lower_source(source),
        });
    }
}
