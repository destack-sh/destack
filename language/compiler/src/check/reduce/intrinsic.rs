use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

impl CheckState<'_> {
    /// Reduce one compiler-recognized intrinsic application.
    /// `module` is the owner of `instance`'s argument list.
    pub(in crate::check) fn reduce_intrinsic_reference(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(item) = self.language_item(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };

        match item {
            // reduce collection aliases to structural types
            dir::LanguageItem::Array => self.reduce_array_application(origin, module, instance),
            dir::LanguageItem::Slice => self.reduce_slice_application(origin, module, instance),
            dir::LanguageItem::FixedArray => {
                self.reduce_fixed_array_application(origin, module, instance)
            }
            dir::LanguageItem::Dynamic => self.reduce_dynamic_application(origin, module, instance),
            dir::LanguageItem::Function => {
                self.reduce_function_application(origin, module, instance)
            }
            dir::LanguageItem::FunctionPointer => {
                self.reduce_function_pointer_application(origin, module, instance)
            }

            // reduce transparent compiler-known aliases
            dir::LanguageItem::Uppercase
            | dir::LanguageItem::Lowercase
            | dir::LanguageItem::Capitalize
            | dir::LanguageItem::Uncapitalize => {
                self.reduce_string_mapping_application(origin, module, item, instance)
            }
            dir::LanguageItem::NoInfer => self.reduce_noinfer_application(origin, module, instance),
            dir::LanguageItem::Awaited => self.reduce_awaited_application(origin, module, instance),
            dir::LanguageItem::Readonly => {
                self.reduce_form_constructor(origin, module, instance, dir::Form::Readonly)
            }

            // reduce memory aliases to canonical written forms
            dir::LanguageItem::Managed => {
                self.reduce_form_constructor(origin, module, instance, dir::Form::Managed)
            }
            dir::LanguageItem::Owned => {
                self.reduce_form_constructor(origin, module, instance, dir::Form::Owned)
            }
            dir::LanguageItem::Raw => {
                self.reduce_form_constructor(origin, module, instance, dir::Form::Raw)
            }
            dir::LanguageItem::Borrowed => {
                self.reduce_borrowed_constructor(origin, module, instance)
            }
            dir::LanguageItem::Placed => self.reduce_placed_constructor(origin, module, instance),

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
                self.reduce_memory_accessor(origin, module, item, instance)
            }

            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Return whether one declaration is a transparent compiler-known intrinsic alias.
    pub(in crate::check) fn is_transparent_intrinsic_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let is_transparent = self.language_item(symbol)?.is_some_and(|item| {
            matches!(
                item,
                dir::LanguageItem::Awaited
                    | dir::LanguageItem::NoInfer
                    | dir::LanguageItem::Readonly
            ) || item.string_mapping().is_some()
        });

        Ok(is_transparent)
    }

    /// Reduce one compiler-known string mapping alias application.
    fn reduce_string_mapping_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(mapping) = item.string_mapping() else {
            return Ok(Answer::Ready(None));
        };
        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Operation(dir::TypeOperation::StringMapping {
            mapping,
            target: *target,
        });

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one inference barrier intrinsic application.
    fn reduce_noinfer_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Operation(dir::TypeOperation::NoInfer(dir::UnaryType {
            target: *target,
        }));

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one awaited-value intrinsic application.
    fn reduce_awaited_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Operation(dir::TypeOperation::Awaited(dir::UnaryType {
            target: *target,
        }));

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one Array intrinsic application.
    fn reduce_array_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Array(dir::ArrayType { element: *element });

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one Slice intrinsic application.
    fn reduce_slice_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Slice(dir::SliceType { element: *element });

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one FixedArray intrinsic application.
    fn reduce_fixed_array_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [element, count] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::FixedArray(dir::FixedArrayType {
            element: *element,
            count: *count,
        });

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one Dynamic intrinsic application.
    fn reduce_dynamic_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [constraint] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let ty = dir::Type::Dynamic(dir::DynamicType {
            constraint: *constraint,
        });

        let ty = self.intern_type(origin.module(), ty)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one Function intrinsic application.
    fn reduce_function_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(signature) =
            answer!(self.function_signature_from_application(origin, module, instance)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let environment = self.intern_type(origin.module(), dir::Type::Unknown)?;
        let function = dir::Type::Function(dir::FunctionType {
            signature,
            environment,
        });

        let ty = self.intern_type(origin.module(), function)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Reduce one FunctionPointer intrinsic application.
    fn reduce_function_pointer_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let Some(signature) =
            answer!(self.function_signature_from_application(origin, module, instance)?)
        else {
            return Ok(Answer::Ready(None));
        };
        let function = dir::Type::FunctionPointer(dir::FunctionPointerType { signature });

        let ty = self.intern_type(origin.module(), function)?;

        Ok(Answer::Ready(Some(ty)))
    }

    /// Return one signature from a callable intrinsic application.
    fn function_signature_from_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let [parameters, return_type] = self.type_ids(module, instance.arguments)? else {
            return Ok(Answer::Ready(None));
        };
        let (parameters, return_type) = (*parameters, *return_type);
        let parameters = answer!(self.reduce_type_head(origin, parameters)?);

        // read the parameter tuple
        let parameters = match self.ty(parameters)? {
            dir::Type::Void => Vec::new(),
            dir::Type::Tuple(tuple) => self
                .tuple_elements(parameters.module_id, tuple.elements)?
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
        let parameters = self.intern_parameters(origin.module(), &parameters)?;

        let function = dir::FunctionSignatureType {
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
        };
        let signature = dir::Type::FunctionSignature(function);
        let signature = self.intern_type(origin.module(), signature)?;

        Ok(Answer::Ready(Some(signature)))
    }
}
