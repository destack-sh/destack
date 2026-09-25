use tspp_dir as dir;
use tspp_mir as mir;

use crate::lower::{NominalField, TypeLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

impl TypeLowerer<'_, '_> {
    /// Lower one value enum declaration to its variant type.
    pub(in crate::lower) fn lower_enum(
        &mut self,
        symbol: dir::GlobalSymbolId,
        definition: dir::EnumDefinition,
        declaration: mir::LocalNodeId<mir::TypeDeclaration>,
    ) -> CompilerResult<Vec<NominalField>> {
        // require an integer representation
        let dir::EnumBackingType::Integer(_) = definition.backing else {
            return Err(LowerError::Unsupported {
                anchor: self.lower.module.into(),
                construct: "a string-backed enum".to_string(),
            }
            .into());
        };

        // collect the case values
        let mut values = Vec::new();
        for variant in definition.variants() {
            let dir::EnumVariantValue::Integer(value) = variant.value else {
                return Err(CompilerError::Internal {
                    message: "a non-integer variant in an integer-backed enum".to_string(),
                });
            };
            values.push(value);
        }

        // fit the discriminant to the narrowest width holding every case value
        let is_signed = values.iter().any(|value| *value < 0);
        let width = [8u16, 16, 32]
            .into_iter()
            .find(|width| {
                let bound = 1i64 << (width - u16::from(is_signed));
                let floor = if is_signed { -bound } else { 0 };
                values.iter().all(|value| (floor..bound).contains(value))
            })
            .unwrap_or(i64::BITS as u16);
        let integer = dir::IntegerType::Fixed { width, is_signed };

        // lower the discriminant scalar
        let discriminant = self
            .lower
            .scalar_type(&dir::Type::Primitive(dir::PrimitiveType::Integer(integer)))?;
        let discriminant = self.tree.intern_type(discriminant);

        // build the variant cases in declaration order
        let storage = self.tree.intern_type(mir::Type::Void);
        let fields = self.lower.nominal_fields(symbol)?;
        let mut variants = Vec::new();
        for (_, value) in definition.variants().zip(values) {
            // build the case beside its discriminant
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

        // define the variant representation from its cases
        let definition = self.tree.intern_type(mir::Type::Variant {
            discriminant,
            cases: variants,
        });
        self.tree.get_mut(declaration).definition = Some(definition);

        Ok(fields)
    }
}
