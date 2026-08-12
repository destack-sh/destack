use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type duplicates implicitly without ownership.
    pub(in crate::check) fn satisfies_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        // use bounds declared by generic types
        if let Some(decision) =
            self.decide_generic_auto_interface(origin, ty, dir::AutoInterface::Copy)?
        {
            return Ok(decision);
        }

        // close recursive structural types coinductively
        let ty = self.shallow_resolve(ty)?;
        if active.contains(&ty) {
            return Ok(true);
        }
        active.push(ty);

        // restore the active stack after this decision
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
    ) -> CompilerResult<bool> {
        let kind = self.ty(ty)?;

        // decide explicit memory carriers before their payload types
        if let dir::Type::Form(form) = kind {
            return match form.form {
                dir::Form::Managed | dir::Form::Raw | dir::Form::Readonly => Ok(true),
                dir::Form::Owned => Ok(false),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    self.body().access_is_readonly(access)
                }
                dir::Form::Placed { .. } => self.satisfies_copy(origin, form.value, active),
            };
        }

        // managed defaults copy their compact runtime handles
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(true);
        }

        // decide the remaining structural forms
        match kind {
            dir::Type::Variable(variable) => Err(CompilerError::Internal {
                message: format!("unsolved type variable {variable:?} reached structural copy"),
            }),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_copy(origin, refined.base, active)
            }

            // a declared hole answers like the error it stands for
            dir::Type::Hole(_) => Ok(true),

            // copy owned scalar values directly
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Range(_) => Ok(true),
            dir::Type::Variant(member) => self.satisfies_copy(origin, member.owner, active),
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(false),
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::This => {
                Err(CompilerError::Internal {
                    message: format!("generic type {ty:?} reached structural copy"),
                })
            }
            dir::Type::Form(_) => unreachable!("memory forms return before structural copy"),
            dir::Type::Application(instance) => {
                self.satisfies_copy_instance(origin, ty.module_id, instance, active)
            }
            dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::Shape(_)
            | dir::Type::Object(_) => {
                unreachable!("managed defaults return before structural copy")
            }
            dir::Type::FixedArray(array) => self.satisfies_copy(origin, array.element, active),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
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
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        // admit the copy capability and the scalar markers by definition
        let item = self.language_item(instance.symbol)?;
        if matches!(
            item,
            Some(dir::LanguageItem::Copy | dir::LanguageItem::Phantom)
        ) || item.is_some_and(|item| item.scalar_domain().is_some())
        {
            return Ok(true);
        }

        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(false);
        };

        match definition {
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_copy(origin, definition.value, active)
            }
            dir::Definition::Struct(definition) => {
                let mut fields = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let dir::DefinitionMember::Field(_) = member
                        && let Some(ty) = self.definition_member_type(member)?
                    {
                        fields.push(ty);
                    }
                }

                self.all_applied_copy(origin, instance_module, &instance, fields, active)
            }
            dir::Definition::Enum(_) => Ok(true),
            dir::Definition::Newtype(definition) => self.all_applied_copy(
                origin,
                instance_module,
                &instance,
                [definition.backing],
                active,
            ),
            dir::Definition::Class(_) | dir::Definition::Interface(_) => {
                unreachable!("managed instances return before structural copy")
            }
            dir::Definition::Extension(_) => Ok(false),
        }
    }

    /// Decide copyability for applied nominal component types.
    fn all_applied_copy(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        let substitution = self.instance_substitution(instance_module, instance)?;
        for id in ids {
            let applied = self.substitute_type(id, &substitution)?;
            if !self.satisfies_copy(origin, applied, active)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide whether every type in one iterator is copyable.
    fn all_copy(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<bool> {
        for id in ids {
            if !self.satisfies_copy(origin, id, active)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
