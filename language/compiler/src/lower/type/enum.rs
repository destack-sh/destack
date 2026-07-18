use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, Nominal, NominalField};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Lower one value enum declaration to its MIR variant type.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
    ) -> CompilerResult<Nominal> {
        // require an integer representation supported by MIR variants
        let dir::EnumBackingType::Integer(integer) = definition.backing else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a string-backed enum".to_string(),
            }
            .into());
        };
        let discriminant =
            self.lower_type(&dir::Type::Primitive(dir::PrimitiveType::Integer(integer)))?;
        let discriminant = builder.tree_mut().insert(discriminant);
        let width = integer
            .width()
            .unwrap_or_else(|| u16::from(builder.pointer_bytes()) * 8);

        // build the variant cases in declaration order
        let storage = builder.tree_mut().insert(mir::Type::Void);
        let mut fields = Vec::new();
        let mut variants = Vec::new();
        for variant in definition.variants() {
            let dir::EnumVariantValue::Integer(value) = variant.value else {
                return Err(CompilerError::Internal {
                    message: "integer-backed enum carries a non-integer variant".to_string(),
                });
            };

            fields.push(NominalField {
                key: variant.key,
                symbol: variant.symbol.local_id,
            });
            let discriminant = match integer.is_signed() {
                true => mir::Constant::Int {
                    value: i128::from(value),
                    width,
                    is_signed: true,
                },
                false => mir::Constant::UInt {
                    value: value as u128,
                    width,
                },
            };
            variants.push(mir::VariantCase {
                discriminant,
                ty: storage,
            });
        }

        // conformance decides whether values copy or move
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = builder.tree_mut().insert(mir::Type::Variant {
            discriminant,
            storage,
            cases: variants,
            copy,
        });

        Ok(Nominal {
            ty,
            value: ty,
            fields,
        })
    }
}
