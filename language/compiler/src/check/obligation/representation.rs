use destack_core::{FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{
    Answer, CheckState, ObligationCheck, ObligationFailure, Origin, Relation, answer,
};
use crate::{CompilerError, CompilerResult};

/// The representation property being checked.
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
    pub(in crate::check) fn satisfies_concrete(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let source = self.origin_source(origin)?;
        let interface = self.language_type(dir::LanguageItem::Concrete, &[])?;
        let mut visited = FxIndexSet::default();
        let failure = answer!(self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Concrete { interface },
            &mut visited,
        )?);

        Ok(Answer::Ready(failure.is_none()))
    }

    /// Decide whether one type's values may live in shared space.
    pub(in crate::check) fn satisfies_shared_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // reject intrinsically local declarations
        let value = answer!(self.strip_form(origin, ty)?);
        let value = answer!(self.reduce_type_head(origin, value)?);
        let symbol = match self.ty(value)? {
            dir::Type::Application(instance) => Some(instance.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        };
        if let Some(symbol) = symbol
            && self.nominal_space(symbol)? == Some(dir::Space::Local)
        {
            return Ok(Answer::Ready(false));
        }

        // walk the stored representation for shared containment
        let source = self.origin_source(origin)?;
        let place = self.intern_type(dir::Type::Memory(dir::MemoryLiteral::Place(
            dir::Place::Space(dir::Space::Shared),
        )))?;
        let mut visited = FxIndexSet::default();
        let failure = answer!(self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Shared {
                place,
                use_fields: false,
            },
            &mut visited,
        )?);

        Ok(Answer::Ready(failure.is_none()))
    }

    /// Check the finite and shared-safety properties of one stored type.
    pub(in crate::check) fn check_representation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<ObligationCheck>> {
        let source = self.origin_source(origin)?;
        let mut visited = FxIndexSet::default();
        let failure = answer!(self.representation_failure(
            origin,
            ty,
            source,
            RepresentationCheck::Finite,
            &mut visited,
        )?);

        // check shared reachability only from concretely shared roots
        let failure = match failure {
            Some(failure) => Some(failure),
            None => {
                let chain = self.form_chain(origin, ty)?;
                let Some(place) = chain.place() else {
                    return Ok(Answer::Ready(ObligationCheck::holds()));
                };
                if self.place_space(place)? != Some(dir::Space::Shared) {
                    return Ok(Answer::Ready(ObligationCheck::holds()));
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
                visited.clear();

                answer!(self.representation_failure(
                    origin,
                    ty,
                    source,
                    RepresentationCheck::Shared { place, use_fields },
                    &mut visited,
                )?)
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
            None => return Ok(Answer::Ready(ObligationCheck::holds())),
        };

        Ok(Answer::Ready(ObligationCheck::fail(failure)))
    }

    /// Return the first invalid stored representation beneath one type.
    fn representation_failure(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        source: dir::GlobalNodeIdAny,
        check: RepresentationCheck,
        visited: &mut FxIndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<Option<RepresentationFailure>>> {
        let mut ty = answer!(self.reduce_type_head(origin, ty)?);
        let check = match check {
            RepresentationCheck::Concrete { interface } => {
                RepresentationCheck::Concrete { interface }
            }
            RepresentationCheck::Finite => RepresentationCheck::Finite,
            RepresentationCheck::Shared { place, use_fields } => {
                ty = answer!(self.resolve_relative_place(origin, ty, place)?);
                let chain = self.form_chain(origin, ty)?;
                let place = chain.place().unwrap_or(place);
                let ownership = answer!(self.form_ownership(origin, &chain)?);

                // skip raw pointers, they are an explicit unchecked escape
                if ownership == Some(dir::Ownership::Raw) {
                    return Ok(Answer::Ready(None));
                }
                let is_local = self.place_space(place)? == Some(dir::Space::Local);
                if is_local && answer!(self.form_is_reference(origin, &chain)?) {
                    return Ok(Answer::Ready(Some(RepresentationFailure::LocalReference(
                        source,
                    ))));
                }
                ty = chain.base();

                RepresentationCheck::Shared { place, use_fields }
            }
        };

        // fail inline storage on a cycle and stop the shared walk
        if !visited.insert(ty) {
            let failure = match check {
                RepresentationCheck::Concrete { .. } => None,
                RepresentationCheck::Finite => Some(RepresentationFailure::Circular(source)),
                RepresentationCheck::Shared { .. } => None,
            };

            return Ok(Answer::Ready(failure));
        }
        let failure = ensure_sufficient_stack(|| {
            self.representation_child_failure(origin, ty, source, check, visited)
        });
        visited.swap_remove(&ty);

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
    ) -> CompilerResult<Answer<Option<RepresentationFailure>>> {
        let owner = ty.module_id;
        let slots: SmallVec<[(dir::GlobalTypeId, dir::GlobalNodeIdAny); 4]> = match self.ty(ty)? {
            // generic slots require an explicit concrete bound
            dir::Type::Parameter(_) => {
                let RepresentationCheck::Concrete { interface } = check else {
                    return Ok(Answer::Ready(None));
                };
                let holds = self.decide_relation(origin, Relation::Satisfies, ty, interface)?;

                return Ok(holds.map(|holds| (!holds).then_some(RepresentationFailure::Abstract)));
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
                return Ok(Answer::Ready(Some(RepresentationFailure::Abstract)));
            }
            // follow direct forms, which shared checking already stripped
            dir::Type::Form(form) => match form.form {
                dir::Form::Owned | dir::Form::Placed { .. } | dir::Form::Readonly => {
                    SmallVec::from_slice(&[(form.value, source)])
                }
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    return Ok(Answer::Ready(None));
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
            dir::Type::Shape(shape) | dir::Type::Object(shape) => self
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
            _ => return Ok(Answer::Ready(None)),
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
    ) -> CompilerResult<Answer<Option<RepresentationFailure>>> {
        // vectors store their element inline, other opaque intrinsics store no visible children
        if let Some(item) = self.language_item(instance.symbol)? {
            if item == dir::LanguageItem::Vector
                && let Some(element) = self.type_ids(owner, instance.arguments)?.first().copied()
            {
                return self.representation_failure(origin, element, source, check, visited);
            }

            return Ok(Answer::Ready(None));
        }

        let storage = match self.definition(instance.symbol)?.cloned() {
            Some(dir::Definition::Newtype(definition)) => {
                SmallVec::<[_; 4]>::from_slice(&[(definition.backing, source)])
            }
            Some(dir::Definition::Interface(_))
                if matches!(check, RepresentationCheck::Concrete { .. }) =>
            {
                return Ok(Answer::Ready(Some(RepresentationFailure::Abstract)));
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
                    let Some(ty) = answer!(self.definition_member_type(member)?) else {
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
            _ => return Ok(Answer::Ready(None)),
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
    ) -> CompilerResult<Answer<Option<RepresentationFailure>>> {
        for (ty, source) in slots {
            let failure =
                answer!(self.representation_failure(origin, *ty, *source, check, visited)?);
            if failure.is_some() {
                return Ok(Answer::Ready(failure));
            }
        }

        Ok(Answer::Ready(None))
    }
}
