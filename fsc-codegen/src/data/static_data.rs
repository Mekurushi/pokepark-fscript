use crate::{CodegenError, CodegenResult};
use cranelift_entity::SecondaryMap;
use fsb_ir::{DataRef, Program, Word};
use fsc_mir::{aggregate_fields, ConstValue, Script, StaticId, Ty};

pub(crate) struct StaticData {
    references: SecondaryMap<StaticId, Option<DataRef>>,
}

impl StaticData {
    pub(crate) fn declare(script: &Script, program: &mut Program) -> CodegenResult<Self> {
        let mut references = SecondaryMap::new();

        for (static_id, declaration) in &script.statics {
            let mut words = Vec::new();
            lower_value(
                static_id,
                &declaration.ty,
                &declaration.initializer,
                &mut words,
            )?;
            let data = program.create_data(Some(declaration.name.clone()), words);
            references.insert(static_id, Some(DataRef::Internal(data)));
        }

        Ok(Self { references })
    }

    pub(crate) fn reference(&self, source: StaticId) -> CodegenResult<DataRef> {
        self.references
            .get(source)
            .and_then(Clone::clone)
            .ok_or(CodegenError::MissingStaticMapping(source))
    }
}

fn lower_value(
    static_id: StaticId,
    ty: &Ty,
    value: &ConstValue,
    words: &mut Vec<Word>,
) -> CodegenResult<()> {
    match (ty, value) {
        (Ty::Int, ConstValue::Int(value)) => words.push(Word::from_i32(*value)),
        (Ty::Float, ConstValue::Float(value)) => words.push(Word::from_f32(*value)),
        (Ty::Bool, ConstValue::Bool(value)) => {
            words.push(Word::from_i32(i32::from(*value)));
        }
        (Ty::Array { element, length }, ConstValue::Array(elements))
            if *length == elements.len() =>
        {
            for element_value in elements {
                lower_value(static_id, element, element_value, words)?;
            }
        }
        (ty, ConstValue::Aggregate(values)) => {
            let Some(fields) = aggregate_fields(ty) else {
                return Err(CodegenError::InvalidStaticInitializer {
                    static_id,
                    ty: ty.clone(),
                });
            };
            for (field, field_value) in fields.iter().zip(values) {
                lower_value(static_id, field.ty(), field_value, words)?;
            }
        }
        (Ty::Str | Ty::Void, _) => {
            return Err(CodegenError::UnsupportedStaticType {
                static_id,
                ty: ty.clone(),
            });
        }
        _ => {
            return Err(CodegenError::InvalidStaticInitializer {
                static_id,
                ty: ty.clone(),
            });
        }
    }

    Ok(())
}
