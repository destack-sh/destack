use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether an aliased object of one type grants one access.
    pub(in crate::sema) fn decide_aliased_access(
        &mut self,
        origin: Origin,
        object: dir::GlobalTypeId,
        rung: dir::Access,
    ) -> CompilerResult<Verdict> {
        let object = self.normalize(origin, object)?;
        let mut active = SmallVec::new();
        match rung {
            dir::Access::Readonly | dir::Access::Mutable => Ok(Verdict::Holds),
            dir::Access::Immutable => self.decide_payloads_copy_type(origin, object, &mut active),
            dir::Access::Exclusive => self.decide_components_copy(origin, object, &mut active),
        }
    }

    /// Decide whether every value stored inline in one object copies.
    fn decide_components_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let ty = self.normalize(origin, ty)?;
        let kind = self.ty(ty)?;

        // read the declaration of a nominal object or of a scalar's library representation
        let instance = match kind {
            dir::Type::Application(instance) if !self.is_stuck_head(origin, ty)? => {
                Some((ty.module_id, instance))
            }
            _ => match kind
                .scalar_domain()
                .and_then(dir::ScalarDomain::representation_item)
            {
                Some(item) => {
                    let symbol = self.language_symbol(item)?;

                    Some((symbol.module_id, self.declaration_instance(symbol)?))
                }
                None => None,
            },
        };

        match (instance, kind) {
            // decide a nominal object by its stored fields
            (Some((instance_module, instance)), _) => {
                let Some(definition) = self.definition(instance.symbol)? else {
                    return Ok(Verdict::Fails);
                };
                let Some(fields) = self.stored_object_types(&definition)? else {
                    return self.decide_copy(origin, ty, active);
                };

                self.decide_all_applied(instance_module, &instance, fields, |state, id| {
                    state.decide_copy(origin, id, active)
                })
            }
            // decide a slice object by its element
            (None, dir::Type::Slice(slice)) => self.decide_copy(origin, slice.element, active),
            // refuse objects of unknown storage
            (None, dir::Type::Dynamic(_) | dir::Type::Function(_)) => Ok(Verdict::Fails),
            // decide every other object by its own copying
            (None, _) => self.decide_copy(origin, ty, active),
        }
    }

    /// Decide whether every inline variant payload of one value copies.
    fn decide_payloads_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let ty = self.normalize(origin, ty)?;

        // close recursive storage coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }

        // stop at a value whose default form is a handle
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(Verdict::Holds);
        }
        active.push(ty);
        let verdict = self.decide_payloads_copy_type(origin, ty, active);
        active.pop();

        verdict
    }

    /// Decide inline payload copying by the head of one normalized type.
    fn decide_payloads_copy_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        match self.ty(ty)? {
            // stop at handles, raw pointers, and borrows
            dir::Type::Form(form)
                if matches!(form.form, dir::Form::Raw | dir::Form::Borrowed(_)) =>
            {
                Ok(Verdict::Holds)
            }
            // follow owned and readonly payloads
            dir::Type::Form(form) => self.decide_payloads_copy(origin, form.value, active),
            // leave an open variable undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // require every alternative to copy
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.decide_all(ids, |state, id| {
                    let copies = state.decide_copy(origin, id, active)?;

                    Ok(copies.and(state.decide_payloads_copy(origin, id, active)?))
                })
            }
            // decide intersections through every constituent
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.decide_all(ids, |state, id| {
                    state.decide_payloads_copy(origin, id, active)
                })
            }
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_payloads_copy(origin, refined.base, active)
            }
            // decide element storage through the element
            dir::Type::Slice(slice) => self.decide_payloads_copy(origin, slice.element, active),
            // decide a fixed array through its element
            dir::Type::FixedArray(array) => {
                self.decide_payloads_copy(origin, array.element, active)
            }
            // require every tuple element to copy
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_all(ids, |state, id| {
                    state.decide_payloads_copy(origin, id, active)
                })
            }
            // require every object property to copy
            dir::Type::Object(shape) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .object_properties(ty.module_id, shape.properties)?
                    .iter()
                    .map(|property| property.access.store())
                    .collect();

                self.decide_all(ids, |state, id| {
                    state.decide_payloads_copy(origin, id, active)
                })
            }
            // decide nominal storage through its declaration, a stuck head staying opaque
            dir::Type::Application(_) if self.is_stuck_head(origin, ty)? => self.stuck_verdict(ty),
            dir::Type::Application(instance) => {
                self.decide_payloads_copy_instance(origin, ty, instance, active)
            }
            // admit memory parameters, which qualify storage
            dir::Type::Parameter(parameter) if self.is_memory_parameter(parameter)? => {
                Ok(Verdict::Holds)
            }
            // admit a type parameter that copies by its bounds
            dir::Type::Parameter(_) => self.decide_copy(origin, ty, active),
            // refuse storage of unknown shape
            dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Erased(_) | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached an aliased access decision"),
            }),
            // admit every other value
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Variant(_)
            | dir::Type::Region(_)
            | dir::Type::Range(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::FunctionPointer(_) => Ok(Verdict::Holds),
        }
    }

    /// Decide payload copying for one nominal instance by its declaration.
    fn decide_payloads_copy_instance(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // look through transparent payload intrinsics to their payload
        if let Some(payload) = self.transparent_payload(ty.module_id, &instance)? {
            return self.decide_payloads_copy(origin, payload, active);
        }

        // decide by the declaration's own storage
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(Verdict::Fails);
        };
        let stored = match self.stored_object_types(&definition)? {
            Some(stored) => stored,
            None => match &*definition {
                dir::Definition::Newtype(definition) => SmallVec::from_slice(&[definition.backing]),
                dir::Definition::Enum(_) => return Ok(Verdict::Holds),
                dir::Definition::TypeAlias(_) => {
                    return Err(CompilerError::Internal {
                        message: format!(
                            "alias {} reached an aliased access decision",
                            self.format_symbol(instance.symbol)
                        ),
                    });
                }
                dir::Definition::Struct(_)
                | dir::Definition::Class(_)
                | dir::Definition::Interface(_)
                | dir::Definition::Extension(_) => return Ok(Verdict::Fails),
            },
        };

        self.decide_all_applied(ty.module_id, &instance, stored, |state, id| {
            state.decide_payloads_copy(origin, id, active)
        })
    }
}
