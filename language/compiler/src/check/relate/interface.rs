use destack_dir as dir;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Dependency, MemberLookup, MemberRole, Origin, Relation, answer,
};

/// Compiler-known interface satisfied by structural checker rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum AutoInterface {
    /// Runtime erasure support for `Dynamic<T>`.
    DynamicSafe,
    /// Non-exclusive overwrite support.
    OverwriteStable,
}

impl AutoInterface {
    /// Return the compiler-known auto interface for one language item.
    pub(in crate::check) fn from_language_item(item: dir::LanguageItem) -> Option<Self> {
        match item {
            dir::LanguageItem::DynamicSafe => Some(Self::DynamicSafe),
            dir::LanguageItem::OverwriteStable => Some(Self::OverwriteStable),
            _ => None,
        }
    }

    /// Return the source-facing interface name.
    pub(in crate::check) fn name(self) -> &'static str {
        match self {
            Self::DynamicSafe => "DynamicSafe",
            Self::OverwriteStable => "OverwriteStable",
        }
    }
}

/// One member required by an applied interface.
#[derive(Debug, Clone, Copy)]
pub(in crate::check) struct InterfaceMember {
    /// The member space.
    pub(in crate::check) space: dir::MemberSpace,
    /// The member key.
    pub(in crate::check) key: dir::StaticKey,
    /// The member type, when the member carries a value.
    pub(in crate::check) ty: Option<dir::GlobalTypeId>,
    /// How the member participates in assignability.
    pub(in crate::check) role: MemberRole,
}

impl CheckState<'_> {
    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::check) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: AutoInterface,
    ) -> CompilerResult<Answer<bool>> {
        let mut visited = IndexSet::new();

        self.satisfies_auto_interface_inner(origin, ty, interface, &mut visited)
    }

    /// Decide auto interface satisfaction with active recursion tracked.
    fn satisfies_auto_interface_inner(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: AutoInterface,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        match interface {
            AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, visited),
            AutoInterface::OverwriteStable => self.satisfies_overwrite_stable(origin, ty, visited),
        }
    }

    /// Decide whether one type has a runtime representation after erasure.
    fn satisfies_dynamic_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.settled_root(ty)?;
        if !visited.insert(ty) {
            return Ok(Answer::Ready(true));
        }

        let result = self.satisfies_dynamic_safe_inner(origin, ty, visited);
        visited.swap_remove(&ty);

        result
    }

    /// Decide dynamic safety after claiming the active recursion slot.
    fn satisfies_dynamic_safe_inner(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_root(origin, ty)?);
        let kind = self.ty(ty)?.clone();

        match kind {
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Object
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::EnumMember(_)
            | dir::Type::Intrinsic
            | dir::Type::This
            | dir::Type::Range(_) => Ok(Answer::Ready(true)),
            dir::Type::Reference(_) => Ok(Answer::Ready(false)),
            dir::Type::Instance(instance) => {
                let Some(definition) = self.definition(instance.symbol) else {
                    return Ok(Answer::Ready(false));
                };
                let is_type_reference = !matches!(definition, dir::Definition::Extension(_));

                Ok(Answer::Ready(is_type_reference))
            }
            dir::Type::Parameter(parameter) => {
                let Some(binding) = self.generic_parameter(parameter) else {
                    return Ok(Answer::Ready(false));
                };
                let Some(constraint) = binding.constraint else {
                    return Ok(Answer::Ready(false));
                };

                self.satisfies_dynamic_safe(origin, constraint, visited)
            }
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Answer::Ready(false)),
            dir::Type::Form(role) => self.satisfies_dynamic_safe(origin, role.value, visited),
            dir::Type::Dynamic(dynamic) => {
                self.satisfies_dynamic_safe(origin, dynamic.constraint, visited)
            }
            dir::Type::Array(array) => self.satisfies_dynamic_safe(origin, array.element, visited),
            dir::Type::FixedArray(array) => {
                self.satisfies_dynamic_safe(origin, array.element, visited)
            }
            dir::Type::Slice(slice) => self.satisfies_dynamic_safe(origin, slice.element, visited),
            dir::Type::Tuple(tuple) => self.all_dynamic_safe(
                origin,
                tuple.elements.iter().map(|element| element.ty),
                visited,
            ),
            dir::Type::Shape(shape) => {
                let fields = shape.fields.iter().map(|field| field.ty);
                let calls = shape.call_signatures.iter().copied();
                let constructors = shape.construct_signatures.iter().copied();
                let indexes = shape
                    .index_signatures
                    .iter()
                    .flat_map(|signature| [signature.key_type, signature.value_type]);

                self.all_dynamic_safe(
                    origin,
                    fields.chain(calls).chain(constructors).chain(indexes),
                    visited,
                )
            }
            dir::Type::FunctionSignature(function) => {
                self.satisfies_dynamic_safe_function(origin, &function, visited)
            }
            dir::Type::Function(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, visited)
            }
            dir::Type::FunctionPointer(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, visited)
            }
            dir::Type::Union(union) => self.all_dynamic_safe(origin, union.elements, visited),
            dir::Type::Intersection(intersection) => {
                self.all_dynamic_safe(origin, intersection.elements, visited)
            }
        }
    }

    /// Decide whether every type in one iterator is dynamic-safe.
    fn all_dynamic_safe(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.satisfies_dynamic_safe(origin, id, visited)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one type can be overwritten through non-exclusive access.
    fn satisfies_overwrite_stable(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.settled_root(ty)?;
        if !visited.insert(ty) {
            return Ok(Answer::Ready(true));
        }

        let result = self.satisfies_overwrite_stable_inner(origin, ty, visited);
        visited.swap_remove(&ty);

        result
    }

    /// Decide overwrite stability after claiming the active recursion slot.
    fn satisfies_overwrite_stable_inner(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_root(origin, ty)?);
        let kind = self.ty(ty)?.clone();

        match kind {
            dir::Type::Variable(variable) => {
                let representative = self.solver.representative(variable)?;

                Ok(Answer::pending([Dependency::Variable(representative)]))
            }
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::EnumMember(_)
            | dir::Type::This
            | dir::Type::Range(_)
            | dir::Type::Reference(_) => Ok(Answer::Ready(true)),
            dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Object
            | dir::Type::Intrinsic
            | dir::Type::Member(_)
            | dir::Type::Operation(_)
            | dir::Type::Dynamic(_)
            | dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::Union(_) => Ok(Answer::Ready(false)),
            dir::Type::Parameter(parameter) => {
                let Some(binding) = self.generic_parameter(parameter) else {
                    return Ok(Answer::Ready(false));
                };
                let Some(constraint) = binding.constraint else {
                    return Ok(Answer::Ready(false));
                };

                self.satisfies_overwrite_stable(origin, constraint, visited)
            }
            dir::Type::Form(role) => match role.form {
                dir::Form::Managed | dir::Form::Borrowed { .. } | dir::Form::Raw => {
                    Ok(Answer::Ready(true))
                }
                dir::Form::Owned => Ok(Answer::Ready(false)),
                dir::Form::Placed { .. } | dir::Form::Readonly => {
                    self.satisfies_overwrite_stable(origin, role.value, visited)
                }
            },
            dir::Type::Instance(instance) => {
                self.satisfies_overwrite_stable_instance(origin, instance, visited)
            }
            dir::Type::Array(_) => Ok(Answer::Ready(true)),
            dir::Type::FixedArray(array) => {
                self.satisfies_overwrite_stable(origin, array.element, visited)
            }
            dir::Type::Slice(_) => Ok(Answer::Ready(true)),
            dir::Type::Tuple(tuple) => self.all_overwrite_stable(
                origin,
                tuple.elements.iter().map(|element| element.ty),
                visited,
            ),
            dir::Type::Shape(shape) => {
                let fields = shape.fields.iter().map(|field| field.ty);

                self.all_overwrite_stable(origin, fields, visited)
            }
            dir::Type::FunctionPointer(_) => Ok(Answer::Ready(true)),
            dir::Type::Intersection(intersection) => {
                self.all_overwrite_stable(origin, intersection.elements, visited)
            }
        }
    }

    /// Decide whether one nominal application can be overwritten without exclusivity.
    fn satisfies_overwrite_stable_instance(
        &mut self,
        origin: Origin,
        instance: dir::GenericInstance,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(definition) = self.definition(instance.symbol).cloned() else {
            return Ok(Answer::Ready(false));
        };

        match definition {
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_overwrite_stable(origin, definition.value, visited)
            }
            dir::Definition::Struct(definition) => {
                let fields = definition
                    .members
                    .into_iter()
                    .filter_map(|member| match member {
                        dir::DefinitionMember::Field(field) => Some(field.ty),
                        _ => None,
                    });

                self.all_substituted_overwrite_stable(origin, &instance, fields, visited)
            }
            dir::Definition::Class(_) | dir::Definition::Interface(_) => Ok(Answer::Ready(true)),
            dir::Definition::Enum(_) => Ok(Answer::Ready(false)),
            dir::Definition::Newtype(definition) => self.all_substituted_overwrite_stable(
                origin,
                &instance,
                [definition.value],
                visited,
            ),
            dir::Definition::Extension(_) => Ok(Answer::Ready(false)),
        }
    }

    /// Decide overwrite stability for substituted nominal member types.
    fn all_substituted_overwrite_stable(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let substitution = self.instance_substitution(instance)?;
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut decision = Answer::Ready(true);
        for id in ids {
            let id = if substitution.is_empty() {
                id
            } else {
                self.fold_type(module, source, id, substitution.rewrite())?
            };
            decision = decision.and(self.satisfies_overwrite_stable(origin, id, visited)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether every type in one iterator is overwrite-stable.
    fn all_overwrite_stable(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.satisfies_overwrite_stable(origin, id, visited)?);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Decide whether one function signature can be dynamically represented.
    fn satisfies_dynamic_safe_function(
        &mut self,
        origin: Origin,
        function: &dir::FunctionSignatureType,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        if !self.signature_generic_parameters(function)?.is_empty() {
            return Ok(Answer::Ready(false));
        }

        let receiver = function.this_parameter;
        let parameters = function.parameters.iter().map(|parameter| parameter.ty);
        let result = function.return_type;

        self.all_dynamic_safe(
            origin,
            receiver.into_iter().chain(parameters).chain(result),
            visited,
        )
    }

    /// Decide whether one source exposes every member of one interface application.
    pub(in crate::check) fn decide_interface_satisfied(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target_instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<bool>> {
        let module = origin.module();
        let members = answer!(self.interface_members(origin, target_instance, source)?);

        // require each member from the source
        let mut decision = Answer::Ready(true);
        for member in members {
            let lookup = self.lookup_member(origin, module, source, member.space, member.key)?;

            let found = match lookup {
                MemberLookup::Field(ty) => Some(ty),
                MemberLookup::Found(candidates) => candidates.first().map(|candidate| candidate.ty),
                MemberLookup::Missing => None,
                MemberLookup::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            };
            let Some(found) = found else {
                return Ok(Answer::Ready(false));
            };

            // associated types without values only need presence
            let Some(member_type) = member.ty else {
                continue;
            };

            let assignment = if member.role.uses_method_assignability() {
                self.decide_method_assignable(origin, found, member_type)?
            } else {
                self.decide_relation(origin, Relation::Assignable, found, member_type)?
            };
            decision = decision.and(assignment);
            if decision.is_ready_false() {
                return Ok(decision);
            }
        }

        Ok(decision)
    }

    /// Return the members required by one interface application.
    pub(in crate::check) fn interface_members(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<SmallVec<[InterfaceMember; 8]>>> {
        let closure = answer!(self.heritage_closure(origin, instance)?);
        let mut members = SmallVec::<[InterfaceMember; 8]>::new();

        // collect inherited members before direct members
        for application in closure.applications {
            self.collect_interface_members(origin, &application.instance, receiver, &mut members)?;
        }
        self.collect_interface_members(origin, instance, receiver, &mut members)?;

        Ok(Answer::Ready(members))
    }

    /// Collect direct members required by one applied interface.
    fn collect_interface_members(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        receiver: dir::GlobalTypeId,
        required: &mut SmallVec<[InterfaceMember; 8]>,
    ) -> CompilerResult<()> {
        let Some(dir::Definition::Interface(definition)) = self.definition(instance.symbol) else {
            return Ok(());
        };
        let members = definition.members.clone();
        let substitution = self
            .instance_substitution(instance)?
            .with_receiver(receiver);
        let module = origin.module();
        let source = self.origin_source_node(origin)?;

        // collect direct interface members in the applied view
        for member in members {
            let Some(role) = MemberRole::from_definition(&member) else {
                continue;
            };
            let Some(key) = member.key() else {
                continue;
            };
            let ty = match member.ty() {
                Some(ty) if !substitution.is_empty() => {
                    Some(self.fold_type(module, source, ty, substitution.rewrite())?)
                }
                ty => ty,
            };
            required.push(InterfaceMember {
                space: member.space(),
                key,
                ty,
                role,
            });
        }

        Ok(())
    }
}
