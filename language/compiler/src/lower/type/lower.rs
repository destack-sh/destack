use std::collections::HashMap;
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use destack_ast::{StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::Repository;

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout, TypeLayoutPolicy};
use crate::lower::lower_mutability;
use crate::{InterfaceRefLayout, LowerError, LowerResult, UnionLayout};

// synthetic field names for function value layouts
const FUNCTION_PTR_FIELD: &str = "@function_ptr";
const ENV_FIELD: &str = "@env";

/// Cached entry for lowered types.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TypeCacheEntry {
    /// Lowering is in progress for this type.
    InProgress,
    /// Lowered mir type is ready.
    Ready(mir::LocalNodeId<mir::Type>),
}

/// Lowers DIR types into MIR types with a shared cache.
#[derive(Debug)]
pub(crate) struct TypeLowerer {
    /// Access to repository metadata for qualified names.
    pub(super) repository: Arc<Repository>,
    /// Cached Vector type symbol for SIMD lowering.
    pub(crate) vector_symbol: Option<dir::GlobalSymbolId>,
    /// Cached MIR types by DIR type id.
    pub(crate) type_cache: HashMap<dir::LocalTypeId, TypeCacheEntry>,
    /// Cached struct layouts by MIR type id (for field index lookup).
    pub(super) layout_cache: HashMap<mir::LocalNodeId<mir::Type>, StructLayout>,
    /// Pointer width in bits for pointer-sized integers.
    pub(super) pointer_width_bits: u16,
    /// Cached MIR void type.
    pub(crate) ty_void: mir::LocalNodeId<mir::Type>,
    /// Cached MIR bool type.
    pub(crate) ty_bool: mir::LocalNodeId<mir::Type>,
    /// Cached MIR i32 type.
    pub(crate) ty_i32: mir::LocalNodeId<mir::Type>,
    /// Cached MIR i64 type.
    pub(crate) ty_i64: mir::LocalNodeId<mir::Type>,
    /// Cached MIR isize type.
    pub(crate) ty_isize: mir::LocalNodeId<mir::Type>,
    /// Cached MIR u32 type.
    pub(crate) ty_u32: mir::LocalNodeId<mir::Type>,
    /// Cached MIR usize type.
    pub(crate) ty_usize: mir::LocalNodeId<mir::Type>,
    /// Cached MIR f32 type.
    pub(crate) ty_f32: mir::LocalNodeId<mir::Type>,
    /// Cached MIR f64 type.
    pub(crate) ty_f64: mir::LocalNodeId<mir::Type>,
    /// Cached MIR string reference type.
    pub(crate) ty_string: Option<mir::LocalNodeId<mir::Type>>,
    /// Cached MIR function environment pointer type.
    pub(crate) function_environment_pointer_type: mir::LocalNodeId<mir::Type>,
    /// Cached union layout metadata by DIR type id.
    pub(crate) union_cache: HashMap<dir::LocalTypeId, UnionLayout>,
    /// Cached interface reference layouts by DIR type id.
    pub(crate) interface_ref_cache: HashMap<dir::LocalTypeId, InterfaceRefLayout>,
    /// Cached function pointer signature types by DIR function type id.
    pub(crate) function_signature_types: HashMap<dir::LocalTypeId, mir::LocalNodeId<mir::Type>>,
    /// Policy values for layout decisions.
    pub(crate) layout_policy: TypeLayoutPolicy,
}

#[allow(clippy::too_many_arguments)]
impl TypeLowerer {
    /// Create a new type lowerer with cached common types.
    pub(crate) fn new(
        builder: &mut mir::ModuleBuilder,
        pointer_bytes: u8,
        repository: Arc<Repository>,
        vector_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        let pointer_width_bits = u16::from(pointer_bytes) * 8;
        let layout_policy = TypeLayoutPolicy::for_target(pointer_bytes);
        let ty_void = builder.type_void();
        let function_environment_pointer_type = builder.type_reference(
            mir::ReferenceKind::Managed,
            ty_void,
            mir::Mutability::Mutable,
            mir::AddressSpace::Generic,
            true,
        );

        Self {
            repository,
            vector_symbol,
            type_cache: HashMap::new(),
            layout_cache: HashMap::new(),
            pointer_width_bits,
            ty_void,
            ty_bool: builder.type_boolean(),
            ty_i32: builder.type_i32(),
            ty_i64: builder.type_i64(),
            ty_isize: builder.type_isize(),
            ty_u32: builder.type_u32(),
            ty_usize: builder.type_usize(),
            ty_f32: builder.type_f32(),
            ty_f64: builder.type_f64(),
            ty_string: None,
            function_environment_pointer_type,
            union_cache: HashMap::new(),
            interface_ref_cache: HashMap::new(),
            function_signature_types: HashMap::new(),
            layout_policy,
        }
    }

    /// Return a cached mir type when available.
    pub(crate) fn cached_type(
        &self,
        type_id: dir::LocalTypeId,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match self.type_cache.get(&type_id) {
            Some(TypeCacheEntry::Ready(mir_type)) => Some(*mir_type),
            _ => None,
        }
    }

    /// Get the cached layout for a struct type.
    pub(crate) fn layout_for_type(&self, ty: mir::LocalNodeId<mir::Type>) -> Option<&StructLayout> {
        self.layout_cache.get(&ty)
    }

    /// Get the cached layout for a struct type or emit a missing layout error.
    pub(crate) fn layout_for_type_or_error(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<&StructLayout> {
        self.layout_cache
            .get(&ty)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node,
                message: "missing struct layout".to_string(),
            })
    }

    /// Return the pointer width in bits for this lowering session.
    pub(crate) fn pointer_width_bits(&self) -> u16 {
        self.pointer_width_bits
    }

    /// Return the canonical function environment pointer type.
    pub(crate) fn function_environment_pointer_type(&self) -> mir::LocalNodeId<mir::Type> {
        self.function_environment_pointer_type
    }

    /// Resolve a field name to its index for a given aggregate type.
    ///
    /// For structs, use the cached layout.
    /// For tuples, parse the numeric field name.
    pub(crate) fn field_index_for_type(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        field_name: StringId,
        strings: &StringPool,
        tree: &mir::NodeTree,
    ) -> Option<usize> {
        let mut target_ty = ty;
        loop {
            let mir_type = tree.get(target_ty);
            match mir_type {
                mir::Type::Reference { pointee, .. } => {
                    target_ty = pointee.ty()?;
                }
                mir::Type::Struct { .. } => {
                    return self
                        .layout_for_type(target_ty)
                        .and_then(|layout| layout.field_index(field_name))
                        .map(|i| i as usize);
                }
                mir::Type::Tuple {
                    elements,
                    copyability: _,
                } => {
                    let name_str = strings.get(field_name);
                    return name_str
                        .parse::<usize>()
                        .ok()
                        .filter(|&i| i < elements.len());
                }
                _ => return None,
            }
        }
    }

    /// Get the pointer size in bytes for this target.
    pub(crate) fn pointer_bytes(&self) -> u8 {
        (self.pointer_width_bits / 8) as u8
    }

    /// Get the cached string type, if initialized.
    pub(crate) fn string_type(&self) -> Option<mir::LocalNodeId<mir::Type>> {
        self.ty_string
    }

    /// Cache the canonical string type.
    pub(crate) fn set_string_type(&mut self, ty: mir::LocalNodeId<mir::Type>) {
        self.ty_string = Some(ty);
    }

    /// Cache a struct layout for a MIR type.
    pub(crate) fn set_layout(&mut self, ty: mir::LocalNodeId<mir::Type>, layout: StructLayout) {
        self.layout_cache.insert(ty, layout);
    }

    /// Create a MIR struct type from a computed layout.
    ///
    /// This creates the MIR `dir::Type::Struct` with fields that have their offsets
    /// already computed by `compute_struct_layout`.
    pub(crate) fn create_struct_type(
        &mut self,
        layout: &StructLayout,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        // compute copyability from field types
        let mut copyability = mir::Copyability::Trivial;
        for field in &layout.fields {
            let field_type = builder.tree().get(field.ty);
            copyability = copyability.combine(field_type.copyability());
        }

        // build the struct type with computed copyability
        self.create_struct_type_with_copyability(layout, copyability, builder)
    }

    /// Create a MIR struct type with explicit copyability.
    pub(crate) fn create_struct_type_with_copyability(
        &mut self,
        layout: &StructLayout,
        copyability: mir::Copyability,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        let mut mir_fields = Vec::with_capacity(layout.fields.len());

        // populate field nodes in layout order
        for field in &layout.fields {
            let mir_field = builder.field(Some(field.name), field.ty);
            mir_fields.push(mir_field);
        }

        // return the struct type
        builder.type_struct(mir_fields, copyability)
    }

    /// Build the function pointer signature type for a function type.
    fn lower_function_signature_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(signature) = self.function_signature_types.get(&type_id) {
            return Ok(*signature);
        }

        let dir::Type::Function {
            dynamic_parameters,
            return_type,
            ..
        } = types.get_type(type_id)
        else {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "function signature expects a function type".to_string(),
            });
        };

        // lower the declared parameters
        let mut parameters = Vec::with_capacity(dynamic_parameters.len());
        for parameter in dynamic_parameters {
            let parameter_type = self.lower_type(types, *parameter, module_id, node, builder)?;
            parameters.push(parameter_type);
        }

        // lower the return type
        let result = match return_type {
            Some(return_type) => self.lower_type(types, *return_type, module_id, node, builder)?,
            None => self.ty_void,
        };

        // cache the signature type
        let signature = builder.type_function_pointer(parameters, result);
        self.function_signature_types.insert(type_id, signature);

        Ok(signature)
    }

    /// Lower a DIR type to a MIR type.
    pub(crate) fn lower_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(entry) = self.type_cache.get(&type_id) {
            return match entry {
                TypeCacheEntry::Ready(mir_type) => Ok(*mir_type),
                TypeCacheEntry::InProgress => Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "cycle detected while lowering type".to_string(),
                }),
            };
        }

        self.type_cache.insert(type_id, TypeCacheEntry::InProgress);

        let dir_type = types.get_type(type_id);

        // lower enum instance types as nominal wrappers over their backing representation
        if let Some(enum_symbol) = types.symbol_for_instance_type(type_id)
            && enum_symbol.ty() == dir::SymbolType::Enum
        {
            let mir_type = self.lower_nominal_enum_type(types, enum_symbol, node, builder)?;
            self.type_cache
                .insert(type_id, TypeCacheEntry::Ready(mir_type));
            return Ok(mir_type);
        }

        let mir_type = match dir_type {
            dir::Type::Reference {
                symbol,
                static_arguments,
            } => self.lower_reference_type(
                types,
                type_id,
                *symbol,
                static_arguments.as_deref(),
                module_id,
                node,
                builder,
            )?,
            dir::Type::ValueOf {
                mutability, right, ..
            } => {
                // lower the owning handle pointee type
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Mutable);
                builder.type_owned_reference(pointee, mutability)
            }
            dir::Type::ReferenceOf {
                mutability, right, ..
            } => {
                // lower the borrowed reference pointee type
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Mutable);
                builder.type_borrowed_reference(pointee, mutability)
            }
            dir::Type::PointerOf { right, .. } => {
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                builder.type_raw_pointer(pointee)
            }
            dir::Type::Object {
                fields,
                call_signatures: _,
                construct_signatures: _,
                index_signatures,
            } => {
                if !index_signatures.is_empty() {
                    return Err(LowerError::UnsupportedType {
                        node,
                        ty: type_id.into_global(module_id),
                        message: "index signatures are not supported for native lowering"
                            .to_string(),
                    });
                }

                self.lower_object_type(types, fields, module_id, node, builder)?
            }
            dir::Type::Tuple { elements, .. } => {
                self.lower_tuple_type(types, elements, module_id, node, builder)?
            }
            dir::Type::ArraySized { element, count, .. } => {
                self.lower_array_sized_type(types, *element, *count, module_id, node, builder)?
            }
            dir::Type::Function {
                dynamic_parameters: _,
                return_type: _,
                ..
            } => self.lower_function_type(types, type_id, module_id, node, builder)?,
            dir::Type::Union { elements } => {
                self.lower_union_type(types, type_id, elements, module_id, node, builder)?
            }
            dir::Type::Intersection { elements } => {
                self.lower_intersection_type(types, elements, module_id, node, builder)?
            }
            _ => self.try_lower_type(dir_type, builder).ok_or_else(|| {
                LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "unsupported type".to_string(),
                }
            })?,
        };
        self.type_cache
            .insert(type_id, TypeCacheEntry::Ready(mir_type));
        Ok(mir_type)
    }

    /// Lower an enum symbol to its backing MIR type.
    fn lower_enum_backing_type(
        &mut self,
        types: &dir::TypeTable,
        enum_symbol: dir::GlobalSymbolId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // load the enum backing type
        let backing_type = types.get_enum_backing_type(enum_symbol).ok_or_else(|| {
            LowerError::UnsupportedConstruct {
                node,
                message: "enum missing backing type".to_string(),
            }
        })?;

        // map backing types to mir
        match backing_type {
            dir::EnumBackingType::Int(int_type) => {
                Ok(self.mir_type_for_int_type(int_type, builder))
            }
            dir::EnumBackingType::String => {
                self.ty_string
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        node,
                        message: "missing well known String layout (load library/native)"
                            .to_string(),
                    })
            }
        }
    }

    /// Lower an enum symbol to its nominal MIR wrapper type.
    fn lower_nominal_enum_type(
        &mut self,
        types: &dir::TypeTable,
        enum_symbol: dir::GlobalSymbolId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let inner_type = self.lower_enum_backing_type(types, enum_symbol, node, builder)?;
        let copyability = builder.tree().get(inner_type).copyability();

        Ok(builder.tree_mut().insert_type(mir::Type::Newtype {
            inner: inner_type.into(),
            copyability,
        }))
    }

    /// Lower a nominal reference type to its MIR representation.
    fn lower_reference_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        static_arguments: Option<&[dir::StaticArgument]>,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if symbol.ty() == dir::SymbolType::Interface {
            return self.lower_interface_reference_type(types, type_id, module_id, node, builder);
        }
        if symbol.ty() == dir::SymbolType::Enum {
            if let Some(instance_type_id) = types.get_instance_type_id(symbol)
                && instance_type_id != type_id
            {
                return self.lower_type(types, instance_type_id, module_id, node, builder);
            }

            return self.lower_nominal_enum_type(types, symbol, node, builder);
        }

        // handle vector type lowering
        let is_alias_with_target = symbol.ty() == dir::SymbolType::TypeAlias
            && types.get_alias_target_type_id(symbol).is_some();

        if !is_alias_with_target && self.is_vector_symbol(symbol) {
            return self.lower_vector_reference_type(
                types,
                type_id,
                module_id,
                node,
                static_arguments,
                builder,
            );
        }

        // handle nominal newtypes with a transparent MIR wrapper
        if symbol.ty() == dir::SymbolType::Newtype {
            if let Some(instance_type_id) = types.get_instance_type_id(symbol)
                && instance_type_id != type_id
            {
                return self.lower_type(types, instance_type_id, module_id, node, builder);
            }

            let Some(alias_target_id) = types.get_alias_target_type_id(symbol) else {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "newtype missing target type".to_string(),
                });
            };
            let inner_type = self.lower_type(types, alias_target_id, module_id, node, builder)?;
            let copyability = builder.tree().get(inner_type).copyability();
            return Ok(builder.tree_mut().insert_type(mir::Type::Newtype {
                inner: inner_type.into(),
                copyability,
            }));
        }

        let instance_type_id =
            types
                .get_instance_type_id(symbol)
                .ok_or_else(|| LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "type reference has no instance type".to_string(),
                })?;

        let instance_type = if instance_type_id == type_id {
            if !matches!(
                symbol.ty(),
                dir::SymbolType::TypeAlias | dir::SymbolType::Newtype
            ) {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "non-alias type is self-referential".to_string(),
                });
            }
            let Some(alias_target_id) = types.get_alias_target_type_id(symbol) else {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "type alias has no target type".to_string(),
                });
            };
            self.lower_type(types, alias_target_id, module_id, node, builder)?
        } else {
            self.lower_type(types, instance_type_id, module_id, node, builder)?
        };

        if symbol.ty() == dir::SymbolType::Class {
            Ok(builder.type_managed_reference(instance_type))
        } else {
            Ok(instance_type)
        }
    }

    /// Lower a function type into its closure-pair representation.
    fn lower_function_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let signature =
            self.lower_function_signature_type(types, type_id, module_id, node, builder)?;

        let env_pointer_type = self.function_environment_pointer_type();
        let signature_type = builder.tree().get(signature);
        let env_type = builder.tree().get(env_pointer_type);
        let (signature_size, signature_align) = self
            .size_and_align_of_type(signature_type, builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "function layout requires concrete nested types".to_string(),
            })?;
        let (env_size, env_align) = self
            .size_and_align_of_type(env_type, builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "function layout requires concrete nested types".to_string(),
            })?;

        let fn_name = builder.intern(FUNCTION_PTR_FIELD);
        let env_name = builder.intern(ENV_FIELD);
        let mut fields = vec![
            FieldInput {
                name: fn_name,
                ty: signature,
                size: signature_size,
                alignment: signature_align,
                source_index: Some(0),
                kind: FieldLayoutKind::Synthetic,
            },
            FieldInput {
                name: env_name,
                ty: env_pointer_type,
                size: env_size,
                alignment: env_align,
                source_index: Some(1),
                kind: FieldLayoutKind::Synthetic,
            },
        ];

        let mir_type = builder.type_function_value(signature);
        let function_value_environment_type = builder.tree().function_value_environment_type();
        fields[1].ty = function_value_environment_type;

        let layout = self.compute_struct_layout(fields, LayoutPolicy::Optimized);
        self.set_layout(mir_type, layout);
        Ok(mir_type)
    }

    /// Lower an intersection type by selecting its primary element.
    fn lower_intersection_type(
        &mut self,
        types: &dir::TypeTable,
        elements: &[dir::LocalTypeId],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let primary = self.select_intersection_primary_type(types, elements, module_id, node)?;
        self.lower_type(types, primary, module_id, node, builder)
    }

    /// Lower a DIR int type into a MIR type.
    fn mir_type_for_int_type(
        &mut self,
        int_type: dir::IntType,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        match int_type.simplify() {
            dir::IntType::Int8 => builder.type_int(8, true),
            dir::IntType::Int16 => builder.type_int(16, true),
            dir::IntType::Int32 => self.ty_i32,
            dir::IntType::Int64 => self.ty_i64,
            dir::IntType::Int128 => builder.type_int(128, true),
            dir::IntType::Int256 => builder.type_int(256, true),
            dir::IntType::Isize => self.ty_isize,
            dir::IntType::Uint8 => builder.type_int(8, false),
            dir::IntType::Uint16 => builder.type_int(16, false),
            dir::IntType::Uint32 => self.ty_u32,
            dir::IntType::Uint64 => builder.type_int(64, false),
            dir::IntType::Uint128 => builder.type_int(128, false),
            dir::IntType::Uint256 => builder.type_int(256, false),
            dir::IntType::Usize => self.ty_usize,
            dir::IntType::Arbitrary { width, is_signed } => builder.type_int(width, is_signed),
        }
    }
}
