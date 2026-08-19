use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type duplicates implicitly without ownership.
    pub(in crate::sema) fn satisfies_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // use bounds declared by generic types
        if let Some(decision) =
            self.decide_generic_auto_interface(origin, ty, dir::AutoInterface::Copy)?
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
    ) -> CompilerResult<Verdict> {
        let kind = self.ty(ty)?;

        // decide explicit memory carriers before their payload types
        if let dir::Type::Form(form) = kind {
            return match form.form {
                dir::Form::Managed | dir::Form::Raw | dir::Form::Readonly => Ok(Verdict::Holds),
                dir::Form::Owned => self.satisfies_owned_copy(origin, form.value, active),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    Ok(Verdict::decided(self.body().access_is_readonly(access)?))
                }
                dir::Form::Placed { .. } => self.satisfies_copy(origin, form.value, active),
            };
        }

        // managed defaults copy their compact runtime handles
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(Verdict::Holds);
        }

        self.satisfies_structural_copy(origin, ty, kind, active)
    }

    /// Decide copyability for one type stored directly in a value.
    fn satisfies_structural_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        kind: dir::Type,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) | dir::Type::Hole(_) => Ok(Verdict::Ambiguous),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_copy(origin, refined.base, active)
            }

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
            | dir::Type::Range(_) => Ok(Verdict::Holds),
            // judge variants through their owning enum
            dir::Type::Variant(member) => self.satisfies_copy(origin, member.owner, active),
            // reject opaque and callable storage
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Parameter(_)
            | dir::Type::Rigid(_)
            | dir::Type::Erased(_)
            | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached structural copy"),
            }),
            // explicit memory forms decide before structural storage
            dir::Type::Form(_) => unreachable!("memory forms return before structural copy"),
            // judge nominal storage through its declaration
            dir::Type::Application(instance) => {
                self.satisfies_copy_instance(origin, ty.module_id, instance, active)
            }
            // bare slices return through their managed default
            dir::Type::Slice(_) => {
                unreachable!("managed slices return before structural copy")
            }
            // judge fixed arrays through their element
            dir::Type::FixedArray(array) => self.satisfies_copy(origin, array.element, active),
            // judge tuples through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_copy(origin, ids, active)
            }
            // judge anonymous objects through their stored properties
            dir::Type::Object(shape) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .iter()
                    .map(|property| property.access.store())
                    .collect();

                self.all_copy(origin, ids, active)
            }
            // judge unions through every alternative
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_copy(origin, ids, active)
            }
            // judge intersections through every constituent
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_copy(origin, ids, active)
            }
        }
    }

    /// Decide copyability for one payload stored inline as an owned value.
    fn satisfies_owned_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // name bare declarations as their canonical applications
        let ty = self.shallow_resolve(ty)?;
        let ty = match self.ty(ty)? {
            dir::Type::Reference(reference) => {
                let instance = self.declaration_instance(reference.symbol)?;

                self.intern_type(dir::Type::Application(instance))?
            }
            _ => ty,
        };

        // close recursive owned values coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }

        let kind = self.ty(ty)?;

        // judge indirect scalar representations through their library storage
        if let Some(item) = kind
            .scalar_domain()
            .and_then(dir::ScalarDomain::representation_item)
        {
            let representation = self.language_type(item, &[])?;

            return self.satisfies_owned_copy(origin, representation, active);
        }

        // decide owned payloads by their stored representation
        match kind {
            // reject carriers whose descriptor uniquely owns indirect storage
            dir::Type::Dynamic(_) | dir::Type::Function(_) | dir::Type::Slice(_) => {
                Ok(Verdict::Fails)
            }
            // judge inline nominal storage past its managed handle default
            dir::Type::Application(instance) => {
                active.push(ty);
                let result = self.satisfies_copy_instance(origin, ty.module_id, instance, active);
                active.pop();

                result
            }
            // preserve structural copy for owned inline values
            _ => self.satisfies_structural_copy(origin, ty, kind, active),
        }
    }

    /// Decide copyability for one nominal instance.
    fn satisfies_copy_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // admit the copy capability and the scalar markers by definition
        let item = self.language_item(instance.symbol)?;
        if matches!(
            item,
            Some(dir::LanguageItem::Copy | dir::LanguageItem::Phantom)
        ) || item.is_some_and(|item| item.scalar_domain().is_some())
        {
            return Ok(Verdict::Holds);
        }

        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(Verdict::Fails);
        };

        match definition {
            // look through an alias to the type it names
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_copy(origin, definition.value, active)
            }
            // copy a struct once every field copies
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
            // copy an enum at its integer tag or managed string reference
            dir::Definition::Enum(_) => Ok(Verdict::Holds),
            // copy a newtype once its backing type copies
            dir::Definition::Newtype(definition) => self.all_applied_copy(
                origin,
                instance_module,
                &instance,
                [definition.backing],
                active,
            ),
            // move class values
            dir::Definition::Class(_) => Ok(Verdict::Fails),
            // move interface values
            dir::Definition::Interface(_) => Ok(Verdict::Fails),
            // move extension values
            dir::Definition::Extension(_) => Ok(Verdict::Fails),
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
    ) -> CompilerResult<Verdict> {
        let substitution = self.instance_substitution(instance_module, instance)?;
        let mut verdict = Verdict::Holds;
        for id in ids {
            let applied = self.substitute_type(id, &substitution)?;
            verdict = verdict.and(self.satisfies_copy(origin, applied, active)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Decide whether every type in one iterator is copyable.
    fn all_copy(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for id in ids {
            verdict = verdict.and(self.satisfies_copy(origin, id, active)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }
}
