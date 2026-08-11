use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{CheckState, Origin, Relation};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type auto-derives one field-wise interface.
    pub(in crate::check) fn satisfies_derivable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // use bounds declared by generic types
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            return Ok(decision);
        }

        // close recursive types coinductively across conformance re-entry
        let ty = self.resolve_head(ty)?;
        if !self.deriving.insert((ty, interface)) {
            return Ok(true);
        }

        // decide the type, then release its re-entry mark
        let result = self.decide_derivable_type(origin, ty, interface);
        self.deriving.swap_remove(&(ty, interface));

        result
    }

    /// Decide field-wise derivability for one active type.
    fn decide_derivable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // read the head structure of the subject
        let ty = self.resolve_head(ty)?;
        let kind = self.ty(ty)?;

        // decide explicit memory carriers before their payload types
        if let dir::Type::Form(form) = kind {
            return match form.form {
                // clone handles and views by duplicating the carrier
                dir::Form::Managed | dir::Form::Readonly | dir::Form::Borrowed(_)
                    if interface == dir::AutoInterface::Clone =>
                {
                    Ok(true)
                }
                // raw addresses compare, hash, and print by identity
                dir::Form::Raw => Ok(true),
                // forward every other carrier to its payload
                dir::Form::Managed
                | dir::Form::Readonly
                | dir::Form::Borrowed(_)
                | dir::Form::Owned => self.satisfies_derivable(origin, form.value, interface),
                dir::Form::Placed { .. } => self.satisfies_derivable(origin, form.value, interface),
            };
        }

        // decide the remaining structural forms
        match kind {
            // FUGU #Suspicious: open variables optimistically conform
            dir::Type::Variable(_) => Ok(true),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_derivable(origin, refined.base, interface)
            }

            // trivial singletons conform to every field-wise interface
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined => Ok(true),
            // decide scalar leaves by their domain
            dir::Type::Primitive(_) | dir::Type::Literal(_) | dir::Type::Range(_) => Ok(kind
                .scalar_domain()
                .and_then(|domain| domain.conforms_to(interface))
                .unwrap_or(false)),
            // decide a variant through its owning enum
            dir::Type::Variant(member) => self.satisfies_derivable(origin, member.owner, interface),

            // opaque and callable forms carry no field-wise conformance
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => Ok(false),
            // fail loudly on generic forms that survived substitution
            dir::Type::Parameter(_) | dir::Type::Erased(_) | dir::Type::This => {
                Err(CompilerError::Internal {
                    message: format!("generic type {ty:?} reached structural derivability"),
                })
            }
            dir::Type::Form(_) => {
                unreachable!("memory forms return before structural derivability")
            }

            // decide a nominal instance through its declaration
            dir::Type::Application(instance) => {
                self.decide_derivable_instance(origin, ty.module_id, instance, interface)
            }

            // structural containers stay with their declared library conformances
            dir::Type::Array(_)
            | dir::Type::Slice(_)
            | dir::Type::Shape(_)
            | dir::Type::Object(_) => Ok(false),

            // decide composites through every component type
            dir::Type::FixedArray(array) => self.field_conforms(origin, array.element, interface),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_fields_conform(origin, ids, interface)
            }
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_fields_conform(origin, ids, interface)
            }
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_fields_conform(origin, ids, interface)
            }
        }
    }

    /// Decide field-wise derivability for one nominal instance.
    fn decide_derivable_instance(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: dir::GenericApplication,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // read the declaration the instance applies
        let Some(definition) = self.definition(instance.symbol)?.cloned() else {
            return Ok(false);
        };

        match definition {
            // look through an alias to its value
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_derivable(origin, definition.value, interface)
            }
            // scalar-backed enums conform through their backing values
            dir::Definition::Enum(_) => Ok(true),
            // structs conform when every stored field conforms
            dir::Definition::Struct(definition) => {
                let mut fields = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let dir::DefinitionMember::Field(_) = member
                        && let Some(ty) = self.definition_member_type(member)?
                    {
                        fields.push(ty);
                    }
                }

                self.all_applied_conform(origin, instance_module, &instance, fields, interface)
            }
            // newtypes conform through their backing type
            dir::Definition::Newtype(definition) => self.all_applied_conform(
                origin,
                instance_module,
                &instance,
                [definition.backing],
                interface,
            ),
            dir::Definition::Class(definition) => {
                // class instances equate, hash, and clone by managed identity
                let formats = matches!(
                    interface,
                    dir::AutoInterface::Debug | dir::AutoInterface::Display
                );
                if !formats {
                    return Ok(true);
                }

                // collect the stored field types
                let mut fields = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let dir::DefinitionMember::Field(_) = member
                        && let Some(ty) = self.definition_member_type(member)?
                    {
                        fields.push(ty);
                    }
                }

                // walk the extended base alongside the stored fields
                if let Some(extends) = &definition.extends {
                    fields.push(extends.ty);
                }

                self.all_applied_conform(origin, instance_module, &instance, fields, interface)
            }
            // interfaces and extensions declare no stored fields
            dir::Definition::Interface(_) | dir::Definition::Extension(_) => Ok(false),
        }
    }

    /// Decide conformance of applied nominal component types.
    fn all_applied_conform(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // apply the instance arguments to each component before deciding it
        let substitution = self.instance_substitution(instance_module, instance)?;
        for id in ids {
            let applied = self.substitute_type(id, &substitution)?;
            if !self.field_conforms(origin, applied, interface)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide whether every component type conforms.
    fn all_fields_conform(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        for id in ids {
            if !self.field_conforms(origin, id, interface)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Decide one component conformance through the full interface relation.
    fn field_conforms(
        &mut self,
        origin: Origin,
        field: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<bool> {
        // full conformance lets declared implementations serve components
        let item = dir::LanguageItem::from(interface);
        let target = self.language_type(item, &[])?;

        self.decide_relation(origin, Relation::Satisfies, field, target)
    }
}
