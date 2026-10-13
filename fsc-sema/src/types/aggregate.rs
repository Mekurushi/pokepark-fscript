use super::Ty;
use cranelift_entity::entity_impl;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FieldIdx(u32);

entity_impl!(FieldIdx, "field");

pub(crate) struct AggregateDef {
    fields: &'static [FieldDef],
}

impl AggregateDef {
    pub(crate) fn field(&self, name: &str) -> Option<&'static FieldDef> {
        self.fields.iter().find(|field| field.name == name)
    }
}

pub(crate) struct FieldDef {
    pub(crate) index: FieldIdx,
    pub(crate) name: &'static str,
    pub(crate) ty: Ty,
}

static VEC3_FIELDS: [FieldDef; 3] = [
    FieldDef {
        index: FieldIdx(0),
        name: "x",
        ty: Ty::Float,
    },
    FieldDef {
        index: FieldIdx(1),
        name: "y",
        ty: Ty::Float,
    },
    FieldDef {
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
