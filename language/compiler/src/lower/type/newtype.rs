use destack_dir as dir;
use destack_mir as mir;

use crate::lower::{ModuleLowerer, Nominal, NominalField};
use crate::{CompilerError, CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Lower one newtype declaration to its transparent MIR type.
    pub(in crate::lower) fn lower_newtype(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        definition: dir::NewtypeDefinition,
    ) -> CompilerResult<Nominal> {
        // tagged newtypes store as variants of their backing arms
        let cases: Vec<_> = definition
            .members
            .iter()
            .filter_map(|member| match member {
                dir::DefinitionMember::Variant(variant) => Some(NominalField {
                    key: variant.key,
                    symbol: variant.symbol.local_id,
                }),
                _ => None,
            })
            .collect();
        if !cases.is_empty() {
            return self.lower_tagged_newtype(builder, symbol, definition.value, cases);
        }

        // wrap the backing type transparently
        let inner = self.lower_nominal_type(builder, definition.value)?;
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = builder.tree_mut().insert(mir::Type::Newtype { inner, copy });

        Ok(Nominal {
            ty,
            value: ty,
            fields: Vec::new(),
        })
    }

    /// Lower one tagged newtype declaration to its variant carrier.
    pub(in crate::lower) fn lower_tagged_newtype(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        symbol: dir::GlobalSymbolId,
        backing: dir::GlobalTypeId,
        cases: Vec<NominalField>,
    ) -> CompilerResult<Nominal> {
        // each declared case stores its backing arm as the payload
        let dir::Type::Union(union) = self.ty(backing)? else {
            return Err(LowerError::Unsupported {
                anchor: self.module.into(),
                construct: "a tagged newtype over a single arm".to_string(),
            })?;
        };
        let arms = self.types(backing.module_id)?.type_ids(union.elements).to_vec();
        if arms.len() != cases.len() {
            return Err(CompilerError::Internal {
                message: "checked DIR sealed mismatched tagged cases and backing arms".to_string(),
            });
        }
        let mut payloads = Vec::with_capacity(arms.len());
        for arm in &arms {
            payloads.push(self.lower_nominal_type(builder, *arm)?);
        }

        // conformance decides whether values copy or move
        let copy = match self.conforms(symbol, dir::AutoInterface::Copy)? {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };
        let ty = self.variant_type(builder.tree_mut(), payloads, copy);

        Ok(Nominal {
            ty,
            value: ty,
            fields: cases,
        })
    }
}
