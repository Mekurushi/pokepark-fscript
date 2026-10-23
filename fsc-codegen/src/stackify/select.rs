use super::{
    candidate::Candidates,
    location::{ConsumerLocation, StatementLocation},
    plan::{StackificationPlan, StackifiedValue},
};
use fsc_mir::{
    Body, CallArgument, LocalId, Operand, Place, PlaceBase, Rvalue, StatementKind, TerminatorKind,
};

pub(super) fn select(body: &Body, candidates: &Candidates) -> StackificationPlan {
    Selector {
        body,
        candidates,
        plan: StackificationPlan::new(body),
    }
    .select()
}

struct Selector<'body> {
    body: &'body Body,
    candidates: &'body Candidates,
    plan: StackificationPlan,
}

impl Selector<'_> {
    fn select(mut self) -> StackificationPlan {
        for (block_id, block) in &self.body.blocks {
            for (statement, data) in block.statements.iter().enumerate() {
                let consumer = ConsumerLocation::Statement(StatementLocation {
                    block: block_id,
                    statement,
                });
                let mut cursor = statement;

                match &data.kind {
                    StatementKind::Assign { value, .. } => {
                        self.select_rvalue(value, consumer, &mut cursor);
                    }
                    StatementKind::Pause { duration } => {
                        self.select_operand(duration, consumer, &mut cursor);
                    }
                    StatementKind::Call { arguments, .. }
                    | StatementKind::RawSyscall { arguments, .. } => {
                        self.select_call_arguments(arguments, consumer, &mut cursor);
                    }
                }
            }

            if let TerminatorKind::Branch { condition, .. } = &block.terminator.kind {
                let consumer = ConsumerLocation::Terminator(block_id);
                let mut cursor = block.statements.len();
                self.select_operand(condition, consumer, &mut cursor);
            }
        }

        self.plan
    }

    fn select_call_arguments(
        &mut self,
        arguments: &[CallArgument],
        consumer: ConsumerLocation,
        cursor: &mut usize,
    ) {
        for argument in arguments.iter().rev() {
            let CallArgument::Value(operand) = argument else {
                break;
            };
            self.select_operand(operand, consumer, cursor);
        }
    }

    fn select_rvalue(&mut self, value: &Rvalue, consumer: ConsumerLocation, cursor: &mut usize) {
        match value {
            Rvalue::Use(operand) | Rvalue::Unary { operand, .. } => {
                self.select_operand(operand, consumer, cursor);
            }
            Rvalue::Binary { lhs, rhs, .. } => {
                self.select_operand(rhs, consumer, cursor);
                self.select_operand(lhs, consumer, cursor);
            }
            Rvalue::Array { .. } | Rvalue::Aggregate { .. } => {}
        }
    }

    fn select_operand(
        &mut self,
        operand: &Operand,
        consumer: ConsumerLocation,
        cursor: &mut usize,
    ) {
        let Some(local) = direct_local(operand) else {
            return;
        };
        let Some(candidate) = self.candidates.get(local) else {
            return;
        };

        if candidate.consumer != consumer
            || candidate.producer.statement.checked_add(1) != Some(*cursor)
        {
            return;
        }

        self.plan.insert(
            local,
            StackifiedValue {
                producer: candidate.producer,
                consumer: candidate.consumer,
            },
        );
        *cursor = candidate.producer.statement;

        let StatementKind::Assign { value, .. } = &self.body.blocks[candidate.producer.block]
            .statements[candidate.producer.statement]
            .kind
        else {
            unreachable!("stackification candidate must be produced by an assignment");
        };
        self.select_rvalue(
            value,
            ConsumerLocation::Statement(candidate.producer),
            cursor,
        );
    }
}

fn direct_local(operand: &Operand) -> Option<LocalId> {
    let Operand::Place(Place {
        base: PlaceBase::Local(local),
        projection,
    }) = operand
    else {
        return None;
    };

    projection.is_empty().then_some(*local)
}
