use super::{block::BlockLowerer, operand::ScalarKind};
use crate::CodegenResult;
use fsb_ir::{BranchKind, BranchMode};
use fsc_mir::{BasicBlockId, BinaryOp, ConstValue, Operand, Rvalue, Ty, UnaryOp};

#[derive(Clone, Copy)]
enum BranchTest {
    Zero,
    NonZero,
}

#[derive(Clone, Copy)]
enum BranchOperandKind {
    Bool,
    IntegerOrBool,
}

struct SelectedBranch<'operand> {
    operand: &'operand Operand,
    test: BranchTest,
    operand_kind: BranchOperandKind,
}

impl BlockLowerer<'_, '_> {
    pub(super) fn lower_branch(
        &mut self,
        condition: &Operand,
        then_target: BasicBlockId,
        else_target: BasicBlockId,
    ) -> CodegenResult<fsb_ir::Terminator> {
        let function = self.function;
        let stackified = match condition {
            Operand::Place(place) => function.stackified_rvalue(place),
            Operand::Constant(_) => None,
        };
        let selected = select_branch_condition(condition, stackified);

        let operand = self.lower_scalar_operand(selected.operand)?;
        debug_assert!(selected.operand_kind.accepts(operand));

        Ok(fsb_ir::Terminator::Branch {
            kind: selected.test.branch_kind(),
            taken: function.block(then_target),
            not_taken: function.block(else_target),
        })
    }
}

impl BranchTest {
    const fn branch_kind(self) -> BranchKind {
        match self {
            Self::Zero => BranchKind::IfZero(BranchMode::Normal),
            Self::NonZero => BranchKind::IfNonZero(BranchMode::Normal),
        }
    }
}

impl BranchOperandKind {
    const fn accepts(self, kind: ScalarKind) -> bool {
        match self {
            Self::Bool => matches!(kind, ScalarKind::Bool),
            Self::IntegerOrBool => matches!(kind, ScalarKind::Integer | ScalarKind::Bool),
        }
    }
}

fn select_branch_condition<'operand>(
    condition: &'operand Operand,
    stackified: Option<&'operand Rvalue>,
) -> SelectedBranch<'operand> {
    let Some(value) = stackified else {
        return materialized_bool_branch(condition);
    };

    match value {
        Rvalue::Unary {
            op: UnaryOp::Not,
            operand,
        } => SelectedBranch {
            operand,
            test: BranchTest::Zero,
            operand_kind: BranchOperandKind::Bool,
        },
        Rvalue::Binary { op, lhs, rhs } if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) => {
            let Some(operand) = zero_comparison_operand(lhs, rhs) else {
                return materialized_bool_branch(condition);
            };
            let test = if *op == BinaryOp::Equal {
                BranchTest::Zero
            } else {
                BranchTest::NonZero
            };
            SelectedBranch {
                operand,
                test,
                operand_kind: BranchOperandKind::IntegerOrBool,
            }
        }
        _ => materialized_bool_branch(condition),
    }
}

const fn materialized_bool_branch(condition: &Operand) -> SelectedBranch<'_> {
    SelectedBranch {
        operand: condition,
        test: BranchTest::NonZero,
        operand_kind: BranchOperandKind::Bool,
    }
}

fn zero_comparison_operand<'operand>(
    lhs: &'operand Operand,
    rhs: &'operand Operand,
) -> Option<&'operand Operand> {
    if is_zero(lhs) {
        Some(rhs)
    } else if is_zero(rhs) {
        Some(lhs)
    } else {
        None
    }
}

fn is_zero(operand: &Operand) -> bool {
    let Operand::Constant(constant) = operand else {
        return false;
    };

    matches!(
        (&constant.value, &constant.ty),
        (ConstValue::Int(0), Ty::Int) | (ConstValue::Bool(false), Ty::Bool)
    )
}
