use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::sema::{CheckState, Origin, Verdict};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Decide whether one type has a runtime representation after erasure.
    pub(in crate::sema) fn decide_dynamic_safe(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        self.decide_recorded(
            origin,
            ty,
            dir::AutoInterface::DynamicSafe,
            active,
            |state, ty, active| state.decide_dynamic_safe_type(origin, ty, active),
        )
    }

    /// Decide dynamic safety for one active type.
    fn decide_dynamic_safe_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // read the head standing on the type
        let kind = self.ty(ty)?;

        // decide each runtime representation
        match kind {
            // leave an open variable or canonical hole undecided
            dir::Type::Variable(_) => Ok(Verdict::Ambiguous),
            // accept region terms, which have no runtime values
            dir::Type::Region(_) => Ok(Verdict::Holds),
            // look through the refinement to its base
            dir::Type::Refined(refined) => {
                let refined = self.type_refined(ty.module_id, refined)?;

                self.decide_dynamic_safe(origin, refined.base, active)
            }
            // represent terminal types directly
            dir::Type::Error
            | dir::Type::Never
            | dir::Type::Unknown
            | dir::Type::Void
            | dir::Type::Null
            | dir::Type::Undefined
            | dir::Type::Primitive(_)
            | dir::Type::Literal(_)
            | dir::Type::Key(_)
            | dir::Type::Static(_)
            | dir::Type::Intrinsic
            | dir::Type::Range(_) => Ok(Verdict::Holds),
            // represent a variant through its owning enum
            dir::Type::Variant(variant) => self.decide_dynamic_safe(origin, variant.owner, active),
            // refuse bare declaration references
            dir::Type::Reference(_) => Ok(Verdict::Fails),
            // represent nominal instances of every declaration but an extension
            dir::Type::Application(instance) => {
                let Some(definition) = self.definition(instance.symbol)? else {
                    return Ok(Verdict::Fails);
                };
                let is_type_reference = !matches!(*definition, dir::Definition::Extension(_));

                Ok(Verdict::decided(is_type_reference))
            }
            // accept memory parameters, they qualify storage alone
            dir::Type::Parameter(parameter) if self.is_memory_parameter(parameter)? => {
                Ok(Verdict::Holds)
            }
            // fail the interface for type parameters that survived substitution
            dir::Type::Parameter(_) => Ok(Verdict::Fails),
            // fail loudly on generic forms that survived substitution
            dir::Type::Erased(_) | dir::Type::This => Err(CompilerError::Internal {
                message: format!("generic type {ty:?} reached structural dynamic safety"),
            }),
            // refuse unreduced type operations
            dir::Type::Member(_) | dir::Type::Operation(_) => Ok(Verdict::Fails),
            // represent a memory form through its payload
            dir::Type::Form(form) => self.decide_dynamic_safe(origin, form.value, active),
            // represent an erased type through its constraint
            dir::Type::Dynamic(dynamic) => {
                self.decide_dynamic_safe(origin, dynamic.constraint, active)
            }
            // represent arrays and slices through their element
            dir::Type::FixedArray(array) => self.decide_dynamic_safe(origin, array.element, active),
            dir::Type::Slice(slice) => self.decide_dynamic_safe(origin, slice.element, active),
            // represent a tuple through every element
            dir::Type::Tuple(tuple) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .tuple_elements(ty.module_id, tuple.elements)?
                    .iter()
                    .map(|element| element.ty)
                    .collect();

                self.decide_all(ids, |state, id| {
                    state.decide_dynamic_safe(origin, id, active)
                })
            }
            // erase an object type with signatures through its members
            dir::Type::Object(shape) if shape.declares_signatures() => {
                let mut ids: SmallVec<[dir::GlobalTypeId; 8]> = self
                    .object_properties(ty.module_id, shape.properties)?
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
                    self.object_index_signatures(ty.module_id, shape.index_signatures)?
                        .iter()
                        .flat_map(|signature| [signature.key_type, signature.value_type]),
                );

                self.decide_all(ids, |state, id| {
                    state.decide_dynamic_safe(origin, id, active)
                })
            }
            // represent a plain object shape directly
            dir::Type::Object(_) => Ok(Verdict::Holds),
            // represent a signature through its erasure rules
            dir::Type::FunctionSignature(function) => {
                let function = self.type_signature(ty.module_id, function)?;

                self.decide_dynamic_safe_function(origin, ty.module_id, &function, active)
            }
            // represent function values and pointers through their signature
            dir::Type::Function(function) => {
                self.decide_dynamic_safe(origin, function.signature, active)
            }
            dir::Type::FunctionPointer(function) => {
                self.decide_dynamic_safe(origin, function.signature, active)
            }
            // represent a union through every alternative
            dir::Type::Union(union) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, union.elements)?);

                self.decide_all(ids, |state, id| {
                    state.decide_dynamic_safe(origin, id, active)
                })
            }
            // represent an intersection through every constituent
            dir::Type::Intersection(intersection) => {
                let ids: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(self.type_ids(ty.module_id, intersection.elements)?);

                self.decide_all(ids, |state, id| {
                    state.decide_dynamic_safe(origin, id, active)
                })
            }
        }
    }

    /// Decide whether one function signature can be dynamically represented.
    fn decide_dynamic_safe_function(
        &mut self,
        origin: Origin,
        module: ModuleId,
        function: &dir::FunctionSignatureType,
        active: &mut SmallVec<[dir::GlobalTypeId; 8]>,
    ) -> CompilerResult<Verdict> {
        // refuse signatures whose parameters require runtime type arguments
        if !self
            .signature_generic_parameters(module, function)?
            .is_empty()
        {
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

        self.decide_all(
            receiver.into_iter().chain(parameters).chain(result),
            |state, id| state.decide_dynamic_safe(origin, id, active),
        )
    }
}
