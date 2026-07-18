use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::ModuleLowerer;
use crate::{CompilerResult, LowerError};

impl ModuleLowerer<'_> {
    /// Lower one checked type during the nominal pass, following declarations.
    pub(in crate::lower) fn lower_nominal_type(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.ty(id)? {
            // nominal instances lower their declarations first
            dir::Type::Instance(instance) => self.ensure_nominal(builder, instance.symbol),
            // tuples lower their elements recursively
            dir::Type::Tuple(tuple) => {
                let ids = self.tuple_element_types(id.module_id, &tuple)?;
                let mut elements = Vec::with_capacity(ids.len());
                for element in ids {
                    elements.push(self.lower_nominal_type(builder, element)?);
                }

                Ok(self.tuple_type(builder.tree_mut(), elements))
            }
            // enum members carry their owning enum
            dir::Type::EnumMember(member) => self.lower_nominal_type(builder, member.owner),
            // instance substitutions resolve generic parameters
            dir::Type::Parameter(parameter) => {
                let Some(argument) = self.substitution.get(&parameter).copied() else {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a generic parameter outside its instance".to_string(),
                    })?;
                };

                self.lower_nominal_type(builder, argument)
            }
            // nullable unions ride their reference carrier's niches
            dir::Type::Union(union) => {
                if let Some((nullability, carrier)) =
                    self.decompose_nullish_union(id.module_id, &union)?
                {
                    let reference = self.lower_nominal_type(builder, carrier)?;

                    return self.nullable_reference(builder.tree_mut(), reference, nullability);
                }

                // literal unions store at their family carrier
                if let Some(carrier) = self.literal_union_carrier(id.module_id, &union)? {
                    return Ok(builder.tree_mut().insert(carrier));
                }

                // tagged unions store as indexed variants
                let elements = self
                    .types(id.module_id)?
                    .type_ids(union.elements)
                    .to_vec();
                let mut payloads = Vec::with_capacity(elements.len());
                for element in &elements {
                    payloads.push(self.lower_nominal_type(builder, *element)?);
                }

                Ok(self.union_variant_type(builder.tree_mut(), payloads))
            }
            // memory forms resolve after their nested nominals exist
            dir::Type::Form(_) => {
                self.ensure_form_nominals(builder, id)?;

                self.lower_form(builder.tree_mut(), id, None)
            }
            other => {
                let ty = self.lower_type(&other)?;

                Ok(builder.tree_mut().insert(ty))
            }
        }
    }

    /// Lower one checked type to a MIR type node.
    pub(in crate::lower) fn lower_type_id(
        &self,
        tree: &mut mir::Tree,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.ty(id)? {
            // nominal instances share their declared MIR type
            dir::Type::Instance(instance) => Ok(self.nominal(&instance)?.value),
            // enum members carry their owning enum
            dir::Type::EnumMember(member) => self.lower_type_id(tree, member.owner),
            // instance substitutions resolve generic parameters
            dir::Type::Parameter(parameter) => {
                let Some(argument) = self.substitution.get(&parameter).copied() else {
                    return Err(LowerError::Unsupported {
                        anchor: self.module.into(),
                        construct: "a generic parameter outside its instance".to_string(),
                    })?;
                };

                self.lower_type_id(tree, argument)
            }
            // nullable unions ride their reference carrier's niches
            dir::Type::Union(union) => {
                if let Some((nullability, carrier)) =
                    self.decompose_nullish_union(id.module_id, &union)?
                {
                    let reference = self.lower_type_id(tree, carrier)?;

                    return self.nullable_reference(tree, reference, nullability);
                }

                // literal unions store at their family carrier
                if let Some(carrier) = self.literal_union_carrier(id.module_id, &union)? {
                    return Ok(tree.insert(carrier));
                }

                // tagged unions store as indexed variants
                let elements = self
                    .types(id.module_id)?
                    .type_ids(union.elements)
                    .to_vec();
                let mut payloads = Vec::with_capacity(elements.len());
                for element in &elements {
                    payloads.push(self.lower_type_id(tree, *element)?);
                }

                Ok(self.union_variant_type(tree, payloads))
            }
            // memory forms resolve through the form algebra
            dir::Type::Form(_) => self.lower_form(tree, id, None),
            // tuples lower their elements recursively
            dir::Type::Tuple(tuple) => {
                let ids = self.tuple_element_types(id.module_id, &tuple)?;
                let mut elements = Vec::with_capacity(ids.len());
                for element in ids {
                    elements.push(self.lower_type_id(tree, element)?);
                }

                Ok(self.tuple_type(tree, elements))
            }
            other => {
                let ty = self.lower_type(&other)?;

                Ok(tree.insert(ty))
            }
        }
    }

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

    /// Insert one indexed variant type with copy composed over its payloads.
    pub(in crate::lower) fn union_variant_type(
        &self,
        tree: &mut mir::Tree,
        payloads: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        // a variant copies when every payload copies
        let copy = payloads
            .iter()
            .all(|payload| tree.get(*payload).copy() == mir::Copy::Yes);
        let copy = match copy {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        self.variant_type(tree, payloads, copy)
    }

    /// Insert one indexed variant type over the given case payloads.
    pub(in crate::lower) fn variant_type(
        &self,
        tree: &mut mir::Tree,
        payloads: Vec<mir::LocalNodeId<mir::Type>>,
        copy: mir::Copy,
    ) -> mir::LocalNodeId<mir::Type> {
        // cases discriminate by their declaration index
        let discriminant = tree.insert(mir::Type::Int {
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
            .unwrap_or_else(|| tree.insert(mir::Type::Void));

        tree.insert(mir::Type::Variant {
            discriminant,
            storage,
            cases,
            copy,
        })
    }

    /// Insert one tuple type with copy composed over its elements.
    pub(in crate::lower) fn tuple_type(
        &self,
        tree: &mut mir::Tree,
        elements: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        // a tuple copies when every element copies
        let copy = elements
            .iter()
            .all(|element| tree.get(*element).copy() == mir::Copy::Yes);
        let copy = match copy {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        tree.insert(mir::Type::Tuple {
            elements: elements.into_iter().collect(),
            copy,
        })
    }
}
