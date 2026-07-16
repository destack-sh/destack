use destack_core::{FxIndexSet, ensure_sufficient_stack};
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, ObligationCheck, ObligationFailure, Origin, answer};

/// The representation property being checked.
#[derive(Debug, Clone, Copy)]
enum RepresentationCheck {
    /// Inline storage must terminate.
    Finite,
    /// Safe references reachable from shared storage must remain shared.
    Shared {
        /// The containing carrier's concrete place.
        place: dir::GlobalTypeId,
        /// Report fields when checking their own declaration.
        use_fields: bool,
    },
}

/// One invalid representation found while walking stored children.
enum RepresentationFailure {
    /// Inline storage contains itself.
    Circular(dir::GlobalNodeIdAny),
    /// Shared storage retains a safe local reference.
    LocalReference(dir::GlobalNodeIdAny),
}

impl CheckState<'_> {
    /// Decide whether one type's values may live in shared space.
    pub(in crate::check) fn satisfies_shared_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        // intrinsically local declarations never store shared
        let value = self.value_beneath_forms(origin, ty)?;
        let value = answer!(self.reduce_type_head(origin, value)?);
        let symbol = match self.ty(value)? {
            dir::Type::Instance(instance) => Some(instance.symbol),
            dir::Type::Reference(reference) => Some(reference.symbol),
            _ => None,
        };
        if let Some(symbol) = symbol
            && self.nominal_space(symbol)? == Some(dir::Space::Local)
        {
            return Ok(Answer::Ready(false));
        }

        // shared containment walks the stored representation
        let source = self.origin_source(origin)?;
        let place = self.intern_type(
            origin.module(),
            dir::Type::Memory(dir::MemoryLiteral::Place(dir::Place::Space(
                dir::Space::Shared,
            ))),
        )?;
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

        // shared reachability applies only to concretely shared roots
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
                    dir::Type::Instance(instance) => {
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
            RepresentationCheck::Finite => RepresentationCheck::Finite,
            RepresentationCheck::Shared { place, use_fields } => {
                ty = self.resolve_relative_place(origin, ty, place)?;
                let chain = self.form_chain(origin, ty)?;
                let place = chain.place().unwrap_or(place);
                let ownership = answer!(self.form_ownership(origin, &chain)?);

                // raw pointers are an explicit unchecked escape hatch
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

        // cycles fail inline storage and terminate shared graph traversal
        if !visited.insert(ty) {
            let failure = match check {
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
            dir::Type::Shape(shape) => self
                .shape_fields(owner, shape.fields)?
                .iter()
                .map(|field| (field.ty, source))
                .collect(),
            dir::Type::Union(union) => self
                .type_ids(owner, union.elements)?
                .iter()
                .map(|element| (*element, source))
                .collect(),
            dir::Type::EnumMember(member) => SmallVec::from_slice(&[(member.owner, source)]),
            dir::Type::Instance(instance) => {
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
        instance: &dir::GenericInstance,
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
                SmallVec::<[_; 4]>::from_slice(&[(definition.value, source)])
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
            slots.push((
                self.substitute_type(origin.module(), ty, &substitution)?,
                source,
            ));
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
