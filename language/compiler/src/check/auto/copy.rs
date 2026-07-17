use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Decide whether one type duplicates implicitly without ownership.
    pub(in crate::check) fn satisfies_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.settled_root(ty)?;
        if active.contains(&ty) {
            return Ok(Answer::Ready(true));
        }
        active.push(ty);

        let result = self.decide_copy_type(origin, ty, active);
        active.pop();

        result
    }

    /// Decide copyability for one active type.
    fn decide_copy_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
        let kind = self.ty(ty)?;

        match kind {
            dir::Type::Variable(variable) => {
                Ok(Answer::pending([self.variable_dependency(variable)?]))
            }
            // refinements constrain members without changing the base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_copy(origin, refined.base, active)
            }

            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::FunctionPointer(_)
            // primitives copy as values or as immutable managed references
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Range(_) => Ok(Answer::Ready(true)),
            dir::Type::EnumMember(member) => self.satisfies_copy(origin, member.owner, active),
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Object
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(Answer::Ready(false)),
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                // prove through declared or assumed bounds
                let mut decision = Answer::Ready(false);
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = decision.or(self.satisfies_copy(origin, bound, active)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }
            dir::Type::This => {
                // prove through assumed this bounds
                let bounds = self.assumed_bounds(origin, |ty| matches!(ty, dir::Type::This))?;
                let mut decision = Answer::Ready(false);
                for bound in bounds {
                    decision = decision.or(self.satisfies_copy(origin, bound, active)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }
            dir::Type::Form(form) => match form.form {
                dir::Form::Raw | dir::Form::Readonly => Ok(Answer::Ready(true)),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    self.body(origin.module())
                        .access_is_readonly(origin, access)
                }
                dir::Form::Managed | dir::Form::Owned | dir::Form::Placed { .. } => {
                    Ok(Answer::Ready(false))
                }
            },
            dir::Type::Instance(instance) => {
                self.satisfies_copy_instance(origin, ty.module_id, instance, active)
            }
            dir::Type::Array(_) | dir::Type::Slice(_) => Ok(Answer::Ready(false)),
            dir::Type::FixedArray(array) => self.satisfies_copy(origin, array.element, active),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_copy(origin, ids, active)
            }
            dir::Type::Shape(shape) => {
                if !self
                    .type_ids(ty.module_id, shape.call_signatures)?
                    .is_empty()
                    || !self
                        .type_ids(ty.module_id, shape.construct_signatures)?
                        .is_empty()
                {
                    return Ok(Answer::Ready(false));
                }
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .shape_fields(ty.module_id, shape.fields)?
                    .iter()
                    .map(|field| field.ty)
                    .collect();

                self.all_copy(origin, ids, active)
            }
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_copy(origin, ids, active)
            }
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_copy(origin, ids, active)
            }
        }
    }

    /// Decide copyability for one nominal instance.
    fn satisfies_copy_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericInstance,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        if matches!(
            self.language_item(instance.symbol)?,
            Some(dir::LanguageItem::Copy | dir::LanguageItem::Integer | dir::LanguageItem::Float)
        ) {
            return Ok(Answer::Ready(true));
        }

        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(Answer::Ready(false));
        };

        match definition {
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_copy(origin, definition.value, active)
            }
            dir::Definition::Struct(definition) => {
                let mut fields = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let dir::DefinitionMember::Field(_) = member
                        && let Some(ty) = answer!(self.definition_member_type(member)?)
                    {
                        fields.push(ty);
                    }
                }

                self.all_applied_copy(origin, instance_module, &instance, fields, active)
            }
            dir::Definition::Enum(definition) => {
                let mut payloads = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let Some(ty) = answer!(self.definition_member_type(member)?) {
                        payloads.push(ty);
                    }
                }

                self.all_applied_copy(origin, instance_module, &instance, payloads, active)
            }
            dir::Definition::Newtype(definition) => self.all_applied_copy(
                origin,
                instance_module,
                &instance,
                [definition.value],
                active,
            ),
            dir::Definition::Class(_)
            | dir::Definition::Interface(_)
            | dir::Definition::Extension(_) => Ok(Answer::Ready(false)),
        }
    }

    /// Decide copyability for applied nominal component types.
    fn all_applied_copy(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericInstance,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let substitution = self.instance_substitution(instance_module, instance)?;
        let mut decision = Answer::Ready(true);
        for id in ids {
            let applied = self.substitute_type(origin.module(), id, &substitution)?;
            decision = decision.and(self.satisfies_copy(origin, applied, active)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether every type in one iterator is copyable.
    fn all_copy(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.satisfies_copy(origin, id, active)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }
}
