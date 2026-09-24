use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{ActiveGoal, CheckState, Origin, Relation, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type auto-derives one field-wise interface.
    pub(in crate::sema) fn decide_derivable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // unfold aliases, then use bounds declared by generic types
        let ty = self.normalize(origin, ty)?;
        if let Some(decision) = self.decide_generic_auto_interface(origin, ty, interface)? {
            return Ok(Verdict::decided(decision));
        }

        // close recursive types coinductively across conformance re-entry
        if !self.active.insert(ActiveGoal::Derive(ty, interface)) {
            return Ok(Verdict::Holds);
        }

        // decide the type, then release its re-entry mark
        let result = self.decide_derivable_type(origin, ty, interface);
        self.active.swap_remove(&ActiveGoal::Derive(ty, interface));

        result
    }

    /// Decide field-wise derivability for one active type.
    fn decide_derivable_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // read the head structure of the subject
        let ty = self.shallow_resolve(ty)?;
        let kind = self.ty(ty)?;

        // decide memory forms before their payload types
        if let dir::Type::Form(form) = kind {
            return match form.form {
                // clone a view by duplicating the reference
                dir::Form::Readonly | dir::Form::Borrowed(_)
                    if interface == dir::AutoInterface::Clone =>
                {
                    Ok(Verdict::Holds)
                }
                // refuse default and zero values for reference forms
                dir::Form::Borrowed(_) | dir::Form::Owned
                    if matches!(
                        interface,
                        dir::AutoInterface::Default | dir::AutoInterface::Zeroable
                    ) =>
                {
                    Ok(Verdict::Fails)
                }
                // unpin reference forms
                dir::Form::Borrowed(_) if interface == dir::AutoInterface::Unpin => {
                    Ok(Verdict::Holds)
                }
                // raw addresses compare, hash, print, and zero by identity, and refuse a default
                dir::Form::Raw => Ok(Verdict::decided(interface != dir::AutoInterface::Default)),
                // forward every other form to its payload
                dir::Form::Readonly | dir::Form::Borrowed(_) | dir::Form::Owned => {
                    self.decide_derivable(origin, form.value, interface)
                }
            };
        }

        // decide the remaining structural forms
        match kind {
            // leave an open variable undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // accept region terms, which have no runtime values
            dir::Type::Region(_) => Ok(Verdict::Holds),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_derivable(origin, refined.base, interface)
            }

            // accept trivial singletons for every field-wise interface
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined => Ok(Verdict::Holds),
            // decide scalar leaves by their domain
            dir::Type::Primitive(_) | dir::Type::Literal(_) | dir::Type::Range(_) => {
                let conforms = kind
                    .scalar_domain()
                    .and_then(|domain| domain.conforms_to(interface))
                    .unwrap_or(false);

                Ok(Verdict::decided(conforms))
            }
            // decide a variant through its owning enum
            dir::Type::Variant(member) => self.decide_derivable(origin, member.owner, interface),

            // clone callable values by duplicating their repeatable handles
            dir::Type::FunctionSignature(_) | dir::Type::FunctionPointer(_)
                if interface == dir::AutoInterface::Clone =>
            {
                Ok(Verdict::Holds)
            }
            dir::Type::Function(function) if interface == dir::AutoInterface::Clone => {
                let mode = self.receiver_mode(function.receiver)?;

                Ok(Verdict::decided(mode != dir::ReceiverMode::Owned))
            }
            // decide opaque and callable forms by unpin alone
            dir::Type::Unknown
            | dir::Type::Intrinsic
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::Dynamic(_)
            | dir::Type::Function(_)
            | dir::Type::Reference(_) => {
                Ok(Verdict::decided(interface == dir::AutoInterface::Unpin))
            }
            // accept memory parameters, they qualify storage alone
            dir::Type::Parameter(parameter) if self.is_memory_parameter(parameter)? => {
                Ok(Verdict::Holds)
            }
            // fail the interface for type parameters that survived substitution
            dir::Type::Parameter(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Erased(_) | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached structural derivability"),
            }),
            // fail loudly on memory forms decided before this point
            dir::Type::Form(_) => Err(CompilerError::Internal {
                message: format!("memory form {ty:?} reached structural derivability"),
            }),

            // decide a nominal instance through its declaration, a stuck head staying opaque
            dir::Type::Application(instance) => match self.is_stuck_head(origin, ty)? {
                true => Ok(Verdict::Fails),
                false => self.decide_derivable_instance(origin, ty.module_id, instance, interface),
            },

            // decide structural containers by unpin, their library declares the rest
            dir::Type::Slice(_) | dir::Type::Object(_) => {
                Ok(Verdict::decided(interface == dir::AutoInterface::Unpin))
            }

            // decide composites through every component type
            dir::Type::FixedArray(array) => self.decide_field(origin, array.element, interface),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_fields(origin, ids, interface)
            }
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.decide_fields(origin, ids, interface)
            }
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.decide_fields(origin, ids, interface)
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
    ) -> CompilerResult<Verdict> {
        // read the declaration the instance applies
        let Some(definition) = self.definition(instance.symbol)? else {
            return Ok(Verdict::Fails);
        };

        // decide by the declaration's own storage
        match &*definition {
            // fail loudly, normalization unfolds aliases before this decision
            dir::Definition::TypeAlias(_) => Err(CompilerError::Internal {
                message: format!(
                    "alias {:?} reached structural derivability",
                    instance.symbol
                ),
            }),
            // conform scalar-backed enums through their backing values
            dir::Definition::Enum(_) => Ok(Verdict::decided(!matches!(
                interface,
                dir::AutoInterface::Default | dir::AutoInterface::Zeroable
            ))),
            // conform structs when every stored field conforms
            dir::Definition::Struct(definition) => {
                // refuse Unpin for pinned storage
                if interface == dir::AutoInterface::Unpin
                    && self.language_item(instance.symbol)? == Some(dir::LanguageItem::Pin)
                {
                    return Ok(Verdict::Fails);
                }

                // collect the stored field types
                let fields = self.stored_field_types(&definition.members)?;

                self.decide_applied_fields(origin, instance_module, &instance, fields, interface)
            }
            // conform newtypes through their backing type
            dir::Definition::Newtype(definition) => self.decide_applied_fields(
                origin,
                instance_module,
                &instance,
                [definition.backing],
                interface,
            ),
            // conform classes through their stored fields and base
            dir::Definition::Class(definition) => {
                if !interface.derives_over_class() {
                    return Ok(Verdict::Fails);
                }

                // collect the stored field types
                let mut fields = self.stored_field_types(&definition.members)?;

                // walk the extended base alongside the stored fields
                if let Some(extends) = &definition.extends {
                    fields.push(extends.ty);
                }

                self.decide_applied_fields(origin, instance_module, &instance, fields, interface)
            }
            // refuse conformance for interfaces and extensions
            dir::Definition::Interface(_) | dir::Definition::Extension(_) => Ok(Verdict::Fails),
        }
    }

    /// Decide conformance of applied nominal component types.
    fn decide_applied_fields(
        &mut self,
        origin: Origin,
        instance_module: ModuleId,
        instance: &dir::GenericApplication,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        self.decide_all_applied(instance_module, instance, ids, |state, id| {
            state.decide_field(origin, id, interface)
        })
    }

    /// Decide whether every component type conforms.
    fn decide_fields(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        self.decide_all(ids, |state, id| state.decide_field(origin, id, interface))
    }

    /// Decide one component conformance through the full interface relation.
    fn decide_field(
        &mut self,
        origin: Origin,
        field: dir::GlobalTypeId,
        interface: dir::AutoInterface,
    ) -> CompilerResult<Verdict> {
        // full conformance lets declared implementations serve components
        let item = dir::LanguageItem::from(interface);
        let target = self.language_type(item, &[])?;
        let verdict = self.decide_relation(origin, Relation::Subtype, field, target)?;

        Ok(verdict)
    }
}
