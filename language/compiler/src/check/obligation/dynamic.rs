use destack_artifact::DiagnosticBuilder;
use destack_dir as dir;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{Answer, CheckError, CheckState, Dependency, Origin};

impl CheckState<'_> {
    /// Check whether one type can be erased into `Dynamic<T>`.
    pub(in crate::check) fn check_dynamic_safe(
        &mut self,
        source: dir::GlobalNodeIdAny,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<DiagnosticBuilder<CheckError>>>> {
        let origin = Origin::Node(source);
        let ty = match self.evaluate_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let mut visited = IndexSet::new();
        let decision = self.is_dynamic_safe(origin, ty, &mut visited)?;

        match decision {
            Answer::Ready(true) => Ok(Answer::Ready(None)),
            Answer::Ready(false) => {
                let (module, anchor) = self.source_anchor(source);
                let ty = self.format_type(ty);
                let error = CheckError::DynamicSafetyNotSatisfied { anchor, module, ty };

                Ok(Answer::Ready(Some(error.into())))
            }
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
        }
    }

    /// Decide whether one reduced type has a runtime-checkable erased surface.
    fn is_dynamic_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = self.shallow_resolve(ty)?;
        if !visited.insert(ty) {
            return Ok(Answer::Ready(true));
        }

        let result = self.is_dynamic_safe_inner(origin, ty, visited);
        visited.swap_remove(&ty);

        result
    }

    /// Decide dynamic safety after cycle protection has claimed this type.
    fn is_dynamic_safe_inner(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        let ty = match self.evaluate_root(origin, ty)? {
            Answer::Ready(ty) => ty,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let kind = self.ty(ty)?.clone();

        match kind {
            dir::Type::Variable(variable) => {
                let representative = self.variables.representative(variable)?;

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
            | dir::Type::Intrinsic
            | dir::Type::This
            | dir::Type::Range(_) => Ok(Answer::Ready(true)),
            dir::Type::Reference(instance) => {
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

                self.is_dynamic_safe(origin, constraint, visited)
            }
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Answer::Ready(false)),
            dir::Type::Form(form) => self.is_dynamic_safe(origin, form.value, visited),
            dir::Type::Dynamic(dynamic) => {
                self.is_dynamic_safe(origin, dynamic.constraint, visited)
            }
            dir::Type::Array(array) => self.is_dynamic_safe(origin, array.element, visited),
            dir::Type::FixedArray(array) => self.is_dynamic_safe(origin, array.element, visited),
            dir::Type::Slice(slice) => self.is_dynamic_safe(origin, slice.element, visited),
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
            dir::Type::Function(function) => {
                self.is_dynamic_safe_function(origin, &function, visited)
            }
            dir::Type::Closure(closure) => self.is_dynamic_safe(origin, closure.function, visited),
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
        let mut blockers = Vec::new();
        for id in ids {
            match self.is_dynamic_safe(origin, id, visited)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Pending(pending) => blockers.extend(pending),
            }
        }

        if blockers.is_empty() {
            Ok(Answer::Ready(true))
        } else {
            Ok(Answer::Pending(blockers.into()))
        }
    }

    /// Decide whether one function surface can be dynamically represented.
    fn is_dynamic_safe_function(
        &mut self,
        origin: Origin,
        function: &dir::FunctionType,
        visited: &mut IndexSet<dir::GlobalTypeId>,
    ) -> CompilerResult<Answer<bool>> {
        if !function.generic_parameters.is_empty() {
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
}
