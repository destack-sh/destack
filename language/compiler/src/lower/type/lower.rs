use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{GenericInstanceKey, LifetimeParameters, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// Recursive type lowering into one tree.
pub(in crate::lower) struct TypeLowerer<'lower, 'module> {
    /// The stable module lowering state.
    pub(in crate::lower) lowerer: &'lower mut ModuleLowerer<'module>,
    /// The tree receiving lowered types.
    pub(in crate::lower) tree: &'lower mut mir::Tree,
    /// The target pointer width in bytes.
    pub(in crate::lower) pointer_bytes: u8,
    /// The polymorphic lifetime parameters available during lowering.
    pub(in crate::lower) lifetime_parameters: &'lower LifetimeParameters,
    /// The sema instance whose materialized rows resolve read types.
    pub(in crate::lower) instance: Option<(ModuleId, dir::LocalInstanceId)>,
    /// The compound types being lowered, with reservations for revisits.
    reservations: FxIndexMap<dir::GlobalTypeId, Option<mir::LocalNodeId<mir::Type>>>,
}

impl<'lower, 'module> TypeLowerer<'lower, 'module> {
    /// Create recursive type lowering over one tree.
    fn new(
        lowerer: &'lower mut ModuleLowerer<'module>,
        tree: &'lower mut mir::Tree,
        pointer_bytes: u8,
        lifetime_parameters: &'lower LifetimeParameters,
    ) -> Self {
        Self {
            lowerer,
            tree,
            pointer_bytes,
            lifetime_parameters,
            instance: None,
            reservations: FxIndexMap::default(),
        }
    }

    /// Return this walker resolving reads through one instance's rows.
    pub(in crate::lower) fn with_instance(
        mut self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
    ) -> Self {
        self.instance = instance;

        self
    }

    /// Return a nested walker resolving through another instance's rows.
    pub(in crate::lower) fn nested<'nested>(
        &'nested mut self,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
    ) -> TypeLowerer<'nested, 'module> {
        TypeLowerer {
            lowerer: &mut *self.lowerer,
            tree: &mut *self.tree,
            pointer_bytes: self.pointer_bytes,
            lifetime_parameters: self.lifetime_parameters,
            instance,
            reservations: FxIndexMap::default(),
        }
    }

    /// Lower one type.
    pub(in crate::lower) fn lower(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the written id through its materialized types
        let id = self.lowerer.instance_type(self.instance, id)?;

        // lower alias declarations through the nominal they name
        let alias = match self.lowerer.ty(id)? {
            dir::Type::Reference(reference) => Some((reference.symbol, None)),
            dir::Type::Application(instance) => Some((instance.symbol, Some(instance.arguments))),
            _ => None,
        };
        if let Some((symbol, arguments)) = alias
            && self.lowerer.alias_form(symbol)?.is_some()
        {
            let arguments = match arguments {
                Some(list) => self.lowerer.types(id.module_id)?.type_ids(list).to_vec(),
                None => Vec::new(),
            };

            // reject bare defaulted uses without instantiation arguments
            let is_generic = match self.lowerer.definition(symbol)? {
                Some(definition) => self
                    .lowerer
                    .definition_is_parameterized(symbol.module_id, definition)?,
                None => false,
            };
            if is_generic && arguments.is_empty() {
                return Err(LowerError::Unsupported {
                    anchor: self.lowerer.module.into(),
                    construct: "a defaulted generic object alias".to_string(),
                }
                .into());
            }

            return Ok(self.lower_nominal(symbol, &arguments)?.value);
        }

        // reserve an identity for anonymous compound graphs, which can cycle through their members
        let compound = matches!(
            self.lowerer.ty(id)?,
            dir::Type::Union(_)
                | dir::Type::Tuple(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_)
                | dir::Type::Object(_)
        );
        if compound {
            // name the persistent identity this cyclic graph resolves to
            let name = format!("cycle{}:{}", id.module_id, id.local_id.0);
            let symbol = mir::Symbol::named(self.lowerer.strings.intern(&name));

            // reuse the identity an earlier walk defined for this cycle
            if let Some(defined) = self.tree.identified_type(symbol)
                && !self.tree.type_is_reserved(defined)
                && !self.reservations.contains_key(&id)
            {
                return Ok(defined);
            }

            // return the reservation a nested revisit takes, creating it once
            if let Some(entry) = self.reservations.get_mut(&id) {
                if let Some(reserved) = entry {
                    return Ok(*reserved);
                }
                let reserved = self.tree.reserve_type(symbol);
                *entry = Some(reserved);

                return Ok(reserved);
            }

            self.reservations.insert(id, None);
        }

        // lower the family, then define any reservation a revisit created
        let mut lowered = self.lower_family(id);
        if compound
            && let Some(reservation) = self.reservations.swap_remove(&id)
            && let (Ok(result), Some(reserved)) = (&lowered, reservation)
        {
            let value = self.tree.get(*result).clone();
            self.tree.define_type(reserved, value);

            // give isomorphic cycles one shared node through the canonical registry
            let key = self.tree.canonical_key(reserved);
            match self.tree.canonical_type(&key) {
                Some(existing) => lowered = Ok(existing),
                None => {
                    let canonical = *result;
                    for (member, member_key) in self.tree.canonical_component(canonical) {
                        self.tree.register_canonical(member_key, member);
                    }
                    self.tree.register_canonical(key, canonical);
                    lowered = Ok(canonical);
                }
            }
        }

        lowered
    }

    /// Lower one type by its family.
    pub(super) fn lower_family(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        match self.lowerer.ty(id)? {
            // lower nominal instances through their concrete representation
            dir::Type::Application(instance) => {
                if let Some(argument) = self.lowerer.storage_carrier_argument(id, &instance)? {
                    let value = self.lower(argument)?;

                    return self.insert_storage_carrier(&instance, value);
                }

                // lower the instance at its applied arguments
                let arguments = self
                    .lowerer
                    .types(id.module_id)?
                    .type_ids(instance.arguments)
                    .to_vec();
                let nominal = self.lower_nominal(instance.symbol, &arguments)?;

                Ok(nominal.value)
            }
            // lower bare references as their nominal applications
            dir::Type::Reference(reference) => {
                let nominal = self.lower_nominal(reference.symbol, &[])?;

                Ok(nominal.value)
            }
            // lower variants through their owning carrier
            dir::Type::Variant(member) => self.lower(member.owner),
            // reject parameters, which materialized rows resolve before lowering
            dir::Type::Parameter(parameter) => {
                let template = self
                    .lowerer
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id)
                    .template;
                let declared = self
                    .lowerer
                    .state(parameter.module_id)?
                    .generics
                    .get_template(template)
                    .symbol;
                let path = match declared {
                    Some(symbol) => self.lowerer.symbol_path(symbol)?,
                    None => "an anonymous template".to_string(),
                };

                Err(CompilerError::Internal {
                    message: format!("a type parameter of '{path}' was never materialized"),
                })
            }
            // reject contextual this, which materialization resolves before lowering
            dir::Type::This => Err(CompilerError::Internal {
                message: "a contextual this was never materialized".to_string(),
            }),
            // ride the reference carrier's niches for nullable unions
            dir::Type::Union(union) => {
                if let Some((nullability, carrier)) =
                    self.lowerer.decompose_nullish_union(id.module_id, &union)?
                {
                    let reference = self.lower(carrier)?;

                    return self.insert_nullability(reference, nullability);
                }

                // store non-nullish unions as indexed variants
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
            // erase signature-bearing object types behind the dynamic carrier
            dir::Type::Object(shape) if shape.declares_signatures() => self.lower_dynamic(id),
            // store concrete object classes behind a managed reference
            dir::Type::Object(shape) => {
                let storage = self.lower_object_struct(&shape, id.module_id)?;

                Ok(self.insert_reference(
                    mir::ReferenceKind::Managed,
                    mir::Access::Mutable,
                    storage,
                ))
            }
            // erase the top constraint behind the dynamic carrier
            dir::Type::Unknown => self.lower_dynamic(id),
            // resolve memory forms through the form algebra
            dir::Type::Form(_) => self.lower_form(id, None),
            // lower slice values as fat headers over managed element storage
            dir::Type::Slice(_) => self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                mir::Access::Mutable,
                id,
            ),
            // default dynamic and callable values to managed fat references
            dir::Type::Dynamic(_) | dir::Type::Function(_) => self.lower_reference(
                mir::ReferenceKind::Managed,
                mir::Lifetime::empty(),
                mir::Access::Mutable,
                id,
            ),
            // fixed arrays lower to their inline element storage
            dir::Type::FixedArray(fixed) => {
                let element = self.lower(fixed.element)?;
                let length = self.lowerer.fixed_array_length(fixed.count)?;
                let copy = self.tree.get(element).copy(self.tree);

                Ok(self.tree.intern_type(mir::Type::FixedArray {
                    element,
                    length,
                    copy,
                }))
            }
            // lower tuple elements recursively
            dir::Type::Tuple(tuple) => {
                let ids = self.lowerer.tuple_element_types(id.module_id, &tuple)?;
                let mut elements = Vec::with_capacity(ids.len());
                for element in ids {
                    elements.push(self.lower(element)?);
                }

                Ok(self.insert_tuple(elements))
            }
            dir::Type::FunctionPointer(pointer) => {
                let signature = self.lower_callable_signature(pointer.signature)?;

                Ok(self
                    .tree
                    .intern_type(mir::Type::FunctionPointer { signature }))
            }
            // lower a bare signature in value position as a managed callable
            dir::Type::FunctionSignature(_) => {
                let signature = self.lower_callable_signature(id)?;

                Ok(self.tree.intern_type(mir::Type::Function {
                    multiplicity: mir::Multiplicity::Repeatable,
                    kind: mir::ReferenceKind::Managed,
                    lifetime: mir::Lifetime::empty(),
                    signature,
                    storage: mir::Storage::Heap(mir::Space::Local),
                    access: mir::Access::Mutable,
                    nullability: mir::Nullability::None,
                }))
            }
            // lower never without a value
            dir::Type::Never => Ok(self.tree.intern_type(mir::Type::Never)),
            other => {
                // lower reference primitives through their representation classes
                if let Some(item) = ModuleLowerer::representation_item(&other) {
                    let symbol = self.lowerer.language_item_symbol(item)?;

                    return Ok(self.lower_nominal(symbol, &[])?.value);
                }

                // fall back to the scalar families
                let ty = self.lowerer.scalar_type(&other)?;

                Ok(self.tree.intern_type(ty))
            }
        }
    }

    /// Build the generic instance key from concrete type arguments.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        let mut arguments = Vec::with_capacity(types.len());
        for ty in types {
            let ty = self.lower(*ty)?;
            arguments.push(self.tree.intern_static(mir::Static::Type(ty)));
        }

        Ok(GenericInstanceKey { symbol, arguments })
    }
}

impl<'module> ModuleLowerer<'module> {
    /// Return recursive type lowering over one tree.
    pub(in crate::lower) fn type_lowerer<'lower>(
        &'lower mut self,
        tree: &'lower mut mir::Tree,
        pointer_bytes: u8,
        lifetime_parameters: &'lower LifetimeParameters,
    ) -> TypeLowerer<'lower, 'module> {
        TypeLowerer::new(self, tree, pointer_bytes, lifetime_parameters)
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

        // storage carriers wrap exactly one value argument
        let arguments = self.types(id.module_id)?.type_ids(instance.arguments);
        let Some(argument) = arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "a storage carrier instantiated without its value".to_string(),
            });
        };

        Ok(Some(argument))
    }
}

impl TypeLowerer<'_, '_> {
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
            // reject optional and rest elements
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
        // compose copy over the payloads
        let copy = payloads
            .iter()
            .all(|payload| self.tree.get(*payload).copy(self.tree) == mir::Copy::Yes);
        let copy = match copy {
            true => mir::Copy::Yes,
            false => mir::Copy::No,
        };

        // select enough bits to distinguish every union arm
        let width = payloads.len().next_power_of_two().ilog2().max(1) as u16;
        let discriminant = self.tree.intern_type(mir::Type::Int {
            width,
            is_signed: false,
        });

        // assign each payload its zero-based arm index
        let cases = payloads
            .into_iter()
            .enumerate()
            .map(|(index, ty)| mir::VariantCase {
                discriminant: mir::Constant::UInt {
                    value: index as u128,
                    width,
                },
                ty,
            })
            .collect();
        let variant = mir::Type::Variant {
            discriminant,
            cases,
            copy,
        };

        self.tree.intern_type(variant)
    }

    /// Insert one tuple type with copy composed over its elements.
    pub(in crate::lower) fn insert_tuple(
        &mut self,
        elements: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        // compose copy over the elements
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
