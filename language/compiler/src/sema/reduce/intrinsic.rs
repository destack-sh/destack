use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

impl CheckState<'_> {
    /// Reduce one nominal declaration standing for a builtin type in type space.
    pub(in crate::sema) fn reduce_representation_declaration(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let primitive = match self.language_item(instance.symbol)? {
            Some(dir::LanguageItem::String) => dir::PrimitiveType::String,
            Some(dir::LanguageItem::BigInt) => dir::PrimitiveType::Bigint,
            Some(dir::LanguageItem::Slice) => {
                return self.reduce_slice_application(origin, module, instance);
            }
            _ => return Ok(None),
        };
        let ty = self.intern_type(dir::Type::Primitive(primitive))?;

        Ok(Some(ty))
    }

    /// Reduce one intrinsic alias application.
    pub(in crate::sema) fn reduce_intrinsic_reference(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(item) = self.language_item(instance.symbol)? else {
            return Ok(None);
        };

        match item {
            // reduce collection aliases to structural types
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
            dir::LanguageItem::Unsigned => {
                self.reduce_unsigned_application(origin, module, instance)
            }
            dir::LanguageItem::Awaited => self.reduce_awaited_application(origin, module, instance),

            // evaluate memory accessors over closed form chains
            dir::LanguageItem::AccessOf | dir::LanguageItem::WithAccess => {
                self.reduce_memory_accessor(origin, module, item, instance)
            }

            // leave every other language item to ordinary application
            _ => Ok(None),
        }
    }

    /// Reduce one unsigned integer type projection.
    fn reduce_unsigned_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let arguments = self.type_ids(module, instance.arguments)?;
        let [target] = arguments else {
            return Ok(None);
        };

        let target = self.normalize(origin, *target)?;

        // project only resolved builtin integers to their unsigned width
        let dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) = self.ty(target)? else {
            return Ok(None);
        };

        let ty = dir::Type::Primitive(dir::PrimitiveType::Integer(integer.unsigned()));
        let ty = self.intern_type(ty)?;

        Ok(Some(ty))
    }

    /// Return whether one declaration is a compiler-known alias reducing when applied.
    pub(in crate::sema) fn is_transparent_intrinsic_alias(
        &mut self,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<bool> {
        let is_transparent = self
            .language_item(symbol)?
            .is_some_and(|item| item.kind() == dir::LanguageItemKind::Type);

        Ok(is_transparent)
    }

    /// Reduce one compiler-known string mapping alias application.
    fn reduce_string_mapping_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        item: dir::LanguageItem,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let Some(mapping) = item.string_mapping() else {
            return Ok(None);
        };

        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };

        let operation = dir::TypeOperation::StringMapping {
            mapping,
            target: *target,
        };
        let ty = self.intern_operation(operation)?;

        Ok(Some(ty))
    }

    /// Reduce one inference barrier intrinsic application.
    fn reduce_noinfer_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };

        let operation = dir::TypeOperation::NoInfer(dir::UnaryType { target: *target });
        let ty = self.intern_operation(operation)?;

        Ok(Some(ty))
    }

    /// Reduce one awaited-value intrinsic application.
    fn reduce_awaited_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [target] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };

        let operation = dir::TypeOperation::Awaited(dir::UnaryType { target: *target });
        let ty = self.intern_operation(operation)?;

        Ok(Some(ty))
    }

    /// Reduce one Slice intrinsic application.
    fn reduce_slice_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [element] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };
        let element = *element;
        let ty = self.intern_type(dir::Type::Slice(dir::SliceType { element }))?;

        Ok(Some(ty))
    }

    /// Reduce one FixedArray intrinsic application.
    fn reduce_fixed_array_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [element, count] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };

        let ty = dir::Type::FixedArray(dir::FixedArrayType {
            element: *element,
            count: *count,
        });
        let ty = self.intern_type(ty)?;

        Ok(Some(ty))
    }

    /// Reduce one Dynamic intrinsic application.
    fn reduce_dynamic_application(
        &mut self,
        _origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [constraint] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };
        let constraint = *constraint;
        let ty = self.intern_type(dir::Type::Dynamic(dir::DynamicType { constraint }))?;

        Ok(Some(ty))
    }

    /// Reduce one Function intrinsic application.
    fn reduce_function_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [parameters, return_type, receiver] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };
        let (parameters, return_type, receiver) = (*parameters, *return_type, *receiver);

        // require a receiver mode literal or an open receiver term
        let receiver = self.normalize(origin, receiver)?;
        match self.ty(receiver)? {
            dir::Type::Literal(dir::Literal::String(text)) => {
                if dir::ReceiverMode::from_text(self.strings().get(text)).is_none() {
                    return Ok(None);
                }
            }
            dir::Type::Variable(_) | dir::Type::Parameter(_) => {}
            _ => return Ok(None),
        }

        // intern the fat callable over the read signature
        let Some(signature) =
            self.function_signature_from_application(origin, parameters, return_type)?
        else {
            return Ok(None);
        };

        // intern the callable over the read signature
        let function = dir::Type::Function(dir::FunctionType {
            signature,
            receiver,
        });
        let ty = self.intern_type(function)?;

        Ok(Some(ty))
    }

    /// Reduce one FunctionPointer intrinsic application.
    fn reduce_function_pointer_application(
        &mut self,
        origin: Origin,
        module: ModuleId,
        instance: &dir::GenericApplication,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let [parameters, return_type] = self.type_ids(module, instance.arguments)? else {
            return Ok(None);
        };

        let (parameters, return_type) = (*parameters, *return_type);
        let Some(signature) =
            self.function_signature_from_application(origin, parameters, return_type)?
        else {
            return Ok(None);
        };

        let function = dir::Type::FunctionPointer(dir::FunctionPointerType { signature });
        let ty = self.intern_type(function)?;

        Ok(Some(ty))
    }

    /// Return one signature from a callable intrinsic application's written components.
    fn function_signature_from_application(
        &mut self,
        origin: Origin,
        parameters: dir::GlobalTypeId,
        return_type: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let parameters = self.normalize(origin, parameters)?;

        // read the parameter tuple
        let parameters = match self.ty(parameters)? {
            dir::Type::Void => Vec::new(),
            dir::Type::Tuple(tuple) => self
                .tuple_elements(parameters.module_id, tuple.elements)?
                .iter()
                .map(|element| dir::FunctionParameterType {
                    name: None,
                    ty: element.ty,
                    is_optional: element.is_optional,
                    is_rest: element.is_rest,
                })
                .collect(),
            // an open or rigid pack stands as one rest parameter binding the whole tuple
            dir::Type::Variable(_) | dir::Type::Parameter(_) => vec![dir::FunctionParameterType {
                name: None,
                ty: parameters,
                is_optional: false,
                is_rest: true,
            }],
            _ => return Ok(None),
        };

        // intern the signature over the read parameters and written return
        let parameters = self.intern_parameters(&parameters)?;
        let function = dir::FunctionSignatureType {
            parks: false,
            asynchrony: dir::Asynchrony::Sync,
            template: None,
            arguments: dir::TypeListId::EMPTY,
            this_parameter: None,
            parameters,
            return_type: Some(return_type),
            is_generator: false,
            is_construct: false,
        };
        let signature = self.intern_signature(function)?;

        Ok(Some(signature))
    }
}
