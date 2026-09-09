use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_mir as mir;

use destack_source::ModuleId;

use crate::lower::{GenericInstanceKey, GenericScope, ModuleLowerer};
use crate::{CompilerError, CompilerResult, LowerError};

/// Recursive type lowering into one tree.
pub(in crate::lower) struct TypeLowerer<'lower, 'module> {
    /// The stable module lowering state.
    pub(in crate::lower) lower: &'lower mut ModuleLowerer<'module>,
    /// The tree receiving lowered types.
    pub(in crate::lower) tree: &'lower mut mir::Tree,
    /// The target pointer width in bytes.
    pub(in crate::lower) pointer_bytes: u8,
    /// The polymorphic lifetime parameters available during lowering.
    pub(in crate::lower) scope: &'lower GenericScope,
    /// The heap space receiving reference layers, written by placed forms.
    pub(in crate::lower) space: mir::Space,
    /// The type an interface's own `this` lowers to while its dynamic shape lowers.
    pub(in crate::lower) this_type: Option<mir::LocalNodeId<mir::Type>>,
    /// The compound types being lowered, with reservations for revisits.
    reservations: FxIndexMap<dir::GlobalTypeId, Option<mir::LocalNodeId<mir::Type>>>,
}

impl<'lower, 'module> TypeLowerer<'lower, 'module> {
    /// Create recursive type lowering over one tree.
    fn new(
        lower: &'lower mut ModuleLowerer<'module>,
        tree: &'lower mut mir::Tree,
        scope: &'lower GenericScope,
    ) -> Self {
        let pointer_bytes = lower.pointer_bytes;

        Self {
            lower,
            tree,
            pointer_bytes,
            scope,
            space: mir::Space::Local,
            this_type: None,
            reservations: FxIndexMap::default(),
        }
    }

    /// Return a lower over the same tree under other template parameters.
    pub(in crate::lower) fn under<'nested>(
        &'nested mut self,
        scope: &'nested GenericScope,
    ) -> TypeLowerer<'nested, 'module> {
        TypeLowerer {
            lower: &mut *self.lower,
            tree: &mut *self.tree,
            pointer_bytes: self.pointer_bytes,
            scope,
            space: self.space,
            this_type: self.this_type,
            reservations: FxIndexMap::default(),
        }
    }

    /// Lower one type, reserving an identity for the compound graphs that cycle through it.
    pub(in crate::lower) fn lower(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // lower alias declarations through the nominal they name
        let alias = match self.lower.ty(id)? {
            dir::Type::Reference(reference) => Some((reference.symbol, None)),
            dir::Type::Application(instance) => Some((instance.symbol, Some(instance.arguments))),
            _ => None,
        };
        if let Some((symbol, arguments)) = alias
            && self.lower.alias_form(symbol)?.is_some()
        {
            // read the arguments the alias applies
            let arguments = match arguments {
                Some(list) => self.lower.types(id.module_id)?.type_ids(list).to_vec(),
                None => Vec::new(),
            };

            // reject bare defaulted uses without instantiation arguments
            let is_generic = match self.lower.definition(symbol)?.cloned() {
                Some(definition) => self
                    .lower
                    .definition_is_parameterized(symbol.module_id, &definition)?,
                None => false,
            };
            if is_generic && arguments.is_empty() {
                return Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: "a defaulted generic object alias".to_string(),
                }
                .into());
            }

            return Ok(self.lower_nominal(id)?.value);
        }

        // reserve an identity for anonymous compound graphs, which can cycle through their members
        let compound = matches!(
            self.lower.ty(id)?,
            dir::Type::Union(_)
                | dir::Type::Tuple(_)
                | dir::Type::Slice(_)
                | dir::Type::FixedArray(_)
                | dir::Type::Object(_)
        );
        if compound {
            // name the persistent identity this cyclic graph resolves to
            let name = format!("cycle{}:{}", id.module_id, id.local_id.0);
            let symbol = mir::Symbol::named(id.module_id, self.lower.strings.intern(&name));

            // reuse the identity an earlier walk defined for this cycle
            if let Some(defined) = self.tree.identified_type(symbol)
                && !self.tree.type_is_reserved(defined)
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

            // mark this graph as being lowered
            self.reservations.insert(id, None);
        }

        // lower the family, then define the reservation a revisit created
        let mut lowered = self.lower_family(id);
        if compound
            && let Some(reservation) = self.reservations.swap_remove(&id)
            && let (Ok(result), Some(reserved)) = (&lowered, reservation)
        {
            if !self.tree.type_is_reserved(reserved) {
                return Ok(reserved);
            }
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
        // a dependent of the enclosing signature lowers to its minted parameter
        if let Some(index) = self.scope.dependent_index(id) {
            return Ok(self.tree.intern_type(mir::Type::Parameter { index }));
        }

        match self.lower.ty(id)? {
            // lower nominal instances through their representation
            dir::Type::Application(instance) => {
                if let Some(argument) = self.lower.memory_form_value(id, &instance)? {
                    let value = self.lower(argument)?;

                    return self.insert_storage_form(&instance, value);
                }

                if let Some(lowered) = self.lower_named_symbol(instance.symbol)? {
                    return Ok(lowered);
                }
                let nominal = self.lower_nominal(id)?;

                Ok(nominal.value)
            }
            // lower bare references as their nominal applications
            dir::Type::Reference(reference) => {
                if let Some(lowered) = self.lower_named_symbol(reference.symbol)? {
                    return Ok(lowered);
                }
                let nominal = self.lower_nominal(id)?;

                Ok(nominal.value)
            }
            // lower variants through the union that owns them
            dir::Type::Variant(member) => self.lower(member.owner),
            // reject a memory literal outside an argument position
            dir::Type::Literal(dir::Literal::String(value))
                if dir::Space::from_text(value).is_some()
                    || dir::Access::from_text(value).is_some() =>
            {
                Err(CompilerError::Internal {
                    message: "a memory literal in type position".to_string(),
                })
            }
            // lower a rigid parameter to its index in the enclosing template
            dir::Type::Parameter(parameter) => {
                if let Some(index) = self.scope.parameter_index(parameter) {
                    return Ok(self.tree.intern_type(mir::Type::Parameter { index }));
                }

                // name the template declaring the parameter
                let template = self
                    .lower
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id)
                    .template;
                let declared = self
                    .lower
                    .state(parameter.module_id)?
                    .generics
                    .get_template(template)
                    .symbol;
                let path = match declared {
                    Some(symbol) => self.lower.symbol_path(symbol)?,
                    None => "an anonymous template".to_string(),
                };

                Err(LowerError::Unsupported {
                    anchor: self.lower.module.into(),
                    construct: format!("a generic type of '{path}' outside its template"),
                }
                .into())
            }
            // lower an interface's this to its receiver parameter, a class's to its application
            dir::Type::This => match (self.this_type, self.scope.receiver) {
                (Some(this), _) => Ok(this),
                (None, Some(index)) => Ok(self.tree.intern_type(mir::Type::Parameter { index })),
                (None, None) => Err(CompilerError::Internal {
                    message: "an unresolved contextual this".to_string(),
                }),
            },
            // store unions as variants, the layout niching nullish cases into references
            dir::Type::Union(_) => self.lower_union(id),
            // erase object types that declare signatures behind the dynamic representation
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
            // erase the top constraint behind the dynamic representation
            dir::Type::Unknown => self.lower_dynamic(id),
            // resolve memory forms through the form algebra
            dir::Type::Form(_) => self.lower_form(id, None),
            // lower fat pointer values as managed headers in their referent place
            dir::Type::Slice(slice) => self.lower_fat_reference(id, slice.place),
            dir::Type::Dynamic(dynamic) => self.lower_fat_reference(id, dynamic.place),
            dir::Type::Function(function) => self.lower_fat_reference(id, function.place),
            // project an associated type through the base's witness for its interface
            dir::Type::Member(member) => {
                let member = *self.lower.types(id.module_id)?.member(member);
                let Some(qualifier) = member.qualifier else {
                    return Err(CompilerError::Internal {
                        message: "an unqualified projection reached lowering".to_string(),
                    });
                };
                let dir::StaticKey::Name(name) = member.key else {
                    return Err(CompilerError::Internal {
                        message: "a projection of an indexed member".to_string(),
                    });
                };
                let base = self.lower(member.owner)?;

                self.lower_associated_type(qualifier, name, base)
            }
            // read type algebra outside a signature as an open dynamic value
            dir::Type::Operation(_) => self.lower_open_dynamic(),
            // lower fixed arrays to their inline element storage at a literal or parameter length
            dir::Type::FixedArray(fixed) => {
                let element = self.lower(fixed.element)?;
                let mir::GenericArgument::Value(length) =
                    self.lower_generic_argument(fixed.count)?
                else {
                    return Err(CompilerError::Internal {
                        message: "a fixed array length outside the value domain".to_string(),
                    });
                };
                Ok(self
                    .tree
                    .intern_type(mir::Type::FixedArray { element, length }))
            }
            // lower tuple elements recursively
            dir::Type::Tuple(tuple) => {
                let ids = self.lower.tuple_element_types(id.module_id, &tuple)?;
                let mut elements = Vec::with_capacity(ids.len());
                for element in ids {
                    elements.push(self.lower(element)?);
                }

                Ok(self.insert_tuple(elements))
            }
            // lower a function pointer over its lowered signature
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
                }))
            }
            // lower never to its own uninhabited type
            dir::Type::Never => Ok(self.tree.intern_type(mir::Type::Never)),
            // lower undefined to void
            dir::Type::Undefined | dir::Type::Literal(dir::Literal::Undefined) => {
                Ok(self.tree.intern_type(mir::Type::Void))
            }
            // every other singleton is its own zero-sized type
            dir::Type::Null => Ok(self.lower.singleton_type(self.tree, &dir::Literal::Null)),
            dir::Type::Literal(literal) => Ok(self.lower.singleton_type(self.tree, &literal)),
            // represent a constrained intersection by its one value operand
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .lower
                    .types(id.module_id)?
                    .type_ids(intersection.elements)
                    .to_vec();

                // split interface constraints from the value they constrain
                let mut value = None;
                for element in elements {
                    if self.lower.is_interface_operand(element)? {
                        continue;
                    }
                    if let Some(other) = value.replace(element) {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "an unreduced value intersection of {:?} and {:?}",
                                self.lower.ty(other)?,
                                self.lower.ty(element)?
                            ),
                        });
                    }
                }

                match value {
                    // lower the value operand as the whole representation
                    Some(element) => self.lower(element),
                    // erase an interface conjunction behind the dynamic representation
                    None => self.lower_dynamic(id),
                }
            }
            // a refinement constrains checking alone, its base gives the representation
            dir::Type::Refined(refined) => {
                let base = self.lower.types(id.module_id)?.refined(refined).base;

                self.lower(base)
            }
            // lower every other head through its value representation
            other => self.lower_value_representation(&other),
        }
    }

    /// Lower one fat pointer value as a managed reference in its referent place.
    fn lower_fat_reference(
        &mut self,
        id: dir::GlobalTypeId,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // enter the referent's heap space for the duration of the lowering
        let saved = self.space;
        if let Some(space) = self.lower_space(place)? {
            self.space = space;
        }

        // lower the reference inside that space
        let lowered = self.lower_reference(
            mir::ReferenceKind::Managed,
            mir::Lifetime::empty(),
            mir::Access::Mutable,
            id,
        );
        self.space = saved;

        lowered
    }

    /// Lower one union to the variant over its recorded members, one scalar domain untagged.
    pub(in crate::lower) fn lower_union(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // store a union of one scalar domain untagged as that scalar
        if let Some(literal) = self.lower.scalar_literal_union(id)? {
            return self.lower_value_representation(&literal.widen());
        }

        // lower a single-member union to that member
        let elements = self.lower.union_members(id)?;
        if let [member] = elements.as_slice() {
            return self.lower(*member);
        }
        let mut payloads = Vec::with_capacity(elements.len());
        for element in elements {
            payloads.push(self.lower(element)?);
        }

        Ok(self.insert_union_variant(payloads))
    }

    /// Build one function instance key with the regions its arguments carry erased, the body
    /// verifying once over its parameters.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        receiver: Option<dir::GlobalTypeId>,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        // lower the receiver into a region-erased type argument
        let receiver = match receiver {
            Some(ty) => {
                let ty = self.lower(ty)?;
                let ty = mir::erase_regions(self.tree, ty);
                Some(mir::GenericArgument::Type(mir::TypeId::from(ty)))
            }
            None => None,
        };
        let mut arguments = self.generic_arguments(types)?;
        for argument in &mut arguments {
            match argument {
                mir::GenericArgument::Type(ty) => {
                    *ty = mir::erase_regions(self.tree, *ty);
                }
                mir::GenericArgument::Region { lifetime, .. } => *lifetime = mir::Lifetime::empty(),
                _ => {}
            }
        }

        Ok(GenericInstanceKey {
            symbol,
            receiver,
            arguments,
        })
    }

    /// Lower each argument in its parameter's domain, regions among them.
    pub(in crate::lower) fn generic_arguments(
        &mut self,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<Vec<mir::GenericArgument>> {
        let mut arguments = Vec::with_capacity(types.len());
        for ty in types {
            // local place arguments canonicalize onto the plain declaration
            if self.lower.place_space(*ty)? == Some(dir::Space::Local) {
                continue;
            }

            arguments.push(self.lower_generic_argument(*ty)?);
        }

        Ok(arguments)
    }

    /// Lower one region argument.
    fn lower_region_argument(
        &mut self,
        region: dir::GlobalTypeId,
    ) -> CompilerResult<mir::GenericArgument> {
        let lifetime = self.lower.lower_lifetime(region, self.scope)?;
        let space = match self.lower.ty(region)? {
            dir::Type::Region(pair) => self.lower_space(pair.space)?,
            dir::Type::Parameter(parameter) => self
                .scope
                .parameter_index(parameter)
                .map(mir::Space::Parameter),
            _ => None,
        };

        Ok(mir::GenericArgument::Region {
            lifetime,
            space: space.unwrap_or(self.space),
        })
    }

    /// Lower one generic argument into its parameter's domain.
    pub(in crate::lower) fn lower_generic_argument(
        &mut self,
        argument: dir::GlobalTypeId,
    ) -> CompilerResult<mir::GenericArgument> {
        if self.lower.type_is_lifetime(argument)? {
            return self.lower_region_argument(argument);
        }
        match self.lower.ty(argument)? {
            dir::Type::Literal(dir::Literal::Integer(value)) => {
                let value = self.tree.intern_static(mir::Static::Integer(value));

                Ok(mir::GenericArgument::Value(value))
            }
            dir::Type::Literal(dir::Literal::String(value))
                if let Some(space) = dir::Space::from_text(value) =>
            {
                Ok(mir::GenericArgument::Space(ModuleLowerer::mir_space(space)))
            }
            dir::Type::Literal(dir::Literal::String(value))
                if dir::Access::from_text(value).is_some() =>
            {
                Ok(mir::GenericArgument::Access(
                    self.lower.borrow_access(argument)?,
                ))
            }
            dir::Type::Parameter(parameter)
                if let Some(index) = self.scope.parameter_index(parameter) =>
            {
                let binding = self
                    .lower
                    .state(parameter.module_id)?
                    .generics
                    .get_parameter(parameter.local_id);
                Ok(match binding.memory_parameter() {
                    Some(dir::MemoryParameter::Space | dir::MemoryParameter::Place) => {
                        mir::GenericArgument::Space(mir::Space::Parameter(index))
                    }
                    Some(dir::MemoryParameter::Access) => {
                        mir::GenericArgument::Access(mir::Access::Parameter(index))
                    }
                    Some(dir::MemoryParameter::Ownership | dir::MemoryParameter::Region) => {
                        return Err(LowerError::Unsupported {
                            anchor: self.lower.module.into(),
                            construct: "an ownership or region parameter argument".to_string(),
                        }
                        .into());
                    }
                    None if binding.is_const => {
                        let value = self.tree.intern_static(mir::Static::Parameter(index));

                        mir::GenericArgument::Value(value)
                    }
                    None => {
                        let ty = self.tree.intern_type(mir::Type::Parameter { index });

                        mir::GenericArgument::Type(mir::TypeId::from(ty))
                    }
                })
            }
            _ => {
                let ty = self.lower(argument)?;

                Ok(mir::GenericArgument::Type(mir::TypeId::from(ty)))
            }
        }
    }

    /// Lower one place or space argument to its space, an induced or unbound one to the ambient.
    pub(in crate::lower) fn lower_space(
        &mut self,
        place: dir::GlobalTypeId,
    ) -> CompilerResult<Option<mir::Space>> {
        Ok(match self.lower.ty(place)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                dir::Space::from_text(value).map(ModuleLowerer::mir_space)
            }
            dir::Type::Parameter(parameter) => match self.scope.parameter_index(parameter) {
                Some(index) => Some(mir::Space::Parameter(index)),
                None => self.lower.place_space(place)?.map(ModuleLowerer::mir_space),
            },
            _ => None,
        })
    }

    /// Lower one access argument to its access, a parameter to its own access.
    pub(in crate::lower) fn lower_access(
        &mut self,
        access: dir::GlobalTypeId,
    ) -> CompilerResult<mir::Access> {
        self.lower.access_in(access, self.scope)
    }
}

impl<'module> ModuleLowerer<'module> {
    /// Return recursive type lowering over one tree.
    pub(in crate::lower) fn type_lowerer<'lower>(
        &'lower mut self,
        tree: &'lower mut mir::Tree,
        scope: &'lower GenericScope,
    ) -> TypeLowerer<'lower, 'module> {
        TypeLowerer::new(self, tree, scope)
    }

    /// Return the value argument when one instance applies a memory form item.
    pub(in crate::lower) fn memory_form_value(
        &mut self,
        id: dir::GlobalTypeId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // require a memory form language item
        let item = self.language_item(instance.symbol);
        if !matches!(
            item,
            Some(dir::LanguageItem::MaybeUninit | dir::LanguageItem::ManuallyDrop)
        ) {
            return Ok(None);
        }

        // take the single value argument the item wraps
        let arguments = self.types(id.module_id)?.type_ids(instance.arguments);
        let Some(argument) = arguments.first().copied() else {
            return Err(CompilerError::Internal {
                message: "a memory form item instantiated without its value".to_string(),
            });
        };

        Ok(Some(argument))
    }
}

impl TypeLowerer<'_, '_> {
    /// Wrap one lowered value type in its memory form.
    fn insert_storage_form(
        &mut self,
        instance: &dir::GenericApplication,
        value: mir::LocalNodeId<mir::Type>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // select the memory form node the item names
        let ty = match self.lower.language_item(instance.symbol) {
            Some(dir::LanguageItem::MaybeUninit) => mir::Type::Uninit { value },
            Some(dir::LanguageItem::ManuallyDrop) => mir::Type::ManuallyDrop { value },
            _ => {
                return Err(CompilerError::Internal {
                    message: "a memory form without its language item".to_string(),
                });
            }
        };

        Ok(self.tree.intern_type(ty))
    }
}

impl ModuleLowerer<'_> {
    /// Return the element types of one plain tuple in position order.
    pub(in crate::lower) fn tuple_element_types(
        &mut self,
        module: ModuleId,
        tuple: &dir::TupleType,
    ) -> CompilerResult<Vec<dir::GlobalTypeId>> {
        // collect the plain element types in position order
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
    /// Intern the variant canonical over one union's payloads.
    pub(in crate::lower) fn insert_union_variant(
        &mut self,
        payloads: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.tree.intern_union(payloads, mir::Copy::Yes)
    }

    /// Insert one tuple type with copy composed over its elements.
    fn insert_tuple(
        &mut self,
        elements: Vec<mir::LocalNodeId<mir::Type>>,
    ) -> mir::LocalNodeId<mir::Type> {
        self.tree.intern_type(mir::Type::Tuple {
            elements: elements.into_iter().collect(),
        })
    }

    /// Lower the bounds one constraint names: interface constraint types and memory literals.
    pub(in crate::lower) fn lower_bounds(
        &mut self,
        constraint: dir::GlobalTypeId,
    ) -> CompilerResult<Vec<mir::TypeId>> {
        let symbol = match self.lower.ty(constraint)? {
            dir::Type::Application(application) => Some(application.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            // bound by the interfaces every arm of a union shares, arms copying as closed types or
            // through their interfaces bounding by copy
            dir::Type::Union(union) => {
                let elements = self
                    .lower
                    .types(constraint.module_id)?
                    .type_ids(union.elements)
                    .to_vec();
                let copy = self.lower.language_item_symbol(dir::LanguageItem::Copy)?;
                let mut shared: Option<Vec<(dir::GlobalSymbolId, dir::GlobalTypeId)>> = None;
                let mut copies = true;
                for element in elements {
                    let ancestors = self.lower.interface_ancestors(element)?;
                    copies &= match ancestors.is_empty() {
                        true => {
                            let lowered = self.lower(element)?;
                            self.tree.get(lowered).copy(self.tree) == mir::Copy::Yes
                        }
                        false => ancestors.iter().any(|(symbol, _)| *symbol == copy),
                    };
                    shared = Some(match shared {
                        None => ancestors,
                        Some(shared) => shared
                            .into_iter()
                            .filter(|(symbol, _)| {
                                ancestors.iter().any(|(other, _)| other == symbol)
                            })
                            .collect(),
                    });
                }
                let mut shared = shared.unwrap_or_default();
                if copies && !shared.iter().any(|(symbol, _)| *symbol == copy) {
                    shared.push((copy, self.lower.symbol_type(copy)?));
                }
                let mut bounds = Vec::new();
                for (_, ancestor) in shared {
                    bounds.extend(self.lower_bounds(ancestor)?);
                }

                return Ok(bounds);
            }
            // bound by every operand of an intersection
            dir::Type::Intersection(intersection) => {
                let elements = self
                    .lower
                    .types(constraint.module_id)?
                    .type_ids(intersection.elements)
                    .to_vec();
                let mut bounds = Vec::new();
                for element in elements {
                    bounds.extend(self.lower_bounds(element)?);
                }

                return Ok(bounds);
            }
            // bound a memory parameter by its literal domain
            dir::Type::Literal(dir::Literal::String(value))
                if dir::Space::from_text(value).is_some()
                    || dir::Access::from_text(value).is_some() =>
            {
                let bound = self.lower(constraint)?;

                return Ok(vec![mir::TypeId::from(bound)]);
            }
            _ => None,
        };

        // bound by what an alias names
        if let Some(symbol) = symbol
            && let Some(dir::Definition::TypeAlias(_)) = self.lower.definition(symbol)?
        {
            let value = self.lower.symbol_type(symbol)?;

            return self.lower_bounds(value);
        }

        // bound a type parameter by the interface's constraint type
        let is_interface = symbol.is_some_and(|symbol| {
            matches!(
                self.lower.definition(symbol),
                Ok(Some(dir::Definition::Interface(_)))
            )
        });
        if !is_interface {
            return Ok(Vec::new());
        }
        let bound = self.lower_nominal(constraint)?.storage;

        Ok(vec![mir::TypeId::from(bound)])
    }

    /// Lower the type a named symbol stands for outside a nominal declaration.
    fn lower_named_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<mir::LocalNodeId<mir::Type>>> {
        let primitive = match self.lower.language_item(symbol) {
            Some(dir::LanguageItem::String) => Some(dir::PrimitiveType::String),
            Some(dir::LanguageItem::BigInt) => Some(dir::PrimitiveType::Bigint),
            _ => None,
        };
        if let Some(primitive) = primitive {
            return Ok(Some(
                self.lower_value_representation(&dir::Type::Primitive(primitive))?,
            ));
        }
        if self.lower.definition(symbol)?.is_none() {
            let ty = self.lower.symbol_type(symbol)?;

            return Ok(Some(self.lower(ty)?));
        }

        Ok(None)
    }

    /// Lower one value type through its representation class, else its scalar form.
    pub(in crate::lower) fn lower_value_representation(
        &mut self,
        ty: &dir::Type,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // lower reference primitives through their representation classes
        if let Some(item) = ModuleLowerer::representation_item(ty) {
            let symbol = self.lower.language_item_symbol(item)?;
            let source = self.lower.symbol_type(symbol)?;

            return Ok(self.lower_nominal(source)?.value);
        }

        // fall back to the scalar families
        let ty = self.lower.scalar_type(ty)?;

        Ok(self.tree.intern_type(ty))
    }
}
