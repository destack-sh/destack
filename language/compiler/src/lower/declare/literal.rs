use std::sync::Arc;

use destack_core::StringId;
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{LifetimeParameters, ModuleLowerer, NominalInstance};
use crate::{CompilerError, CompilerResult};

impl ModuleLowerer<'_> {
    /// Declare the immortal globals backing every collected string literal.
    pub(in crate::lower) fn declare_string_literals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        strings: impl IntoIterator<Item = StringId>,
    ) -> CompilerResult<()> {
        for string in strings {
            // declare each distinct content once
            if self.string_literals.contains_key(&string) {
                continue;
            }

            // bank declaration failures for the first reading body
            match self.declare_string_literal(builder, string) {
                Ok(declared) => {
                    self.string_literals.insert(string, Ok(declared));
                }
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.string_literals
                        .insert(string, Err(Arc::from(diagnostic)));
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare the immortal globals backing every collected bigint literal.
    pub(in crate::lower) fn declare_bigint_literals(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bigints: impl IntoIterator<Item = i64>,
    ) -> CompilerResult<()> {
        for bigint in bigints {
            // declare each distinct value once
            if self.bigint_literals.contains_key(&bigint) {
                continue;
            }

            // bank declaration failures for the first reading body
            match self.declare_bigint_literal(builder, bigint) {
                Ok(declared) => {
                    self.bigint_literals.insert(bigint, Ok(declared));
                }
                Err(CompilerError::Diagnostic(diagnostic)) => {
                    self.bigint_literals
                        .insert(bigint, Err(Arc::from(diagnostic)));
                }
                Err(error) => return Err(error),
            }
        }

        Ok(())
    }

    /// Declare one literal's constant code units and pre-built immortal String object.
    fn declare_string_literal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        string: StringId,
    ) -> CompilerResult<(mir::GlobalId, mir::LocalNodeId<mir::Type>)> {
        // lower the String representation named by its language item
        let nominal = self.lower_literal_nominal(builder, dir::LanguageItem::String)?;

        // encode the constant UTF-16 code units into an immortal array
        let code_units = self.strings.get(string).encode_utf16();
        let length = code_units.clone().count() as u64;
        let bytes = code_units.flat_map(u16::to_le_bytes).collect();
        let array = Self::intern_element_array(builder, 16, length);
        let code_units_global = builder.immortal(
            &format!("string.{}.codeUnits", string.0),
            array,
            mir::GlobalInitializer::Bytes(bytes),
        );

        // pre-build the immortal String object over its constant code units
        let initializer = builder
            .constant_object(
                nominal.storage,
                &[(
                    "codeUnits",
                    mir::ConstantValue::Slice {
                        elements: code_units_global,
                        length,
                    },
                )],
            )
            .map_err(|error| CompilerError::Internal {
                message: error.to_string(),
            })?;
        let object = builder.immortal(
            &format!("string.{}", string.0),
            nominal.storage,
            initializer,
        );

        Ok((object, nominal.value))
    }

    /// Declare one literal's constant limbs and pre-built immortal BigInt object.
    fn declare_bigint_literal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        bigint: i64,
    ) -> CompilerResult<(mir::GlobalId, mir::LocalNodeId<mir::Type>)> {
        // lower the BigInt representation named by its language item
        let nominal = self.lower_literal_nominal(builder, dir::LanguageItem::BigInt)?;

        // intern the little-endian magnitude limbs as an immortal limb array
        let limbs: Vec<u64> = match bigint {
            0 => Vec::new(),
            value => vec![value.unsigned_abs()],
        };
        let length = limbs.len() as u64;
        let bytes = limbs.iter().flat_map(|limb| limb.to_le_bytes()).collect();
        let array = Self::intern_element_array(builder, 64, length);
        let name = Self::bigint_name(bigint);
        let limbs_global = builder.immortal(
            &format!("bigint.{name}.limbs"),
            array,
            mir::GlobalInitializer::Bytes(bytes),
        );

        // pre-build the immortal BigInt object over its constant limbs
        let pointer_width = (builder.pointer_bytes() * 8) as u16;
        let initializer = builder
            .constant_object(
                nominal.storage,
                &[
                    (
                        "sign",
                        mir::ConstantValue::Scalar(mir::Constant::Int {
                            value: bigint.signum() as i128,
                            width: 8,
                            is_signed: true,
                        }),
                    ),
                    (
                        "limbs",
                        mir::ConstantValue::Slice {
                            elements: limbs_global,
                            length,
                        },
                    ),
                    (
                        "length",
                        mir::ConstantValue::Scalar(mir::Constant::UInt {
                            value: length as u128,
                            width: pointer_width,
                        }),
                    ),
                ],
            )
            .map_err(|error| CompilerError::Internal {
                message: error.to_string(),
            })?;
        let object = builder.immortal(&format!("bigint.{name}"), nominal.storage, initializer);

        Ok((object, nominal.value))
    }

    /// Lower the nominal representation named by one literal language item.
    fn lower_literal_nominal(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        item: dir::LanguageItem,
    ) -> CompilerResult<NominalInstance> {
        let symbol = self.language_item_symbol(item)?;
        let lifetime_parameters = LifetimeParameters::default();
        let pointer_bytes = builder.pointer_bytes();
        let mut lowerer =
            self.type_lowerer(builder.tree_mut(), pointer_bytes, &lifetime_parameters);

        lowerer.lower_nominal(symbol, &[])
    }

    /// Intern one fixed unsigned element array type for immortal payload bytes.
    fn intern_element_array(
        builder: &mut mir::ModuleBuilder,
        width: u16,
        length: u64,
    ) -> mir::LocalNodeId<mir::Type> {
        let element = builder.tree_mut().intern_type(mir::Type::Int {
            width,
            is_signed: false,
        });
        let copy = builder.tree().get(element).copy(builder.tree());

        builder.tree_mut().intern_type(mir::Type::FixedArray {
            element,
            length,
            copy,
        })
    }

    /// Render one bigint value as a global name segment.
    fn bigint_name(bigint: i64) -> String {
        if bigint < 0 {
            format!("n{}", bigint.unsigned_abs())
        } else {
            format!("{bigint}")
        }
    }
}
