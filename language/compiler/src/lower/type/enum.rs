use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one value enum declaration to its variant type.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        _symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
        ty: mir::LocalNodeId<mir::Type>,
        copy: mir::Copy,
    ) -> CompilerResult<Vec<NominalField>> {
        // require an integer representation
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

            // record the member beside the discriminant it carries
            fields.push(NominalField {
                key: variant.key,
                symbol: variant.symbol,
                is_optional: false,
                initializer: None,
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

        self.tree.define_type(
            ty,
            mir::Type::Variant {
                discriminant,
                cases: variants,
                copy,
            },
        );

        Ok(fields)
    }
}
