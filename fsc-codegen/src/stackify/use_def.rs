use super::location::{ConsumerLocation, StatementLocation};
use cranelift_entity::SecondaryMap;
use fsc_mir::{
    Body, CallArgument, LocalId, Operand, Place, PlaceBase, ProjectionElem, Rvalue, StatementKind,
    TerminatorKind,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DefinitionKind {
    Assignment,
    CallResult,
    RawSyscallResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Definition {
    pub(super) location: StatementLocation,
    pub(super) kind: DefinitionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ValueUseKind {
    Rvalue,
    AggregateElement,
    CallArgument,
    RawSyscallArgument,
    Pause,
    Branch,
    ProjectionIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LocalUseKind {
    Value(ValueUseKind),
    Place,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct LocalUse {
    pub(super) consumer: ConsumerLocation,
    pub(super) kind: LocalUseKind,
}

#[derive(Debug, Clone, Default)]
pub(super) struct LocalFacts {
    pub(super) definitions: Vec<Definition>,
    pub(super) uses: Vec<LocalUse>,
}

pub(super) struct LocalAnalysis {
    facts: SecondaryMap<LocalId, LocalFacts>,
}

impl LocalAnalysis {
    pub(super) fn collect(body: &Body) -> Self {
        let mut analysis = Self {
            facts: SecondaryMap::new(),
        };

        for (block_id, block) in &body.blocks {
            for (statement, data) in block.statements.iter().enumerate() {
                let location = StatementLocation {
                    block: block_id,
                    statement,
                };
                let consumer = ConsumerLocation::Statement(location);

                match &data.kind {
                    StatementKind::Assign { target, value } => {
                        analysis.record_definition(target, location, DefinitionKind::Assignment);
                        analysis.record_rvalue(value, consumer);
                    }
                    StatementKind::Call {
                        destination,
                        arguments,
                        ..
                    } => {
                        if let Some(destination) = destination {
                            analysis.record_definition(
                                destination,
                                location,
                                DefinitionKind::CallResult,
                            );
                        }
                        for argument in arguments {
                            analysis.record_call_argument(
                                argument,
                                consumer,
                                ValueUseKind::CallArgument,
                            );
                        }
                    }
                    StatementKind::RawSyscall {
                        destination,
                        arguments,
                        ..
                    } => {
                        if let Some(destination) = destination {
                            analysis.record_definition(
                                destination,
                                location,
                                DefinitionKind::RawSyscallResult,
                            );
                        }
                        for argument in arguments {
                            analysis.record_call_argument(
                                argument,
                                consumer,
                                ValueUseKind::RawSyscallArgument,
                            );
                        }
                    }
                    StatementKind::Pause { duration } => {
                        analysis.record_operand(duration, consumer, ValueUseKind::Pause);
                    }
                }
            }

            if let TerminatorKind::Branch { condition, .. } = &block.terminator.kind {
                analysis.record_operand(
                    condition,
                    ConsumerLocation::Terminator(block_id),
                    ValueUseKind::Branch,
                );
            }
        }

        analysis
    }

    pub(super) fn facts(&self, local: LocalId) -> &LocalFacts {
        &self.facts[local]
    }

    fn record_definition(
        &mut self,
        place: &Place,
        location: StatementLocation,
        kind: DefinitionKind,
    ) {
        match place.base {
            PlaceBase::Local(local) if place.projection.is_empty() => {
                self.facts[local]
                    .definitions
                    .push(Definition { location, kind });
            }
            PlaceBase::Local(local) => {
                self.record_use(
                    local,
                    ConsumerLocation::Statement(location),
                    LocalUseKind::Place,
                );
            }
            PlaceBase::Static(_) => {}
        }
        self.record_projection_indices(place, ConsumerLocation::Statement(location));
    }

    fn record_rvalue(&mut self, value: &Rvalue, consumer: ConsumerLocation) {
        match value {
            Rvalue::Use(operand) | Rvalue::Unary { operand, .. } => {
                self.record_operand(operand, consumer, ValueUseKind::Rvalue);
            }
            Rvalue::Binary { lhs, rhs, .. } => {
                self.record_operand(lhs, consumer, ValueUseKind::Rvalue);
                self.record_operand(rhs, consumer, ValueUseKind::Rvalue);
            }
            Rvalue::Array { elements } => {
                for element in elements {
                    self.record_operand(element, consumer, ValueUseKind::AggregateElement);
                }
            }
            Rvalue::Aggregate { fields, .. } => {
                for field in fields {
                    self.record_operand(&field.value, consumer, ValueUseKind::AggregateElement);
                }
            }
        }
    }

    fn record_call_argument(
        &mut self,
        argument: &CallArgument,
        consumer: ConsumerLocation,
        value_kind: ValueUseKind,
    ) {
        match argument {
            CallArgument::Value(operand) => self.record_operand(operand, consumer, value_kind),
            CallArgument::Reference(place) => self.record_place(place, consumer),
        }
    }

    fn record_operand(
        &mut self,
        operand: &Operand,
        consumer: ConsumerLocation,
        kind: ValueUseKind,
    ) {
        let Operand::Place(place) = operand else {
            return;
        };

        match place.base {
            PlaceBase::Local(local) if place.projection.is_empty() => {
                self.record_use(local, consumer, LocalUseKind::Value(kind));
            }
            PlaceBase::Local(local) => {
                self.record_use(local, consumer, LocalUseKind::Place);
            }
            PlaceBase::Static(_) => {}
        }
        self.record_projection_indices(place, consumer);
    }

    fn record_place(&mut self, place: &Place, consumer: ConsumerLocation) {
        if let PlaceBase::Local(local) = place.base {
            self.record_use(local, consumer, LocalUseKind::Place);
        }
        self.record_projection_indices(place, consumer);
    }

    fn record_projection_indices(&mut self, place: &Place, consumer: ConsumerLocation) {
        for projection in &place.projection {
            if let ProjectionElem::Index(local) = projection {
                self.record_use(
                    *local,
                    consumer,
                    LocalUseKind::Value(ValueUseKind::ProjectionIndex),
                );
            }
        }
    }

    fn record_use(&mut self, local: LocalId, consumer: ConsumerLocation, kind: LocalUseKind) {
        self.facts[local].uses.push(LocalUse { consumer, kind });
    }
}
