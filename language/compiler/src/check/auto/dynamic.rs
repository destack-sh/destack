use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

impl CheckState<'_> {
    /// Decide whether one type has a runtime representation after erasure.
    pub(in crate::check) fn satisfies_dynamic_safe(
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
        let kind = self.ty(ty)?;

        match kind {
            dir::Type::Variable(variable) => Ok(Answer::pending([Dependency::Variable(variable)])),
            // refinements constrain members without changing the base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_dynamic_safe(origin, refined.base, active)
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
            | dir::Type::Range(_) => Ok(Answer::Ready(true)),
            dir::Type::Reference(_) => Ok(Answer::Ready(false)),
            dir::Type::Instance(instance) => {
                let Some(definition) = self.definition(instance.symbol)? else {
                    return Ok(Answer::Ready(false));
                };
                let is_type_reference = !matches!(definition, dir::Definition::Extension(_));

                Ok(Answer::Ready(is_type_reference))
            }
            dir::Type::Parameter(parameter) | dir::Type::Erased(parameter) => {
                // prove through declared or assumed bounds
                let mut decision = Answer::Ready(false);
                for bound in self.parameter_bounds(origin, parameter)? {
                    decision = decision.or(self.satisfies_dynamic_safe(origin, bound, active)?);
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
                    decision = decision.or(self.satisfies_dynamic_safe(origin, bound, active)?);
                    if decision.is_ready_true() {
                        break;
                    }
                }

                Ok(decision)
            }
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Answer::Ready(false)),
            dir::Type::Form(form) => self.satisfies_dynamic_safe(origin, form.value, active),
            dir::Type::Dynamic(dynamic) => {
                self.satisfies_dynamic_safe(origin, dynamic.constraint, active)
            }
            dir::Type::Array(array) => self.satisfies_dynamic_safe(origin, array.element, active),
            dir::Type::FixedArray(array) => {
                self.satisfies_dynamic_safe(origin, array.element, active)
            }
            dir::Type::Slice(slice) => self.satisfies_dynamic_safe(origin, slice.element, active),
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_dynamic_safe(origin, ids, active)
            }
            dir::Type::Shape(shape) => {
                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .shape_fields(ty.module_id, shape.fields)?
                    .iter()
                    .map(|field| field.ty)
                    .collect();
                ids.extend(
                    self.type_ids(ty.module_id, shape.call_signatures)?
                        .iter()
                        .copied(),
                );
                ids.extend(
                    self.type_ids(ty.module_id, shape.construct_signatures)?
                        .iter()
                        .copied(),
                );
                ids.extend(
                    self.shape_index_signatures(ty.module_id, shape.index_signatures)?
                        .iter()
                        .flat_map(|signature| [signature.key_type, signature.value_type]),
                );

                self.all_dynamic_safe(origin, ids, active)
            }
            dir::Type::FunctionSignature(function) => {
                let function = self.type_signature(ty.module_id, function)?;
                self.satisfies_dynamic_safe_function(origin, ty.module_id, &function, active)
            }
            dir::Type::Function(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::FunctionPointer(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_dynamic_safe(origin, ids, active)
            }
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.all_dynamic_safe(origin, ids, active)
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

    /// Decide whether one function signature can be dynamically represented.
    fn satisfies_dynamic_safe_function(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: &dir::FunctionSignatureType,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Answer<bool>> {
        if !self.signature_generic_parameters(function)?.is_empty() {
            return Ok(Answer::Ready(false));
        }

        let receiver = function.this_parameter;
        let parameters: SmallVec<[dir::GlobalTypeId; 8]> = self
            .signature_parameters(module, function.parameters)?
            .iter()
            .map(|parameter| parameter.ty)
            .collect();
        let result = function.return_type;

        self.all_dynamic_safe(
            origin,
            receiver.into_iter().chain(parameters).chain(result),
            active,
        )
    }
}
