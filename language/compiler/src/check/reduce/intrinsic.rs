use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Reduce one compiler-recognized intrinsic application.
    pub(in crate::check) fn reduce_intrinsic_reference(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(item) = self.language_item(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };

        match item {
            // normalize collection constructors to structural types
            dir::LanguageItem::Array => self.normalize_array_application(origin, instance),
            dir::LanguageItem::Slice => self.normalize_slice_application(origin, instance),
            dir::LanguageItem::FixedArray => {
                self.normalize_fixed_array_application(origin, instance)
            }
            dir::LanguageItem::Dynamic => self.normalize_dynamic_application(origin, instance),
            dir::LanguageItem::Function => self.normalize_function_application(origin, instance),
            dir::LanguageItem::FunctionPointer => {
                self.normalize_function_pointer_application(origin, instance)
            }

            // reduce transparent compiler-known aliases
            dir::LanguageItem::Uppercase
            | dir::LanguageItem::Lowercase
            | dir::LanguageItem::Capitalize
            | dir::LanguageItem::Uncapitalize => {
                self.normalize_string_mapping_application(origin, item, instance)
            }
            dir::LanguageItem::NoInfer => self.normalize_noinfer_application(origin, instance),

            // normalize memory constructors to canonical written forms
            dir::LanguageItem::Managed => {
                self.normalize_form_constructor(origin, instance, dir::Form::Managed)
            }
            dir::LanguageItem::Owned => {
                self.normalize_form_constructor(origin, instance, dir::Form::Owned)
            }
            dir::LanguageItem::Raw => {
                self.normalize_form_constructor(origin, instance, dir::Form::Raw)
            }
            dir::LanguageItem::Borrowed => self.normalize_borrowed_constructor(origin, instance),
            dir::LanguageItem::Placed => self.normalize_placed_constructor(origin, instance),

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
            | dir::LanguageItem::WithAccess => self.reduce_memory_accessor(origin, item, instance),

            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return whether one declaration is a transparent compiler-known intrinsic alias.
    pub(in crate::check) fn is_transparent_intrinsic_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let is_transparent = self.language_item(symbol)?.is_some_and(|item| {
            item == dir::LanguageItem::NoInfer || item.string_mapping().is_some()
        });

        Ok(is_transparent)
    }

    /// Normalize one compiler-known string mapping alias application.
    fn normalize_string_mapping_application(
        &mut self,
        origin: Origin,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(mapping) = item.string_mapping() else {
            return Ok(Answer::Ready(None));
        };
        let [target] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Operation(dir::TypeOperation::StringMapping {
            mapping,
            target: *target,
        });

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one inference barrier intrinsic application.
    fn normalize_noinfer_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [target] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Operation(dir::TypeOperation::NoInfer(dir::UnaryType {
            target: *target,
        }));

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one Array intrinsic application.
    fn normalize_array_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Array(dir::ArrayType { element: *element });

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one Slice intrinsic application.
    fn normalize_slice_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Slice(dir::SliceType { element: *element });

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one FixedArray intrinsic application.
    fn normalize_fixed_array_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element, count] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::FixedArray(dir::FixedArrayType {
            element: *element,
            count: *count,
        });

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one Dynamic intrinsic application.
    fn normalize_dynamic_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [constraint] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Dynamic(dir::DynamicType {
            constraint: *constraint,
        });

        let ty = self.push_type_at_origin(origin, ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one Function intrinsic application.
    fn normalize_function_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(signature) = answer!(self.function_signature_from_application(origin, instance)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let environment = self.push_type_at_origin(origin, dir::Type::Unknown)?;
        let function = dir::Type::Function(dir::FunctionType {
            signature,
            environment,
        });

        let ty = self.push_type_at_origin(origin, function)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Normalize one FunctionPointer intrinsic application.
    fn normalize_function_pointer_application(
        &mut self,
        origin: Origin,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(signature) = answer!(self.function_signature_from_application(origin, instance)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let function = dir::Type::FunctionPointer(dir::FunctionPointerType { signature });

        let ty = self.push_type_at_origin(origin, function)?;

        Ok(Answer::Ready(Some(ty)))
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
        let parameters = answer!(self.reduce_type_root(origin, *parameters)?);

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
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(*return_type),
            is_generator: false,
        };
        let signature = dir::Type::FunctionSignature(function);
        let signature = self.push_type_at_origin(origin, signature)?;

        Ok(Answer::Ready(Some(signature)))
    }
}
