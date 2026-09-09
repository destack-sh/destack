use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Record the copy verdict of each type this module writes, leaving open types undecided.
    pub(in crate::sema) fn write_copies(&mut self, module: ModuleId) -> CompilerResult<()> {
        let node_types = self
            .module
            .types
            .node_types()
            .chain(self.module.types_tail.node_types())
            .filter(|(node, _)| node.module_id == module)
            .collect::<Vec<_>>();
        for (node, ty) in node_types {
            let origin = self.anchored_origin(node)?;
            let mut pending = vec![ty];
            let mut visited = FxIndexSet::default();
            while let Some(id) = pending.pop() {
                if id.module_id != module || !visited.insert(id) {
                    continue;
                }
                self.write_copy(origin, id)?;
                let kind = self.ty(id)?;
                self.for_each_type_child(id.module_id, &kind, |child| pending.push(child))?;
            }
        }

        Ok(())
    }

    /// Record whether the values of one type copy, an open or error type staying undecided.
    fn write_copy(&mut self, origin: Origin, id: dir::GlobalTypeId) -> CompilerResult<()> {
        if self.module.representations_tail.copies(id).is_some() {
            return Ok(());
        }
        let flags = self.type_flags(id)?;
        if flags.has_variable() || flags.has_error() || flags.has_this() {
            return Ok(());
        }
        let copies = match self.decide_copy(origin, id, &mut SmallVec::new())? {
            Verdict::Holds => true,
            Verdict::Fails => false,
            Verdict::Ambiguous => return Ok(()),
        };
        self.module.representations_tail.set_copies(id, copies);

        Ok(())
    }

    /// Decide whether one type duplicates implicitly without ownership.
    pub(in crate::sema) fn decide_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        self.decide_guarded(
            origin,
            ty,
            dir::AutoInterface::Copy,
            active,
            |state, ty, active| state.decide_copy_type(origin, ty, active),
        )
    }

    /// Decide copyability for one active type.
    fn decide_copy_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // read the head standing on the type
        let kind = self.ty(ty)?;

        // decide explicit memory representations before their payload types
        if let dir::Type::Form(form) = kind {
            return match form.form {
                dir::Form::Managed { .. } | dir::Form::Raw | dir::Form::Readonly => {
                    Ok(Verdict::Holds)
                }
                dir::Form::Owned => self.decide_owned_copy(origin, form.value, active),
                dir::Form::Borrowed(borrow) => {
                    let access = self.type_borrow(ty.module_id, borrow)?.access;

                    Ok(Verdict::decided(self.is_readonly_access(access)?))
                }
            };
        }

        // copy a callable handle while its call borrows the receiver, an open mode undecided
        if let dir::Type::Function(function) = kind {
            let receiver = self.shallow_resolve(function.receiver)?;
            if matches!(self.ty(receiver)?, dir::Type::Parameter(_)) {
                return Ok(Verdict::Ambiguous);
            }
            let mode = self.receiver_mode(function.receiver)?;

            return Ok(Verdict::decided(mode != dir::ReceiverMode::Owned));
        }

        // managed defaults copy their compact runtime handles
        if self.default_ownership(origin, ty)? == Some(dir::Ownership::Managed) {
            return Ok(Verdict::Holds);
        }

        self.decide_structural_copy(origin, ty, kind, active)
    }

    /// Decide copyability for one type stored directly in a value.
    fn decide_structural_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        kind: dir::Type,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // decide by the value's own storage
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // accept region terms outright, they carry no runtime values
            dir::Type::Region(_) => Ok(Verdict::Holds),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_copy(origin, refined.base, active)
            }
            // copy owned scalar values directly
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Range(_) => Ok(Verdict::Holds),
            // copy callable values as their compact runtime handles
            dir::Type::FunctionSignature(_) => Ok(Verdict::Holds),
            // decide variants through their owning enum
            dir::Type::Variant(member) => self.decide_copy(origin, member.owner, active),
            // refuse opaque and callable storage
            dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(Verdict::Fails),
            // memory parameters qualify storage and impose none of their own
            dir::Type::Parameter(parameter) if self.is_memory_parameter(parameter)? => {
                Ok(Verdict::Holds)
            }
            // fail the interface for type parameters that survived substitution
            dir::Type::Parameter(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Erased(_) | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached structural copy"),
            }),
            // fail loudly on memory forms decided before this point
            dir::Type::Form(_) => Err(CompilerError::Internal {
                message: format!("memory form {ty:?} reached structural copy"),
            }),
            // decide nominal storage through its declaration, a stuck head staying opaque
            dir::Type::Application(instance) => {
                if self.is_stuck_head(origin, ty)? {
                    return Ok(Verdict::Fails);
                }

                self.decide_copy_instance(origin, ty.module_id, instance, active)
            }
            // fail loudly on managed slices decided before this point
            dir::Type::Slice(_) => Err(CompilerError::Internal {
                message: format!("managed slice {ty:?} reached structural copy"),
            }),
            // decide fixed arrays through their element
            dir::Type::FixedArray(array) => self.decide_copy(origin, array.element, active),
            // decide tuples through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_all(ids, |state, id| state.decide_copy(origin, id, active))
            }
            // decide anonymous objects through their stored properties
            dir::Type::Object(shape) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .object_properties(ty.module_id, shape.properties)?
                    .iter()
                    .map(|property| property.access.store())
                    .collect();

                self.decide_all(ids, |state, id| state.decide_copy(origin, id, active))
            }
            // decide unions through every alternative
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.decide_all(ids, |state, id| state.decide_copy(origin, id, active))
            }
            // decide intersections through every constituent
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.decide_all(ids, |state, id| state.decide_copy(origin, id, active))
            }
        }
    }

    /// Decide copyability for one payload stored inline as an owned value.
    fn decide_owned_copy(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // reduce the payload the owned form keeps written
        let ty = self.normalize(origin, ty)?;

        // close recursive owned values coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }

        // read the head standing on the payload
        let kind = self.ty(ty)?;

        // decide indirect scalar representations through their library storage
        if let Some(item) = kind
            .scalar_domain()
            .and_then(dir::ScalarDomain::representation_item)
        {
            let symbol = self.language_symbol(item)?;
            let instance = self.declaration_instance(symbol)?;
            active.push(ty);
            let result = self.decide_copy_instance(origin, symbol.module_id, instance, active);
            active.pop();

            return result;
        }

        // decide owned payloads by their stored representation
        match kind {
            // decide explicit memory representations through the representation rules
            dir::Type::Form(_) => self.decide_copy(origin, ty, active),
            // refuse representations whose descriptor uniquely owns indirect storage
            dir::Type::Dynamic(_) | dir::Type::Function(_) | dir::Type::Slice(_) => {
                Ok(Verdict::Fails)
            }
            // decide inline nominal storage past its managed handle default
            dir::Type::Application(_) if self.is_stuck_head(origin, ty)? => Ok(Verdict::Fails),
            dir::Type::Application(instance) => {
                active.push(ty);
                let result = self.decide_copy_instance(origin, ty.module_id, instance, active);
                active.pop();

                result
            }
            // preserve structural copy for owned inline values
            _ => self.decide_structural_copy(origin, ty, kind, active),
        }
    }

    /// Decide copyability for one nominal instance.
    fn decide_copy_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // hold the copy capability and the scalar markers by definition
        let item = self.language_item(instance.symbol)?;
        if matches!(
            item,
            Some(dir::LanguageItem::Copy | dir::LanguageItem::Phantom)
        ) || item.is_some_and(|item| item.scalar_domain().is_some())
        {
            return Ok(Verdict::Holds);
        }

        // copy transparent payload intrinsics through their first type argument
        if matches!(
            item,
            Some(
                dir::LanguageItem::UnsafeCell
                    | dir::LanguageItem::ManuallyDrop
                    | dir::LanguageItem::MaybeUninit
                    | dir::LanguageItem::Wrapping
                    | dir::LanguageItem::Pin
            )
        ) {
            let arguments = self.type_ids(instance_module, instance.arguments)?;
            let Some(argument) = arguments.first().copied() else {
                return Ok(Verdict::Fails);
            };

            return self.decide_copy(origin, argument, active);
        }

        // move an instance whose symbol declares no definition
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(Verdict::Fails);
        };

        // move a value whose declaration refuses copies
        if !self.permits_copy(instance.symbol)? {
            return Ok(Verdict::Fails);
        }

        // decide by the declaration's own storage
        match &*definition {
            // normalization unfolds aliases before this decision
            dir::Definition::TypeAlias(_) => Err(CompilerError::Internal {
                message: format!(
                    "alias {} reached structural copy",
                    self.format_symbol(instance.symbol)
                ),
            }),
            // copy a struct once every field copies
            dir::Definition::Struct(definition) => {
                let fields = self.stored_field_types(&definition.members)?;

                self.decide_all_applied(instance_module, &instance, fields, |state, id| {
                    state.decide_copy(origin, id, active)
                })
            }
            // copy an enum at its integer tag or managed string reference
            dir::Definition::Enum(_) => Ok(Verdict::Holds),
            // copy a newtype through its backing type
            dir::Definition::Newtype(definition) => self.decide_all_applied(
                instance_module,
                &instance,
                [definition.backing],
                |state, id| state.decide_copy(origin, id, active),
            ),
            // move class values
            dir::Definition::Class(_) => Ok(Verdict::Fails),
            // move interface values
            dir::Definition::Interface(_) => Ok(Verdict::Fails),
            // move extension values
            dir::Definition::Extension(_) => Ok(Verdict::Fails),
        }
    }

    /// Return whether one type is a raw pointer form.
    fn is_raw_pointer(&mut self, ty: dir::GlobalTypeId) -> CompilerResult<bool> {
        let ty = self.shallow_resolve(ty)?;

        Ok(matches!(self.ty(ty)?, dir::Type::Form(form) if form.form == dir::Form::Raw))
    }

    /// Commit whether one value declaration derives Copy.
    pub(in crate::sema) fn commit_copy_derivation(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let derives_copy = self.permits_copy(symbol)?;
        self.module_mut(symbol.module_id)
            .representations_tail
            .set_derives_copy(symbol, derives_copy);

        Ok(())
    }

    /// Return whether one declaration permits copying by structure.
    fn permits_copy(&mut self, symbol: dir::GlobalSymbolId) -> CompilerResult<bool> {
        Ok(
            !self.declares_drop(symbol)?
                && !self.stores_raw_pointer_without_derived_copy(symbol)?,
        )
    }

    /// Return whether one declaration stores a raw pointer outside a written Copy derive.
    fn stores_raw_pointer_without_derived_copy(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let Some(definition) = self.definition(symbol)? else {
            return Ok(false);
        };
        let (derives, stored) = match &*definition {
            dir::Definition::Struct(definition) => (
                definition.derives.as_deref(),
                self.stored_field_types(&definition.members)?.to_vec(),
            ),
            dir::Definition::Newtype(definition) => {
                (definition.derives.as_deref(), vec![definition.backing])
            }
            _ => return Ok(false),
        };
        if derives
            .unwrap_or_default()
            .contains(&dir::AutoInterface::Copy)
        {
            return Ok(false);
        }
        for field in stored {
            if self.is_raw_pointer(field)? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
