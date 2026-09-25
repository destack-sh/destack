use smallvec::SmallVec;
use tspp_core::{FxIndexSet, ensure_sufficient_stack};
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::auto::DecisionKey;
use crate::sema::{
    CheckState, ObligationCheck, ObligationFailure, Origin, Relation, SharedStorageObligation,
};
use crate::{CompilerError, CompilerResult};

/// The representation interface being checked.
#[derive(Debug, Clone, Copy)]
enum RepresentationCheck {
    /// Every stored value must have a fixed representation.
    Concrete {
        /// The `Concrete` interface that proves generic slots.
        interface: dir::GlobalTypeId,
    },
    /// Inline storage must terminate.
    Finite,
    /// Safe references reachable from shared storage must remain shared.
    Shared {
        /// Report fields when checking their declaration.
        use_fields: bool,
    },
}

/// One invalid representation found while walking stored children.
enum RepresentationFailure {
    /// One stored value has no fixed representation.
    Abstract,
    /// Inline storage contains itself.
    Circular(dir::GlobalNodeIdAny),
    /// Shared storage keeps a safe local reference.
    LocalReference(dir::GlobalNodeIdAny),
}

impl CheckState<'_> {
    /// Return whether one type has a fixed storage representation.
    pub(in crate::sema) fn is_concrete(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // reuse the decision recorded by an earlier walk
        if self.has_decided_representation(origin, ty, dir::AutoInterface::Concrete)? {
            return Ok(true);
        }

        // walk the stored representation for abstract slots
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
            self.commit_representation(origin, ty, dir::AutoInterface::Concrete)?;
        }

        Ok(failure.is_none())
    }

    /// Return whether one type's values may live in shared space.
    pub(in crate::sema) fn is_shared_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // reuse the decision recorded by an earlier walk
        if self.has_decided_representation(origin, ty, dir::AutoInterface::SharedSafe)? {
            return Ok(true);
        }

        // reject intrinsically local declarations
        let value = self.strip_form(origin, ty)?;
        let symbol = match self.ty(value)? {
            dir::Type::Application(instance) => Some(instance.symbol),
            _ => None,
        };
        if let Some(symbol) = symbol
            && (self.nominal_space(symbol)? == Some(dir::Space::Local)
                || self.declares_negative(symbol, dir::AutoInterface::SharedSafe)?)
        {
            return Ok(false);
        }

        // walk the stored representation for shared containment
        let source = self.origin_source(origin)?;
        let mut visited = FxIndexSet::default();
        let failure = self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Shared { use_fields: false },
            &mut visited,
        )?;
        if failure.is_none() {
            self.commit_representation(origin, ty, dir::AutoInterface::SharedSafe)?;
        }

        Ok(failure.is_none())
    }

    /// Check one shared binding obligation: the stored type may live in shared space.
    pub(in crate::sema) fn check_shared_storage(
        &mut self,
        origin: Origin,
        obligation: &SharedStorageObligation,
    ) -> CompilerResult<ObligationCheck> {
        // wait for the binding type to solve
        let stalls = self.collect_open_variables([obligation.ty])?;
        if !stalls.is_empty() {
            return Ok(ObligationCheck::Ambiguous(stalls));
        }

        match self.is_shared_safe(origin, obligation.ty)? {
            true => Ok(ObligationCheck::holds()),
            false => Ok(ObligationCheck::fail(
                ObligationFailure::LocalReferenceInSharedStorage {
                    source: obligation.source,
                },
            )),
        }
    }

    /// Check the finite and shared safety properties of one stored type.
    pub(in crate::sema) fn check_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<ObligationCheck> {
        // reuse the decision recorded by an earlier walk
        if let Some(key) = self.storage_key(origin, ty)?
            && self.storables.contains(&key)
        {
            return Ok(ObligationCheck::holds());
        }

        // walk the stored representation for inline cycles
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
                if self.type_space(ty)? != Some(dir::Space::Shared) {
                    self.commit_storage(origin, ty)?;

                    return Ok(ObligationCheck::holds());
                }
                let chain = self.form_chain(origin, ty)?;
                let use_fields = match self.ty(chain.base())? {
                    // require a declaration in this module to name the source site
                    dir::Type::Application(instance)
                        if self.is_own_module(instance.symbol.module_id) =>
                    {
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
                    RepresentationCheck::Shared { use_fields },
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
                // prove storage outside the field's declaration site
                if !is_declaration_site {
                    self.commit_storage(origin, ty)?;
                }

                return Ok(ObligationCheck::holds());
            }
        };

        Ok(ObligationCheck::fail(failure))
    }

    /// Return whether one type already proved a representation interface.
    fn has_decided_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        let Some(key) = self.representation_key(origin, ty, interface)? else {
            return Ok(false);
        };

        Ok(self.conformances.get(&key) == Some(&true))
    }

    /// Remember one decided representation interface.
    fn commit_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<()> {
        if let Some(key) = self.representation_key(origin, ty, interface)? {
            self.conformances.insert(key, true);
        }

        Ok(())
    }

    /// Remember one decided storable representation.
    fn commit_storage(&mut self, origin: Origin, ty: dir::GlobalTypeId) -> CompilerResult<()> {
        if let Some(key) = self.storage_key(origin, ty)? {
            self.storables.insert(key);
        }

        Ok(())
    }

    /// Key one decided storable representation by type identity, for resolved types only.
    fn storage_key(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<(dir::GlobalTypeId, Option<dir::GlobalGenericTemplateId>)>> {
        // decide storability once over variable-free types under the assuming template
        let flags = self.type_flags(ty)?;
        if flags.has_variable() {
            return Ok(None);
        }

        Ok(self
            .decision_scope(origin, &[ty])?
            .map(|assumes| (ty, assumes)))
    }

    /// Key one representation interface by type identity, for resolved types only.
    fn representation_key(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Option<DecisionKey>> {
        // decide the interface once over variable-free types under the assuming template
        let flags = self.type_flags(ty)?;
        if flags.has_variable() {
            return Ok(None);
        }

        Ok(self
            .decision_scope(origin, &[ty])?
            .map(|assumes| DecisionKey {
                ty,
                interface,
                assumes,
            }))
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
        // read the space a shared check stores the type in
        if let RepresentationCheck::Shared { .. } = check {
            let chain = self.form_chain(origin, ty)?;
            let ownership = self.form_ownership(origin, &chain)?;

            // skip raw pointers, they are an explicit unchecked escape
            if ownership == Some(dir::Ownership::Raw) {
                return Ok(None);
            }

            // reject a reference into local storage
            let space = match chain.region() {
                Some(region) => match self.region_space(region)? {
                    Some(space) => self.literal_space(space)?,
                    None => None,
                },
                None => self.type_space(chain.base())?,
            };

            // hold only references proven outside local space
            let is_local = !matches!(space, Some(dir::Space::Shared | dir::Space::Constant));
            if is_local && self.form_is_reference(origin, &chain)? {
                return Ok(Some(RepresentationFailure::LocalReference(source)));
            }
            ty = chain.base();
        }

        // reuse the decisions the space-free walks recorded
        match check {
            RepresentationCheck::Concrete { .. } => {
                if self.has_decided_representation(origin, ty, dir::AutoInterface::Concrete)? {
                    return Ok(None);
                }
            }
            RepresentationCheck::Finite => {
                if let Some(key) = self.storage_key(origin, ty)?
                    && self.storables.contains(&key)
                {
                    return Ok(None);
                }
            }
            RepresentationCheck::Shared { .. } => {}
        }

        // treat an open numeric variable like the scalar it settles to
        let ty = self.shallow_resolve(ty)?;
        if self.numeric_root(ty)?.is_some() {
            return Ok(None);
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
                    self.commit_representation(origin, ty, dir::AutoInterface::Concrete)?;
                }
                RepresentationCheck::Finite => {
                    self.commit_storage(origin, ty)?;
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
                let is_proven = self
                    .decide_relation(origin, Relation::Subtype, ty, interface)?
                    .holds();

                return Ok((!is_proven).then_some(RepresentationFailure::Abstract));
            }
            // fail abstract type expressions, they select no runtime representation
            dir::Type::Unknown
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
                // bound inline layout at owned indirection
                dir::Form::Owned if matches!(check, RepresentationCheck::Finite) => {
                    return Ok(None);
                }
                dir::Form::Owned | dir::Form::Readonly => {
                    SmallVec::from_slice(&[(form.value, source)])
                }
                // accept managed storage and borrows into heap blocks
                dir::Form::Borrowed(_) | dir::Form::Raw => {
                    return Ok(None);
                }
            },
            // walk the element of a bare slice for the shared reachability check
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
                .object_properties(owner, shape.properties)?
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
        // walk the element of a vector inline, and stop at every other intrinsic
        if let Some(item) = self.language_item(instance.symbol)? {
            if item == dir::LanguageItem::Vector
                && let Some(element) = self.type_ids(owner, instance.arguments)?.first().copied()
            {
                return self.representation_failure(origin, element, source, check, visited);
            }

            return Ok(None);
        }

        // collect the storage the declaration holds
        let storage = match self.definition(instance.symbol)?.as_deref() {
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
