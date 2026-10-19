use super::Ty;
use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FieldIdx(u32);

entity_impl!(FieldIdx, "field");

pub(crate) struct AggregateDef {
    fields: &'static [AggregateField],
}

impl AggregateDef {
    pub(crate) fn field(&self, name: &str) -> Option<&'static AggregateField> {
        self.fields.iter().find(|field| field.name() == name)
    }
}

pub struct AggregateField {
    index: FieldIdx,
    name: &'static str,
    ty: Ty,
}

impl AggregateField {
    pub const fn index(&self) -> FieldIdx {
        self.index
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn ty(&self) -> &Ty {
        &self.ty
    }
}

static VEC3_FIELDS: [AggregateField; 3] = [
    AggregateField {
        index: FieldIdx(0),
        name: "x",
        ty: Ty::Float,
    },
    AggregateField {
        index: FieldIdx(1),
        name: "y",
        ty: Ty::Float,
    },
    AggregateField {
        index: FieldIdx(2),
        name: "z",
        ty: Ty::Float,
    },
];

static VEC3: AggregateDef = AggregateDef {
    fields: &VEC3_FIELDS,
};

pub(crate) fn aggregate_definition(ty: &Ty) -> Option<&'static AggregateDef> {
    match ty {
        Ty::Vec3 => Some(&VEC3),
        _ => None,
    }
}

pub fn aggregate_fields(ty: &Ty) -> Option<&'static [AggregateField]> {
    aggregate_definition(ty).map(|definition| definition.fields)
}
