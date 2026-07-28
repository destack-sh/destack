use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    GenericInstanceKey, LifetimeParameters, ModuleLowerer, NominalInstance, TypeSubstitution,
};
use crate::{CompilerError, CompilerResult, LowerError};

/// Recursive checked type lowering into one MIR tree.
pub(in crate::lower) struct TypeLowerer<'lower, 'module> {
    /// The stable module lowering state.
    pub(in crate::lower) lowerer: &'lower mut ModuleLowerer<'module>,
    /// The MIR tree receiving lowered types.
    pub(in crate::lower) tree: &'lower mut mir::Tree,
    /// The target pointer width in bytes.
    pub(in crate::lower) pointer_bytes: u8,
    /// The concrete type substitutions applied during lowering.
    pub(in crate::lower) type_substitution: &'lower TypeSubstitution,
    /// The polymorphic lifetime parameters available during lowering.
    pub(in crate::lower) lifetime_parameters: &'lower LifetimeParameters,
}

impl<'lower, 'module> TypeLowerer<'lower, 'module> {
    /// Create recursive type lowering over one MIR tree.
    fn new(
        lowerer: &'lower mut ModuleLowerer<'module>,
        tree: &'lower mut mir::Tree,
        pointer_bytes: u8,
        type_substitution: &'lower TypeSubstitution,
        lifetime_parameters: &'lower LifetimeParameters,
    ) -> Self {
        Self {
            lowerer,
            tree,
            pointer_bytes,
            type_substitution,
            lifetime_parameters,
        }
    }

    /// Lower one checked type into the MIR tree.
    pub(in crate::lower) fn lower(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let id = self.lowerer.reduced_type(id)?;

        match self.lowerer.ty(id)? {
            // nominal instances lower through their concrete representation
            dir::Type::Application(instance) => {
                if let Some(argument) = self.lowerer.storage_carrier_argument(id, &instance)? {
                    let value = self.lower(argument)?;

                    return self.insert_storage_carrier(&instance, value);
                }

                let arguments = self
                    .lowerer
                    .types(id.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();
                let nominal = self.lower_nominal(instance.symbol, &arguments)?;

                Ok(nominal.value)
            }
            // variants lower through their owning carrier
            dir::Type::Variant(member) => self.lower(member.owner),
            // instance substitutions resolve generic parameters
            dir::Type::Parameter(_) => {
                let argument = self.type_substitution.resolve(self.lowerer, id)?;

                self.lower(argument)
            }
            // contextual this lowers through the receiver substitution
            dir::Type::This => Ok(self.lower_receiver()?.value),
            // nullable unions ride their reference carrier's niches
            dir::Type::Union(union) => {
                if let Some((nullability, carrier)) =
                    self.lowerer.decompose_nullish_union(id.module_id, &union)?
                {
                    let reference = self.lower(carrier)?;

                    return self.insert_nullable_reference(reference, nullability);
                }

                // non-nullish unions store as indexed variants
                let elements = self
                    .lowerer
                    .types(id.module_id)?
                    .type_ids(union.elements)
                    .to_vec();
                let mut payloads = Vec::with_capacity(elements.len());
                for element in elements {
                    payloads.push(self.lower(element)?);
                }

                Ok(self.insert_union_variant(payloads))
            }
            // memory forms resolve through the form algebra
            dir::Type::Form(_) => self.lower_form(id, None),
            // tuples lower their elements recursively
            dir::Type::Tuple(tuple) => {
                let ids = self.lowerer.tuple_element_types(id.module_id, &tuple)?;
                let mut elements = Vec::with_capacity(ids.len());
                for element in ids {
                    elements.push(self.lower(element)?);
                }

                Ok(self.insert_tuple(elements))
            }
            other => {
                let ty = self.lowerer.scalar_type(&other)?;

                Ok(self.tree.intern_type(ty))
            }
        }
    }

    /// Build the generic instance key from representation-relevant type arguments.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        let mut representations = Vec::with_capacity(arguments.len());
        for argument in arguments {
            let representation = self.lower(*argument)?;
            let representation = self.tree.intern_representation(representation);
            representations.push(representation);
        }

        Ok(GenericInstanceKey {
            symbol,
            representations,
        })
    }
}

impl<'module> ModuleLowerer<'module> {
    /// Return recursive type lowering over one MIR tree.
    pub(in crate::lower) fn type_lowerer<'lower>(
        &'lower mut self,
        tree: &'lower mut mir::Tree,
        pointer_bytes: u8,
        type_substitution: &'lower TypeSubstitution,
        lifetime_parameters: &'lower LifetimeParameters,
    ) -> TypeLowerer<'lower, 'module> {
        TypeLowerer::new(
            self,
            tree,
            pointer_bytes,
            type_substitution,
            lifetime_parameters,
        )
    }

    /// Return the wrapped argument when one instance is a storage carrier.
    fn storage_carrier_argument(
        &self,
        id: dir::GlobalTypeId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let item = self.language_item(instance.symbol)?;
        if !matches!(
            item,
            Some(dir::LanguageItem::MaybeUninit | dir::LanguageItem::ManuallyDrop)
        ) {
            return Ok(None);
        }

        let arguments = self.types(id.module_id)?.type_ids(instance.arguments);
        let Some(argument) = arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "checked DIR instantiated a storage carrier without its value".to_string(),
            });
        };

        Ok(Some(argument))
    }
}

impl TypeLowerer<'_, '_> {
    /// Lower the contextual receiver application to its nominal representation.
    pub(in crate::lower) fn lower_receiver(&mut self) -> CompilerResult<NominalInstance> {
        let Some(receiver) = self.type_substitution.receiver().cloned() else {
            return Err(CompilerError::Internal {
                message: "checked DIR left a contextual receiver unbound".to_string(),
            });
        };
        let arguments = self
            .lowerer
            .types(receiver.symbol.module_id)?
            .type_ids(receiver.arguments)
            .to_vec();

        self.lower_nominal(receiver.symbol, &arguments)
    }

    /// Wrap one lowered value type in its storage carrier.
    fn insert_storage_carrier(
        &mut self,
        instance: &dir::GenericApplication,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let ty = match self.lowerer.language_item(instance.symbol)? {
            Some(dir::LanguageItem::MaybeUninit) => mir::Type::Uninit { value },
            Some(dir::LanguageItem::ManuallyDrop) => mir::Type::ManuallyDrop { value },
            _ => {
                return Err(CompilerError::Internal {
                    message: "lowered a storage carrier without its language item".to_string(),
                });
            }
        };

        Ok(self.tree.intern_type(ty))
    }
}

impl ModuleLowerer<'_> {
    /// Return the element types of one plain tuple in position order.
    pub(in crate::lower) fn tuple_element_types(
        &self,
        module: ModuleId,
        tuple: &dir::TupleType,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        let mut ids = Vec::with_capacity(tuple.elements.len() as usize);
        for element in self.types(module)?.elements(tuple.elements) {
            // optional and rest elements have no fixed positional layout
            if element.is_optional || element.is_rest {
                return Err(LowerError::Unsupported {
                    anchor: self.module.into(),
                    construct: "an optional or rest tuple element".to_string(),
                }
                .into());
            }
            ids.push(element.ty);
        }

        Ok(ids)
    }
}

impl TypeLowerer<'_, '_> {
    /// Insert one indexed variant type with copy composed over its payloads.
    pub(in crate::lower) fn insert_union_variant(
        &mut self,
        payloads: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        // a variant copies when every payload copies
        let copy = payloads
            .iter()
            .all(|payload| self.tree.get(*payload).copy(self.tree) == mir::Copy::Yes);
        let copy = match copy {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        let variant = self.variant_type(payloads, copy);

        self.tree.intern_type(variant)
    }

    /// Build one indexed variant type over the given case payloads.
    pub(in crate::lower) fn variant_type(
        &mut self,
        payloads: Vec<mir::LocalNodeId<mir::Type>>,
        copy: mir::Copy,
    ) -> mir::Type {
        // cases discriminate by their declaration index
        let discriminant = self.tree.intern_type(mir::Type::Int {
            width: 8,
            is_signed: false,
        });
        let cases: Vec<_> = payloads
            .iter()
            .enumerate()
            .map(|(index, payload)| mir::VariantCase {
                discriminant: mir::Constant::UInt {
                    value: index as u128,
                    width: 8,
                },
                ty: *payload,
            })
            .collect();

        // the first case stands in as the logical storage carrier
        let storage = payloads
            .first()
            .copied()
            .unwrap_or_else(|| self.tree.intern_type(mir::Type::Void));

        mir::Type::Variant {
            discriminant,
            storage,
            cases,
            copy,
        }
    }

    /// Insert one tuple type with copy composed over its elements.
    pub(in crate::lower) fn insert_tuple(
        &mut self,
        elements: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        // a tuple copies when every element copies
        let copy = elements
            .iter()
            .all(|element| self.tree.get(*element).copy(self.tree) == mir::Copy::Yes);
        let copy = match copy {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        self.tree.intern_type(mir::Type::Tuple {
            elements: elements.into_iter().collect(),
            copy,
        })
    }
}
