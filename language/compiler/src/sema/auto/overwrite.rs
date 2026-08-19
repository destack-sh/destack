use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type can be overwritten through non-exclusive access.
    pub(in crate::sema) fn satisfies_overwrite_stable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // use bounds declared by generic types
        if let Some(decision) =
            self.decide_generic_auto_interface(origin, ty, dir::AutoInterface::OverwriteStable)?
        {
            return Ok(Verdict::decided(decision));
        }

        // close recursive structural types coinductively
        let ty = self.shallow_resolve(ty)?;
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }
        active.push(ty);

        // restore the active stack after this decision
        let result = self.decide_overwrite_stable_type(origin, ty, active);
        active.pop();

        result
    }

    /// Decide overwrite stability for one active type.
    fn decide_overwrite_stable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let kind = self.ty(ty)?;

        // decide each stored representation
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) | dir::Type::Hole(_) => Ok(Verdict::Ambiguous),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_overwrite_stable(origin, refined.base, active)
            }

            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_)
            | dir::Type::Reference(_) => Ok(Verdict::Holds),
            dir::Type::Variant(variant) => {
                self.satisfies_overwrite_stable(origin, variant.owner, active)
            }
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::Union(_) => Ok(Verdict::Fails),
            dir::Type::Parameter(_)
            | dir::Type::Rigid(_)
            | dir::Type::Erased(_)
            | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached structural overwrite stability"),
            }),
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed | dir::Form::Borrowed(_) | dir::Form::Raw => Ok(Verdict::Holds),
                dir::Form::Owned => Ok(Verdict::Fails),
                dir::Form::Placed { .. } | dir::Form::Readonly => {
                    self.satisfies_overwrite_stable(origin, form.value, active)
                }
            },
            dir::Type::Application(instance) => {
                self.satisfies_overwrite_stable_instance(origin, ty.module_id, instance, active)
            }
            dir::Type::FixedArray(array) => {
                self.satisfies_overwrite_stable(origin, array.element, active)
            }
            dir::Type::Slice(_) => Ok(Verdict::Holds),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_overwrite_stable(origin, ids, active)
            }
            dir::Type::Object(shape) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .iter()
                    .flat_map(|field| field.access.types())
                    .collect();

                self.all_overwrite_stable(origin, ids, active)
            }
            dir::Type::FunctionPointer(_) => Ok(Verdict::Holds),
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_overwrite_stable(origin, ids, active)
            }
        }
    }

    /// Decide whether one nominal application can be overwritten without exclusivity.
    fn satisfies_overwrite_stable_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(Verdict::Fails);
        };

        match definition {
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_overwrite_stable(origin, definition.value, active)
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

                self.all_applied_overwrite_stable(
                    origin,
                    instance_module,
                    &instance,
                    fields,
                    active,
                )
            }
            dir::Definition::Class(_) | dir::Definition::Interface(_) => Ok(Verdict::Holds),
            dir::Definition::Enum(_) => Ok(Verdict::Fails),
            dir::Definition::Newtype(definition) => self.all_applied_overwrite_stable(
                origin,
                instance_module,
                &instance,
                [definition.backing],
                active,
            ),
            dir::Definition::Extension(_) => Ok(Verdict::Fails),
        }
    }

    /// Decide overwrite stability for applied nominal component types.
    fn all_applied_overwrite_stable(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let substitution = self.instance_substitution(instance_module, instance)?;
        let mut verdict = Verdict::Holds;
        for id in ids {
            let id = self.substitute_type(id, &substitution)?;
            verdict = verdict.and(self.satisfies_overwrite_stable(origin, id, active)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Decide whether every type in one iterator is overwrite-stable.
    fn all_overwrite_stable(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for id in ids {
            verdict = verdict.and(self.satisfies_overwrite_stable(origin, id, active)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }
}
