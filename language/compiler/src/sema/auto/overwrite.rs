use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type can be overwritten through non-exclusive access.
    pub(in crate::sema) fn decide_overwrite_stable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        self.decide_guarded(
            origin,
            ty,
            dir::AutoInterface::OverwriteStable,
            active,
            |state, ty, active| state.decide_overwrite_stable_type(origin, ty, active),
        )
    }

    /// Decide overwrite stability for one active type.
    fn decide_overwrite_stable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // read the head standing on the type
        let kind = self.ty(ty)?;

        // decide each stored representation
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // accept region terms outright, they carry no runtime values
            dir::Type::Region(_) => Ok(Verdict::Holds),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_overwrite_stable(origin, refined.base, active)
            }
            // accept the heads whose stored value has one fixed shape
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Range(_)
            | dir::Type::Reference(_) => Ok(Verdict::Holds),
            // decide a variant through its owning enum
            dir::Type::Variant(variant) => {
                self.decide_overwrite_stable(origin, variant.owner, active)
            }
            // reject the unresolved and dispatched heads
            dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::Union(_) => Ok(Verdict::Fails),
            // generic heads resolve before this decision runs
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::This => {
                Err(CompilerError::Internal {
                    message: format!("generic type {ty:?} reached structural overwrite stability"),
                })
            }
            // decide a form through its qualifier
            dir::Type::Form(form) => match form.form {
                dir::Form::Managed { .. } | dir::Form::Borrowed(_) | dir::Form::Raw => {
                    Ok(Verdict::Holds)
                }
                dir::Form::Owned => Ok(Verdict::Fails),
                dir::Form::Readonly => self.decide_overwrite_stable(origin, form.value, active),
            },
            // decide a nominal through its definition
            dir::Type::Application(instance) => {
                self.decide_overwrite_stable_instance(origin, ty.module_id, instance, active)
            }
            // decide an array through its element
            dir::Type::FixedArray(array) => {
                self.decide_overwrite_stable(origin, array.element, active)
            }
            // slices overwrite through their pointer and extent
            dir::Type::Slice(_) => Ok(Verdict::Holds),
            // decide a tuple through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_all(ids, |state, id| {
                    state.decide_overwrite_stable(origin, id, active)
                })
            }
            // decide an object through every property type
            dir::Type::Object(shape) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .object_properties(ty.module_id, shape.properties)?
                    .iter()
                    .flat_map(|field| field.access.types())
                    .collect();

                self.decide_all(ids, |state, id| {
                    state.decide_overwrite_stable(origin, id, active)
                })
            }
            // function pointers store one fixed address
            dir::Type::FunctionPointer(_) => Ok(Verdict::Holds),
            // decide an intersection through every element
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.decide_all(ids, |state, id| {
                    state.decide_overwrite_stable(origin, id, active)
                })
            }
        }
    }

    /// Decide whether one nominal application can be overwritten without exclusivity.
    fn decide_overwrite_stable_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // read the declaration the instance applies
        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(Verdict::Fails);
        };

        // decide by the declaration's own storage
        match definition {
            // decide an alias through its written value
            dir::Definition::TypeAlias(definition) => {
                self.decide_overwrite_stable(origin, definition.value, active)
            }
            // decide a struct through its stored fields
            dir::Definition::Struct(definition) => {
                let fields = self.stored_field_types(&definition.members)?;

                self.decide_all_applied(instance_module, &instance, fields, |state, id| {
                    state.decide_overwrite_stable(origin, id, active)
                })
            }
            // classes and interfaces store one reference
            dir::Definition::Class(_) | dir::Definition::Interface(_) => Ok(Verdict::Holds),
            // an enum's stored payload varies with its tag
            dir::Definition::Enum(_) => Ok(Verdict::Fails),
            // decide a newtype through its backing type
            dir::Definition::Newtype(definition) => self.decide_all_applied(
                instance_module,
                &instance,
                [definition.backing],
                |state, id| state.decide_overwrite_stable(origin, id, active),
            ),
            // extensions store nothing of their own
            dir::Definition::Extension(_) => Ok(Verdict::Fails),
        }
    }
}
