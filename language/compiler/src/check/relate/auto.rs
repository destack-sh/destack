use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

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

impl CheckState<'_> {
    /// Decide whether one type satisfies a compiler-known auto interface.
    pub(in crate::check) fn satisfies_auto_interface(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        interface: AutoInterface,
    ) -> CompilerResult<Answer<bool>> {
        let mut active = SmallVec::<[dir::GlobalTypeId; 8]>::new();

        match interface {
            AutoInterface::DynamicSafe => self.satisfies_dynamic_safe(origin, ty, &mut active),
            AutoInterface::OverwriteStable => {
                self.satisfies_overwrite_stable(origin, ty, &mut active)
            }
        }
    }

    /// Decide whether one type has a runtime representation after erasure.
    fn satisfies_dynamic_safe(
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

        let result = self.decide_dynamic_safe_type(origin, ty, active);
        active.pop();

        result
    }

    /// Decide dynamic safety for one active type.
    fn decide_dynamic_safe_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
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
            | dir::Type::Key(_)
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

                self.satisfies_dynamic_safe(origin, constraint, active)
            }
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Answer::Ready(false)),
            dir::Type::Form(role) => self.satisfies_dynamic_safe(origin, role.value, active),
            dir::Type::Dynamic(dynamic) => {
                self.satisfies_dynamic_safe(origin, dynamic.constraint, active)
            }
            dir::Type::Array(array) => self.satisfies_dynamic_safe(origin, array.element, active),
            dir::Type::FixedArray(array) => {
                self.satisfies_dynamic_safe(origin, array.element, active)
            }
            dir::Type::Slice(slice) => self.satisfies_dynamic_safe(origin, slice.element, active),
            dir::Type::Tuple(tuple) => self.all_dynamic_safe(
                origin,
                tuple.elements.iter().map(|element| element.ty),
                active,
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
                    active,
                )
            }
            dir::Type::FunctionSignature(function) => {
                self.satisfies_dynamic_safe_function(origin, &function, active)
            }
            dir::Type::Function(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::FunctionPointer(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::Union(union) => self.all_dynamic_safe(origin, union.elements, active),
            dir::Type::Intersection(intersection) => {
                self.all_dynamic_safe(origin, intersection.elements, active)
            }
        }
    }

    /// Decide whether every type in one iterator is dynamic-safe.
    fn all_dynamic_safe(
        &mut self,
        origin: Origin,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.satisfies_dynamic_safe(origin, id, active)?);
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
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.settled_root(ty)?;
        if active.contains(&ty) {
            return Ok(Answer::Ready(true));
        }
        active.push(ty);

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
    ) -> CompilerResult<Answer<bool>> {
        let ty = answer!(self.reduce_type_head(origin, ty)?);
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
            | dir::Type::Key(_)
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

                self.satisfies_overwrite_stable(origin, constraint, active)
            }
            dir::Type::Form(role) => match role.form {
                dir::Form::Managed | dir::Form::Borrowed { .. } | dir::Form::Raw => {
                    Ok(Answer::Ready(true))
                }
                dir::Form::Owned => Ok(Answer::Ready(false)),
                dir::Form::Placed { .. } | dir::Form::Readonly => {
                    self.satisfies_overwrite_stable(origin, role.value, active)
                }
            },
            dir::Type::Instance(instance) => {
                self.satisfies_overwrite_stable_instance(origin, instance, active)
            }
            dir::Type::Array(_) => Ok(Answer::Ready(true)),
            dir::Type::FixedArray(array) => {
                self.satisfies_overwrite_stable(origin, array.element, active)
            }
            dir::Type::Slice(_) => Ok(Answer::Ready(true)),
            dir::Type::Tuple(tuple) => self.all_overwrite_stable(
                origin,
                tuple.elements.iter().map(|element| element.ty),
                active,
            ),
            dir::Type::Shape(shape) => {
                let fields = shape.fields.iter().map(|field| field.ty);

                self.all_overwrite_stable(origin, fields, active)
            }
            dir::Type::FunctionPointer(_) => Ok(Answer::Ready(true)),
            dir::Type::Intersection(intersection) => {
                self.all_overwrite_stable(origin, intersection.elements, active)
            }
        }
    }

    /// Decide whether one nominal application can be overwritten without exclusivity.
    fn satisfies_overwrite_stable_instance(
        &mut self,
        origin: Origin,
        instance: dir::GenericInstance,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let Some(definition) = self.definition(instance.symbol).cloned() else {
            return Ok(Answer::Ready(false));
        };

        match definition {
            dir::Definition::TypeAlias(definition) => {
                self.satisfies_overwrite_stable(origin, definition.value, active)
            }
            dir::Definition::Struct(definition) => {
                let mut fields = SmallVec::<[_; 8]>::new();
                for member in &definition.members {
                    if let dir::DefinitionMember::Field(_) = member {
                        if let Some(ty) = answer!(self.definition_member_type(member)?) {
                            fields.push(ty);
                        }
                    }
                }

                self.all_applied_overwrite_stable(origin, &instance, fields, active)
            }
            dir::Definition::Class(_) | dir::Definition::Interface(_) => Ok(Answer::Ready(true)),
            dir::Definition::Enum(_) => Ok(Answer::Ready(false)),
            dir::Definition::Newtype(definition) => {
                self.all_applied_overwrite_stable(origin, &instance, [definition.value], active)
            }
            dir::Definition::Extension(_) => Ok(Answer::Ready(false)),
        }
    }

    /// Decide overwrite stability for applied nominal component types.
    fn all_applied_overwrite_stable(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
        ids: impl IntoIterator<Item = dir::GlobalTypeId>,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let substitution = self.instance_substitution(instance)?;
        let module = origin.module();
        let source = self.origin_source_node(origin)?;
        let mut decision = Answer::Ready(true);
        for id in ids {
            let id = self.substitute_type(module, source, id, &substitution)?;
            decision = decision.and(self.satisfies_overwrite_stable(origin, id, active)?);
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
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        let mut decision = Answer::Ready(true);
        for id in ids {
            decision = decision.and(self.satisfies_overwrite_stable(origin, id, active)?);
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
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
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
            active,
        )
    }
}
