use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type has a runtime representation after erasure.
    pub(in crate::sema) fn satisfies_dynamic_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // unfold aliases, then use bounds declared by generic types
        let ty = self.normalize(origin, ty)?;
        if let Some(decision) =
            self.decide_generic_auto_interface(origin, ty, dir::AutoInterface::DynamicSafe)?
        {
            return Ok(Verdict::decided(decision));
        }

        // close recursive structural types coinductively
        if active.contains(&ty) {
            return Ok(Verdict::Holds);
        }
        active.push(ty);

        // restore the active stack after this decision
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
    ) -> CompilerResult<Verdict> {
        let kind = self.ty(ty)?;

        // decide each runtime representation
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) | dir::Type::Hole(_) => Ok(Verdict::Ambiguous),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.satisfies_dynamic_safe(origin, refined.base, active)
            }
            // represent scalar leaves directly
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Any
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Memory(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Range(_) => Ok(Verdict::Holds),
            // represent a variant through its owning enum
            dir::Type::Variant(variant) => {
                self.satisfies_dynamic_safe(origin, variant.owner, active)
            }
            // reject bare declaration references
            dir::Type::Reference(_) => Ok(Verdict::Fails),
            // represent nominal instances of every declaration but an extension
            dir::Type::Application(instance) => {
                let Some(definition) = self.definition(instance.symbol)? else {
                    return Ok(Verdict::Fails);
                };
                let is_type_reference = !matches!(definition, dir::Definition::Extension(_));

                Ok(Verdict::decided(is_type_reference))
            }
            // memory parameters qualify storage and impose none of their own
            dir::Type::Parameter(parameter) if self.is_memory_parameter(parameter) => {
                Ok(Verdict::Holds)
            }
            // fail the interface for type parameters that survived substitution
            dir::Type::Parameter(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Rigid(_) | dir::Type::Erased(_) | dir::Type::This => {
                Err(CompilerError::Internal {
                    message: format!("generic type {ty:?} reached structural dynamic safety"),
                })
            }
            // reject unreduced type operations
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Verdict::Fails),
            // represent a memory carrier through its payload
            dir::Type::Form(form) => self.satisfies_dynamic_safe(origin, form.value, active),
            // represent an erased type through its constraint
            dir::Type::Dynamic(dynamic) => {
                self.satisfies_dynamic_safe(origin, dynamic.constraint, active)
            }
            // represent arrays and slices through their element
            dir::Type::FixedArray(array) => {
                self.satisfies_dynamic_safe(origin, array.element, active)
            }
            dir::Type::Slice(slice) => self.satisfies_dynamic_safe(origin, slice.element, active),
            // represent a tuple through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.all_dynamic_safe(origin, ids, active)
            }
            // an object type declaring signatures erases through the members it declares
            dir::Type::Object(shape) if shape.declares_signatures() => {
                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .shape_properties(ty.module_id, shape.properties)?
                    .iter()
                    .flat_map(|field| field.access.types())
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
            // represent a plain object shape directly
            dir::Type::Object(_) => Ok(Verdict::Holds),
            // represent a signature through its erasure rules
            dir::Type::FunctionSignature(function) => {
                let function = self.type_signature(ty.module_id, function)?;

                self.satisfies_dynamic_safe_function(origin, ty.module_id, &function, active)
            }
            // represent function values and pointers through their signature
            dir::Type::Function(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::FunctionPointer(function) => {
                self.satisfies_dynamic_safe(origin, function.signature, active)
            }
            // represent a union through every alternative
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.all_dynamic_safe(origin, ids, active)
            }
            // represent an intersection through every constituent
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
    ) -> CompilerResult<Verdict> {
        let mut verdict = Verdict::Holds;
        for id in ids {
            verdict = verdict.and(self.satisfies_dynamic_safe(origin, id, active)?);
            if verdict == Verdict::Fails {
                return Ok(Verdict::Fails);
            }
        }

        Ok(verdict)
    }

    /// Decide whether one function signature can be dynamically represented.
    fn satisfies_dynamic_safe_function(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: &dir::FunctionSignatureType,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // reject signatures whose parameters require runtime type arguments
        if !self.signature_generic_parameters(function)?.is_empty() {
            return Ok(Verdict::Fails);
        }

        // require the receiver, parameters, and result to survive erasure
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
