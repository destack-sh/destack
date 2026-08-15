use destack_core::{FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, ObligationCheck, ObligationFailure, Origin, Relation, Scope};
use crate::{CompilerError, CompilerResult};

/// The representation interface being checked.
#[derive(Debug, Clone, Copy)]
enum RepresentationCheck {
    /// Every stored value must have a fixed representation.
    Concrete {
        /// The `Concrete` interface used to prove generic slots.
        interface: dir::GlobalTypeId,
    },
    /// Inline storage must terminate.
    Finite,
    /// Safe references reachable from shared storage must remain shared.
    Shared {
        /// The containing value's concrete place.
        place: dir::GlobalTypeId,
        /// Report fields when checking their own declaration.
        use_fields: bool,
    },
}

/// One invalid representation found while walking stored children.
enum RepresentationFailure {
    /// One stored value has no fixed representation.
    Abstract,
    /// Inline storage contains itself.
    Circular(dir::GlobalNodeIdAny),
    /// Shared storage retains a safe local reference.
    LocalReference(dir::GlobalNodeIdAny),
}

impl CheckState<'_> {
    /// Decide whether one type has a fixed storage representation.
    pub(in crate::sema) fn satisfies_concrete(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.is_representation_proven(origin, ty, dir::AutoInterface::Concrete)? {
            return Ok(true);
        }

        let source = self.origin_source(origin)?;
        let interface = self.language_type(dir::LanguageItem::Concrete, &[])?;
        let mut visited = FxIndexSet::default();
        let failure = self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Concrete { interface },
            &mut visited,
        )?;
        if failure.is_none() {
            self.prove_representation(origin, ty, dir::AutoInterface::Concrete)?;
        }

        Ok(failure.is_none())
    }

    /// Decide whether one type's values may live in shared space.
    pub(in crate::sema) fn satisfies_shared_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        if self.is_representation_proven(origin, ty, dir::AutoInterface::SharedSafe)? {
            return Ok(true);
        }

        // reject intrinsically local declarations
        let value = self.strip_form(origin, ty)?;
        let symbol = match self.ty(value)? {
            dir::Type::Application(instance) => Some(instance.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        };
        if let Some(symbol) = symbol
            && self.nominal_space(symbol)? == Some(dir::Space::Local)
        {
            return Ok(false);
        }

        // walk the stored representation for shared containment
        let source = self.origin_source(origin)?;
        let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(dir::Space::Shared),
        )))?;
        let mut visited = FxIndexSet::default();
        let failure = self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Shared {
                place,
                use_fields: false,
            },
            &mut visited,
        )?;
        if failure.is_none() {
            self.prove_representation(origin, ty, dir::AutoInterface::SharedSafe)?;
        }

        Ok(failure.is_none())
    }

    /// Check the finite and shared-safety properties of one stored type.
    pub(in crate::sema) fn check_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<ObligationCheck> {
        if let Some(key) = self.storage_key(origin, ty)?
            && self.storable.contains(&key)
        {
            return Ok(ObligationCheck::holds());
        }

        let source = self.origin_source(origin)?;
        let mut visited = FxIndexSet::default();
        let failure = self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Finite,
            &mut visited,
        )?;

        // check shared reachability only from concretely shared roots
        let mut is_declaration_site = false;
        let failure = match failure {
            Some(failure) => Some(failure),
            None => {
                let chain = self.form_chain(origin, ty)?;
                let Some(place) = chain.place() else {
                    self.prove_storage(origin, ty)?;

                    return Ok(ObligationCheck::holds());
                };
                if self.place_space(place)? != Some(dir::Space::Shared) {
                    self.prove_storage(origin, ty)?;

                    return Ok(ObligationCheck::holds());
                }
                let use_fields = match self.ty(chain.base())? {
                    dir::Type::Application(instance) => {
                        let declaration = self
                            .module(instance.symbol.module_id)
                            .symbol_declaration_node(instance.symbol.local_id)?
                            .into_global(instance.symbol.module_id);

                        declaration == source
                    }
                    _ => false,
                };
                is_declaration_site = use_fields;
                visited.clear();

                self.representation_failure(
                    origin,
                    ty,
                    source,
                    RepresentationCheck::Shared { place, use_fields },
                    &mut visited,
                )?
            }
        };
        let failure = match failure {
            Some(RepresentationFailure::Abstract) => {
                return Err(CompilerError::Internal {
                    message: "representation validation entered a concrete marker check".into(),
                });
            }
            Some(RepresentationFailure::Circular(source)) => {
                ObligationFailure::CircularType { source }
            }
            Some(RepresentationFailure::LocalReference(source)) => {
                ObligationFailure::LocalReferenceInSharedStorage { source }
            }
            None => {
                // prove storage away from the field's own declaration site
                if !is_declaration_site {
                    self.prove_storage(origin, ty)?;
                }

                return Ok(ObligationCheck::holds());
            }
        };

        Ok(ObligationCheck::fail(failure))
    }

    /// Return whether one type already proved a representation interface.
    fn is_representation_proven(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        let Some(key) = self.representation_key(origin, ty, interface)? else {
            return Ok(false);
        };

        Ok(self.conforms.get(&key) == Some(&true))
    }

    /// Record one proven representation interface.
    fn prove_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<()> {
        if let Some(key) = self.representation_key(origin, ty, interface)? {
            self.conforms.insert(key, true);
        }

        Ok(())
    }

    /// Record one proven storable representation.
    fn prove_storage(&mut self, origin: Origin, ty: dir::GlobalTypeId) -> CompilerResult<()> {
        if let Some(key) = self.storage_key(origin, ty)? {
            self.storable.insert(key);
        }

        Ok(())
    }

    /// Key one proven storable representation by its assuming scope, for closed types only.
    fn storage_key(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, Scope)>> {
        let flags = self.type_flags(ty)?;
        if flags.has_variable() {
            return Ok(None);
        }
        let scope = if flags.has_parameter() || flags.has_this() {
            self.assuming_scope(origin)?
        } else {
            None
        };

        Ok(Some((ty, scope)))
    }

    /// Key one representation interface by its assuming scope, for closed types only.
    fn representation_key(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, dir::AutoInterface, Scope)>> {
        let flags = self.type_flags(ty)?;
        if flags.has_variable() {
            return Ok(None);
        }
        let scope = if flags.has_parameter() || flags.has_this() {
            self.assuming_scope(origin)?
        } else {
            None
        };

        Ok(Some((ty, interface, scope)))
    }

    /// Return the first invalid stored representation beneath one type.
    fn representation_failure(
        &mut self,
        origin: Origin,
        mut ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        check: RepresentationCheck,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<RepresentationFailure>> {
        // project the type into the place a shared check stores it
        let check = match check {
            RepresentationCheck::Concrete { interface } => {
                RepresentationCheck::Concrete { interface }
            }
            RepresentationCheck::Finite => RepresentationCheck::Finite,
            RepresentationCheck::Shared { place, use_fields } => {
                ty = self.resolve_relative_place(origin, ty, place)?;
                let chain = self.form_chain(origin, ty)?;
                let place = chain.place().unwrap_or(place);
                let ownership = self.form_ownership(origin, &chain)?;

                // skip raw pointers, they are an explicit unchecked escape
                if ownership == Some(dir::Ownership::Raw) {
                    return Ok(None);
                }
                let is_local = self.place_space(place)? == Some(dir::Space::Local);
                if is_local && self.form_is_reference(origin, &chain)? {
                    return Ok(Some(RepresentationFailure::LocalReference(source)));
                }
                ty = chain.base();

                RepresentationCheck::Shared { place, use_fields }
            }
        };

        // reuse per-node proofs, since concrete and finite walks are place independent
        match check {
            RepresentationCheck::Concrete { .. } => {
                if self.is_representation_proven(origin, ty, dir::AutoInterface::Concrete)? {
                    return Ok(None);
                }
            }
            RepresentationCheck::Finite => {
                if let Some(key) = self.storage_key(origin, ty)?
                    && self.storable.contains(&key)
                {
                    return Ok(None);
                }
            }
            RepresentationCheck::Shared { .. } => {}
        }

        // fail inline storage on a cycle and stop the shared walk
        if !visited.insert(ty) {
            let failure = match check {
                RepresentationCheck::Concrete { .. } => None,
                RepresentationCheck::Finite => Some(RepresentationFailure::Circular(source)),
                RepresentationCheck::Shared { .. } => None,
            };

            return Ok(failure);
        }
        let failure = ensure_sufficient_stack(|| {
            self.representation_child_failure(origin, ty, source, check, visited)
        });
        visited.swap_remove(&ty);

        // prove the walked subtree once its children hold
        if let Ok(None) = &failure {
            match check {
                RepresentationCheck::Concrete { .. } => {
                    self.prove_representation(origin, ty, dir::AutoInterface::Concrete)?;
                }
                RepresentationCheck::Finite => {
                    self.prove_storage(origin, ty)?;
                }
                RepresentationCheck::Shared { .. } => {}
            }
        }

        failure
    }

    /// Check every stored child of one reduced type.
    fn representation_child_failure(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        check: RepresentationCheck,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<RepresentationFailure>> {
        let owner = ty.module_id;
        let slots: SmallVec<[(dir::GlobalTypeId, dir::GlobalNodeIdAny); 4]> = match self.ty(ty)? {
            // generic slots require an explicit concrete bound
            dir::Type::Parameter(_) => {
                let RepresentationCheck::Concrete { interface } = check else {
                    return Ok(None);
                };
                let holds = self
                    .evaluate_relation(origin, Relation::Satisfies, ty, interface)?
                    .holds();

                return Ok((!holds).then_some(RepresentationFailure::Abstract));
            }
            // skip abstract type expressions, they select no runtime representation
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Erased(_)
            | dir::Type::Reference(_)
            | dir::Type::This
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Intersection(_)
                if matches!(check, RepresentationCheck::Concrete { .. }) =>
            {
                return Ok(Some(RepresentationFailure::Abstract));
            }
            // follow direct forms, which shared checking already stripped
            dir::Type::Form(form) => match form.form {
                // bound inline layout at owned indirection, like a managed handle
                dir::Form::Owned if matches!(check, RepresentationCheck::Finite) => {
                    return Ok(None);
                }
                dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                    SmallVec::from_slice(&[(form.value, source)])
                }
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    return Ok(None);
                }
            },
            dir::Type::Array(array) if matches!(check, RepresentationCheck::Shared { .. }) => {
                SmallVec::from_slice(&[(array.element, source)])
            }
            dir::Type::Slice(slice) if matches!(check, RepresentationCheck::Shared { .. }) => {
                SmallVec::from_slice(&[(slice.element, source)])
            }
            dir::Type::FixedArray(array) => SmallVec::from_slice(&[(array.element, source)]),
            dir::Type::Tuple(tuple) => self
                .tuple_elements(owner, tuple.elements)?
                .iter()
                .map(|element| (element.ty, source))
                .collect(),
            dir::Type::Object(shape) => self
                .shape_properties(owner, shape.properties)?
                .iter()
                .flat_map(|field| field.access.types().map(move |ty| (ty, source)))
                .collect(),
            dir::Type::Union(union) => self
                .type_ids(owner, union.elements)?
                .iter()
                .map(|element| (*element, source))
                .collect(),
            dir::Type::Variant(member) => SmallVec::from_slice(&[(member.owner, source)]),
            dir::Type::Application(instance) => {
                return self.representation_instance_failure(
                    origin, owner, &instance, source, check, visited,
                );
            }
            _ => return Ok(None),
        };

        self.representation_slot_failure(origin, &slots, check, visited)
    }

    /// Check the substituted storage of one applied declaration.
    fn representation_instance_failure(
        &mut self,
        origin: Origin,
        owner: ModuleId,
        instance: &dir::GenericApplication,
        source: dir::GlobalNodeIdAny,
        check: RepresentationCheck,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<RepresentationFailure>> {
        // vectors store their element inline, other opaque intrinsics store no visible children
        if let Some(item) = self.language_item(instance.symbol)? {
            if item == dir::LanguageItem::Vector
                && let Some(element) = self.type_ids(owner, instance.arguments)?.first().copied()
            {
                return self.representation_failure(origin, element, source, check, visited);
            }

            return Ok(None);
        }

        let storage = match self.definition(instance.symbol)?.cloned() {
            Some(dir::Definition::Newtype(definition)) => {
                SmallVec::<[_; 4]>::from_slice(&[(definition.backing, source)])
            }
            Some(dir::Definition::Interface(_))
                if matches!(check, RepresentationCheck::Concrete { .. }) =>
            {
                return Ok(Some(RepresentationFailure::Abstract));
            }
            // collect struct fields always and class fields only for reachability
            Some(definition @ (dir::Definition::Struct(_) | dir::Definition::Class(_)))
                if matches!(definition, dir::Definition::Struct(_))
                    || matches!(check, RepresentationCheck::Shared { .. }) =>
            {
                let use_source = !instance.arguments.is_empty();
                let mut fields = SmallVec::new();
                for member in definition.members() {
                    let dir::DefinitionMember::Field(field) = member else {
                        continue;
                    };
                    if field.space != dir::MemberSpace::Instance {
                        continue;
                    }
                    let Some(ty) = self.definition_member_type(member)? else {
                        continue;
                    };
                    let field_source = match check {
                        RepresentationCheck::Concrete { .. } => source,
                        RepresentationCheck::Finite if !use_source => field.source,
                        RepresentationCheck::Shared {
                            use_fields: true, ..
                        } => field.source,
                        _ => source,
                    };
                    fields.push((ty, field_source));
                }

                fields
            }
            _ => return Ok(None),
        };

        // substitute the applied arguments once before checking stored children
        let substitution = self.instance_substitution(owner, instance)?;
        let mut slots = SmallVec::<[_; 4]>::new();
        for (ty, source) in storage {
            slots.push((self.substitute_type(ty, &substitution)?, source));
        }

        self.representation_slot_failure(origin, &slots, check, visited)
    }

    /// Return the first invalid representation in a list of stored slots.
    fn representation_slot_failure(
        &mut self,
        origin: Origin,
        slots: &[(dir::GlobalTypeId, dir::GlobalNodeIdAny)],
        check: RepresentationCheck,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Option<RepresentationFailure>> {
        for (ty, source) in slots {
            let failure = self.representation_failure(origin, *ty, *source, check, visited)?;
            if failure.is_some() {
                return Ok(failure);
            }
        }

        Ok(None)
    }
}
