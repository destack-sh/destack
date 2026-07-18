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
        // gather the variant cases in declaration order
        let mut cases = Vec::new();
        for member in &definition.members {
            let dir::DefinitionMember::Variant(variant) = member else {
                continue;
            };

            cases.push(NominalField {
                key: variant.key,
                symbol: variant.symbol.local_id,
            });
        }

        // build each case over the checked discriminant value
        let discriminant = builder.tree_mut().insert(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let storage = builder.tree_mut().insert(mir::Type::Void);
        let mut case_nodes = Vec::with_capacity(cases.len());
        for field in &cases {
            let member = field.symbol.into_global(symbol.module_id);
            case_nodes.push(mir::VariantCase {
                discriminant: self.enum_discriminant(member)?,
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
            cases: case_nodes,
            copy,
        });

        Ok(Nominal {
            ty,
            value: ty,
            fields: cases,
        })
    }

    /// Return the checked discriminant constant of one enum case.
    pub(in crate::lower) fn enum_discriminant(&self, symbol: dir::GlobalSymbolId) -> CompilerResult<mir::Constant> {
        let statics = &self.state(symbol.module_id)?.statics;
        let Some(value) = statics.get_symbol_static_id(symbol) else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a variant discriminant value".to_string(),
            });
        };

        // value enums discriminate over checked integer values
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = self.static_term(value)?
        else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a non-integer enum discriminant".to_string(),
            }
            .into());
        };

        Ok(mir::Constant::Int {
            value: value as i128,
            width: 32,
            is_signed: true,
        })
    }
}
