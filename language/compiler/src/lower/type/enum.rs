use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Return one enum or Tagged variant's declaration position.
    pub(in crate::lower) fn variant_position(
        &self,
        owner: dir::GlobalSymbolId,
        variant: dir::GlobalSymbolId,
    ) -> CompilerResult<u32> {
        // select the declaration order owned by the variant family
        let index = match self.definition(owner)? {
            Some(dir::Definition::Enum(definition)) => definition.variant_position(variant),
            Some(dir::Definition::Newtype(definition)) if definition.is_tagged() => {
                definition.tagged_variant_position(variant)
            }
            _ => {
                return Err(CompilerError::Internal {
                    message: "checked DIR variant owner has no variant definition".to_string(),
                });
            }
        };
        let Some(index) = index else {
            return Err(CompilerError::Internal {
                message: "checked DIR variant is missing from its owner definition".to_string(),
            });
        };

        Ok(index as u32)
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower one value enum declaration to its MIR variant type.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        _symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
        ty: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<Vec<NominalField>> {
        // require an integer representation supported by MIR variants
        let dir::EnumBackingType::Integer(integer) = definition.backing else {
            return Err(LowerError::Unsupported {
                anchor: self.lowerer.module.into(),
                construct: "a string-backed enum".to_string(),
            }
            .into());
        };
        let discriminant = self
            .lowerer
            .scalar_type(&dir::Type::Primitive(dir::PrimitiveType::Integer(integer)))?;
        let discriminant = self.tree.intern_type(discriminant);
        let width = integer
            .width()
            .unwrap_or_else(|| u16::from(self.pointer_bytes) * 8);

        // build the variant cases in declaration order
        let storage = self.tree.intern_type(mir::Type::Void);
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

        // value enums carry only their copyable integer discriminant
        let copy = mir::Copy::Yes;
        self.tree.define_type(
            ty,
            mir::Type::Variant {
                discriminant,
                storage,
                cases: variants,
                copy,
            },
        );

        Ok(fields)
    }
}
