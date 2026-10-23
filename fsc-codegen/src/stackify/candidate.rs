use super::{
    location::{ConsumerLocation, StatementLocation},
    use_def::{DefinitionKind, LocalAnalysis, LocalUseKind, ValueUseKind},
};
use cranelift_entity::SecondaryMap;
use fsc_mir::{Body, LocalId, LocalKind, Rvalue, StatementKind, Ty};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Candidate {
    pub(super) producer: StatementLocation,
    pub(super) consumer: ConsumerLocation,
}

pub(super) struct Candidates {
    values: SecondaryMap<LocalId, Option<Candidate>>,
}

impl Candidates {
    pub(super) fn classify(body: &Body, analysis: &LocalAnalysis) -> Self {
        let mut values = SecondaryMap::new();

        for (local, declaration) in &body.locals {
            let facts = analysis.facts(local);
            let ([definition], [use_]) = (facts.definitions.as_slice(), facts.uses.as_slice())
            else {
                continue;
            };

            if body.local_kind(local) != LocalKind::Local
                || !is_stack_scalar(&declaration.ty)
                || definition.kind != DefinitionKind::Assignment
                || !is_supported_use(use_.kind)
                || !definition_precedes_use(definition.location, use_.consumer)
                || !is_stack_producible(body, definition.location)
            {
                continue;
            }

            values[local] = Some(Candidate {
                producer: definition.location,
                consumer: use_.consumer,
            });
        }

        Self { values }
    }

    pub(super) fn get(&self, local: LocalId) -> Option<Candidate> {
        self.values.get(local).copied().flatten()
    }
}

fn is_stack_scalar(ty: &Ty) -> bool {
    matches!(ty, Ty::Int | Ty::Float | Ty::Bool | Ty::Str)
}

fn is_supported_use(kind: LocalUseKind) -> bool {
    matches!(
        kind,
        LocalUseKind::Value(ValueUseKind::Rvalue | ValueUseKind::Pause | ValueUseKind::Branch)
    )
}

fn definition_precedes_use(producer: StatementLocation, consumer: ConsumerLocation) -> bool {
    producer.block == consumer.block()
        && match consumer {
            ConsumerLocation::Statement(location) => producer.statement < location.statement,
            ConsumerLocation::Terminator(_) => true,
        }
}

fn is_stack_producible(body: &Body, producer: StatementLocation) -> bool {
    let StatementKind::Assign { value, .. } =
        &body.blocks[producer.block].statements[producer.statement].kind
    else {
        return false;
    };

    matches!(
        value,
        Rvalue::Use(_) | Rvalue::Unary { .. } | Rvalue::Binary { .. }
    )
}
