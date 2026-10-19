use crate::{CodegenError, CodegenResult};
use cranelift_entity::SecondaryMap;
use fsb_ir::{DataRef, Program, Word};
use fsc_mir::{ConstValue, Script, StaticId, Ty, aggregate_fields};

pub(crate) struct StaticData {
    declarations: SecondaryMap<StaticId, Option<DeclaredStatic>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeclaredStatic {
    pub(crate) base: DataRef,
    pub(crate) ty: Ty,
}

impl StaticData {
    pub(crate) fn declare(script: &Script, program: &mut Program) -> CodegenResult<Self> {
        let mut declarations = SecondaryMap::new();

        for (static_id, declaration) in &script.statics {
            let mut words = Vec::new();
            lower_value(
                static_id,
                &declaration.ty,
                &declaration.initializer,
                &mut words,
            )?;
            let data = program.create_data(Some(declaration.name.clone()), words);
            declarations.insert(
                static_id,
                Some(DeclaredStatic {
                    base: DataRef::Internal(data),
                    ty: declaration.ty.clone(),
                }),
            );
        }

        Ok(Self { declarations })
    }

    pub(crate) fn reference(&self, source: StaticId) -> CodegenResult<DataRef> {
        Ok(self.declaration(source)?.base.clone())
    }

    pub(crate) fn declaration(&self, source: StaticId) -> CodegenResult<&DeclaredStatic> {
        self.declarations
            .get(source)
            .and_then(Option::as_ref)
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
