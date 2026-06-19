use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin};

impl CheckState<'_> {
    /// Reduce one compiler-recognized intrinsic application.
    pub(in crate::check) fn evaluate_intrinsic_reference(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(item) = self.environment.language.item(instance.symbol) else {
            return Ok(Answer::Ready(None));
        };

        match item {
            // normalize collection constructors to structural views
            dir::LanguageItem::Array => self.evaluate_array_application(origin, instance),
            dir::LanguageItem::Slice => self.evaluate_slice_application(origin, instance),
            dir::LanguageItem::FixedArray => {
                self.evaluate_fixed_array_application(origin, instance)
            }
            dir::LanguageItem::Dynamic => self.evaluate_dynamic_application(origin, instance),
            dir::LanguageItem::Function => self.evaluate_function_application(origin, instance),
            dir::LanguageItem::FunctionPointer => {
                self.evaluate_function_pointer_application(origin, instance)
            }

            // normalize memory constructors to canonical written forms
            dir::LanguageItem::Managed => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Managed)
            }
            dir::LanguageItem::Owned => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Owned)
            }
            dir::LanguageItem::Raw => {
                self.evaluate_form_constructor(origin, instance, dir::Form::Raw)
            }
            dir::LanguageItem::Borrowed => self.evaluate_borrowed_constructor(origin, instance),
            dir::LanguageItem::Placed => self.evaluate_placed_constructor(origin, instance),

            // evaluate memory accessors over closed form chains
            dir::LanguageItem::PayloadOf
            | dir::LanguageItem::BaseOf
            | dir::LanguageItem::OwnershipOf
            | dir::LanguageItem::OwnershipOr
            | dir::LanguageItem::PlaceOf
            | dir::LanguageItem::PlaceOr
            | dir::LanguageItem::PlaceIn
            | dir::LanguageItem::SpaceOf
            | dir::LanguageItem::SpaceOr
            | dir::LanguageItem::LifetimeOf
            | dir::LanguageItem::LifetimeOr
            | dir::LanguageItem::AccessOf
            | dir::LanguageItem::AccessOr
            | dir::LanguageItem::IsManaged
            | dir::LanguageItem::IsOwned
            | dir::LanguageItem::IsBorrowed
            | dir::LanguageItem::IsRaw
            | dir::LanguageItem::IsShared
            | dir::LanguageItem::IsSharedIn
            | dir::LanguageItem::WithBase
            | dir::LanguageItem::WithOwnership
            | dir::LanguageItem::WithPlace
            | dir::LanguageItem::WithSpace
            | dir::LanguageItem::WithLifetime
            | dir::LanguageItem::WithAccess => {
                self.evaluate_memory_accessor(origin, item, instance)
            }

            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Normalize one Array intrinsic application.
    fn evaluate_array_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let view = dir::Type::Array(dir::ArrayType { element: *element });

        self.push_intrinsic_view(origin, view)
    }

    /// Normalize one Slice intrinsic application.
    fn evaluate_slice_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let view = dir::Type::Slice(dir::SliceType { element: *element });

        self.push_intrinsic_view(origin, view)
    }

    /// Normalize one FixedArray intrinsic application.
    fn evaluate_fixed_array_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element, count] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let view = dir::Type::FixedArray(dir::FixedArrayType {
            element: *element,
            count: *count,
        });

        self.push_intrinsic_view(origin, view)
    }

    /// Normalize one Dynamic intrinsic application.
    fn evaluate_dynamic_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [constraint] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let view = dir::Type::Dynamic(dir::DynamicType {
            constraint: *constraint,
        });

        self.push_intrinsic_view(origin, view)
    }

    /// Normalize one Function intrinsic application.
    fn evaluate_function_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let signature = match self.function_signature_from_application(origin, instance)? {
            Answer::Ready(Some(signature)) => signature,
            Answer::Ready(None) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let source = self.origin_source_node(origin)?;
        let environment = self.push_type(origin.module(), dir::Type::Unknown, source)?;
        let function = dir::Type::Function(dir::FunctionType {
            signature,
            environment,
        });

        self.push_intrinsic_view(origin, function)
    }

    /// Normalize one FunctionPointer intrinsic application.
    fn evaluate_function_pointer_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let signature = match self.function_signature_from_application(origin, instance)? {
            Answer::Ready(Some(signature)) => signature,
            Answer::Ready(None) => return Ok(Answer::Ready(None)),
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };
        let function = dir::Type::FunctionPointer(dir::FunctionPointerType { signature });

        self.push_intrinsic_view(origin, function)
    }

    /// Return one signature from a callable intrinsic application.
    fn function_signature_from_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [parameters, return_type] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let parameters = match self.evaluate_root(origin, *parameters)? {
            Answer::Ready(parameters) => parameters,
            Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
        };

        // read the parameter tuple
        let parameters = match self.ty(parameters)? {
            dir::Type::Void => Vec::new(),
            dir::Type::Tuple(tuple) => tuple
                .elements
                .iter()
                .map(|element| dir::FunctionParameterType {
                    ty: element.ty,
                    static_parameter: None,
                    is_optional: element.is_optional,
                    is_rest: element.is_rest,
                })
                .collect(),
            _ => return Ok(Answer::Ready(None)),
        };

        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            generic_parameters: Vec::new(),
            this_parameter: None,
            parameters,
            return_type: Some(*return_type),
            is_generator: false,
        };
        let signature = dir::Type::FunctionSignature(function);
        let source = self.origin_source_node(origin)?;
        let signature = self.push_type(origin.module(), signature, source)?;

        Ok(Answer::Ready(Some(signature)))
    }

    /// Allocate one normalized intrinsic view.
    fn push_intrinsic_view(
        &mut self,
        origin: Origin,
        view: dir::Type,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let source = self.origin_source_node(origin)?;
        let view = self.push_type(origin.module(), view, source)?;

        Ok(Answer::Ready(Some(view)))
    }
}
