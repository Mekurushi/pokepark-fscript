use super::location::{ConsumerLocation, StatementLocation};
use cranelift_entity::SecondaryMap;
use fsc_mir::{BasicBlockId, Body, LocalId, Place, PlaceBase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StackifiedValue {
    pub(crate) producer: StatementLocation,
    pub(crate) consumer: ConsumerLocation,
}

#[derive(Debug)]
pub(crate) struct StackificationPlan {
    values: SecondaryMap<LocalId, Option<StackifiedValue>>,
    selected_statements: SecondaryMap<BasicBlockId, Vec<bool>>,
}

impl StackificationPlan {
    pub(crate) fn new(body: &Body) -> Self {
        let mut selected_statements = SecondaryMap::new();
        for (block, data) in &body.blocks {
            selected_statements[block] = vec![false; data.statements.len()];
        }

        Self {
            values: SecondaryMap::new(),
            selected_statements,
        }
    }

    pub(super) fn insert(&mut self, local: LocalId, value: StackifiedValue) {
        if let Some(existing) = self.values.get(local).copied().flatten() {
            debug_assert_eq!(existing, value);
        } else {
            self.values[local] = Some(value);
            self.selected_statements[value.producer.block][value.producer.statement] = true;
        }
    }

    pub(crate) fn value(&self, place: &Place) -> Option<StackifiedValue> {
        let Place {
            base: PlaceBase::Local(local),
            projection,
        } = place
        else {
            return None;
        };

        if projection.is_empty() {
            self.values.get(*local).copied().flatten()
        } else {
            None
        }
    }

    pub(crate) fn selects_statement(&self, location: StatementLocation) -> bool {
        self.selected_statements
            .get(location.block)
            .and_then(|statements| statements.get(location.statement))
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn is_stackified(&self, local: LocalId) -> bool {
        self.values.get(local).copied().flatten().is_some()
    }
}
