use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use {destack_dir as dir, destack_mir as mir};

use destack_ast::{StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Repository, Revision};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout, TypeLayoutPolicy};
use crate::lower::{lower_mutability, static_key_to_field_name};
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
    /// Cached remote nominal layouts by source symbol.
    pub(crate) remote_nominal_layouts_by_symbol:
        HashMap<dir::GlobalSymbolId, mir::LocalNodeId<mir::Type>>,
    /// Remote nominal layouts currently being lowered.
    pub(crate) remote_nominal_layouts_in_progress: HashSet<dir::GlobalSymbolId>,
    /// Policy values for layout decisions.
    pub(crate) layout_policy: TypeLayoutPolicy,
    /// Artifact revision used to read remote symbol names.
    pub(crate) artifact_revision: Option<Revision>,
    /// Profile used to read remote symbol names.
    pub(crate) profile: Option<ProfileId>,
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
            mir::AddressSpace::Local,
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
            remote_nominal_layouts_by_symbol: HashMap::new(),
            remote_nominal_layouts_in_progress: HashSet::new(),
            layout_policy,
            artifact_revision: None,
            profile: None,
        }
    }

    /// Attach artifact context for cross-module symbol reads.
    pub(crate) fn with_artifact_context(mut self, revision: Revision, profile: ProfileId) -> Self {
        self.artifact_revision = Some(revision);
        self.profile = Some(profile);
        self
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
        tree: &mir::Tree,
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
                mir::Type::Tuple { elements, copy: _ } => {
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
        // compute copy from field types
        let mut copy = mir::Copy::Yes;
        for field in &layout.fields {
            let field_type = builder.tree().get(field.ty);
            copy = copy.combine(field_type.copy());
        }

        // build the struct type with computed copy
        self.create_struct_type_with_copyability(layout, copy, builder)
    }

    /// Create a MIR struct type with explicit copy.
    pub(crate) fn create_struct_type_with_copyability(
        &mut self,
        layout: &StructLayout,
        copy: mir::Copy,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        let mut mir_fields = Vec::with_capacity(layout.fields.len());

        // populate field nodes in layout order
        for field in &layout.fields {
            let mir_field = builder.field(Some(field.name), field.ty);
            mir_fields.push(mir_field);
        }

        // return the struct type
        builder.type_struct(mir_fields, copy)
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
            parameters,
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
        let mut lowered_parameters = Vec::with_capacity(parameters.len());
        for parameter in parameters {
            let parameter_type = self.lower_type(types, *parameter, module_id, node, builder)?;
            lowered_parameters.push(parameter_type);
        }

        // lower the return type
        let result = match return_type {
            Some(return_type) => self.lower_type(types, *return_type, module_id, node, builder)?,
            None => self.ty_void,
        };

        // cache the signature type
        let signature = builder.type_function_signature(lowered_parameters, result);
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
                generic_arguments,
            } => self.lower_reference_type(
                types,
                type_id,
                *symbol,
                generic_arguments.as_deref(),
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
            dir::Type::Array { element, .. } => {
                let element = (*element).ok_or_else(|| LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "array without element type".to_string(),
                })?;
                let mir_element = self.lower_type(types, element, module_id, node, builder)?;

                if mir_element == self.ty_void {
                    return Err(LowerError::UnsupportedType {
                        node,
                        ty: element.into_global(module_id),
                        message: "void is not allowed in arrays".to_string(),
                    });
                }

                builder.type_slice(mir_element)
            }
            dir::Type::ArraySized { element, count, .. } => {
                self.lower_array_sized_type(types, *element, *count, module_id, node, builder)?
            }
            dir::Type::Function {
                parameters: _,
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
                    message: format!("unsupported type {dir_type:?}"),
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
        let copy = builder.tree().get(inner_type).copy();

        Ok(builder.tree_mut().insert_type(mir::Type::Newtype {
            inner: inner_type.into(),
            copy,
        }))
    }

    /// Lower a nominal reference type to its MIR representation.
    fn lower_reference_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        generic_arguments: Option<&[dir::StaticArgument]>,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(mir_type) = self.lower_ownership_alias_type(
            types,
            type_id,
            symbol,
            generic_arguments,
            module_id,
            node,
            builder,
        )? {
            return Ok(mir_type);
        }

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
        if let Some(mir_type) = self.lower_remote_nominal_type(symbol, module_id, node, builder)? {
            return Ok(mir_type);
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
                generic_arguments,
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
            let copy = builder.tree().get(inner_type).copy();
            return Ok(builder.tree_mut().insert_type(mir::Type::Newtype {
                inner: inner_type.into(),
                copy,
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

    /// Lower a remote nominal struct layout when its analyzed artifact is available.
    fn lower_remote_nominal_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        current_module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        if symbol.module_id == current_module_id || symbol.ty() != dir::SymbolType::Struct {
            return Ok(None);
        }
        if let Some(mir_type) = self.remote_nominal_layouts_by_symbol.get(&symbol).copied() {
            return Ok(Some(mir_type));
        }
        if !self.remote_nominal_layouts_in_progress.insert(symbol) {
            return Err(LowerError::UnsupportedConstruct {
                node,
                message: "cycle detected while lowering remote nominal layout".to_string(),
            });
        }

        let Some(revision) = self.artifact_revision else {
            self.remote_nominal_layouts_in_progress.remove(&symbol);
            return Ok(None);
        };
        let Some(profile) = self.profile else {
            self.remote_nominal_layouts_in_progress.remove(&symbol);
            return Ok(None);
        };
        let Some(dir) = self
            .repository
            .dir_analyzed(revision, symbol.module_id, profile)
        else {
            self.remote_nominal_layouts_in_progress.remove(&symbol);
            return Ok(None);
        };
        let Some(members) = self.struct_members_for_symbol(symbol, &dir.symbols, &dir.tree) else {
            self.remote_nominal_layouts_in_progress.remove(&symbol);
            return Ok(None);
        };

        let mut field_lowerer = TypeLowerer::new(
            builder,
            self.pointer_bytes(),
            Arc::clone(&self.repository),
            self.vector_symbol,
        )
        .with_artifact_context(revision, profile);
        let mut fields = Vec::new();

        for (source_index, member_id) in members.iter().enumerate() {
            let dir::Member::Field {
                key, declared_type, ..
            } = dir.tree.get(*member_id)
            else {
                continue;
            };
            let Some(key) = Self::static_key_from_member_key(*key) else {
                continue;
            };
            let Some(declared_type) = declared_type else {
                continue;
            };
            let type_id = dir
                .types
                .get_declared_or_inferred_type_id(declared_type.into_global_any(symbol.module_id))
                .ok_or_else(|| LowerError::MissingType {
                    node: declared_type
                        .into_global_any(symbol.module_id)
                        .into_anchored(Some(profile)),
                })?;
            let field_type =
                field_lowerer.lower_type(&dir.types, type_id, symbol.module_id, node, builder)?;
            let mir_type = builder.tree().get(field_type);
            let (size, alignment) = field_lowerer
                .size_and_align_of_type(mir_type, builder.tree())
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    node,
                    message: "remote nominal layout requires concrete nested types".to_string(),
                })?;

            fields.push(FieldInput {
                name: static_key_to_field_name(&key, builder),
                ty: field_type,
                size,
                alignment,
                source_index: Some(source_index as u32),
                kind: FieldLayoutKind::Source,
            });
        }

        let layout = self.compute_struct_layout(fields, LayoutPolicy::Source);
        let mir_type = self.create_struct_type(&layout, builder);
        self.set_layout(mir_type, layout);
        self.remote_nominal_layouts_by_symbol
            .insert(symbol, mir_type);
        self.remote_nominal_layouts_in_progress.remove(&symbol);

        Ok(Some(mir_type))
    }

    /// Return struct members for a symbol declaration.
    fn struct_members_for_symbol(
        &self,
        symbol: dir::GlobalSymbolId,
        symbols: &dir::SymbolTable,
        tree: &dir::Tree,
    ) -> Option<Vec<dir::LocalNodeId<dir::Member>>> {
        let declaration = symbols.get_symbol(symbol.local_id).primary_declaration?;
        let declaration_id = declaration
            .local_id
            .try_into_typed::<dir::Declaration>()
            .ok()?;

        let dir::Declaration::Struct(declaration) = tree.get(declaration_id) else {
            return None;
        };

        Some(declaration.members.clone())
    }

    /// Convert a simple member key to a static field key.
    fn static_key_from_member_key(key: dir::Key) -> Option<dir::StaticKey> {
        match key {
            dir::Key::Name(dir::Name::Identifier(name) | dir::Name::String(name)) => {
                Some(dir::StaticKey::Name(name))
            }
            dir::Key::Name(dir::Name::Number(name)) => Some(dir::StaticKey::Number(name)),
            dir::Key::Private(name) => Some(dir::StaticKey::Name(name)),
            dir::Key::Expression(_) => None,
        }
    }

    /// Lower an intrinsic ownership alias into a MIR reference type.
    fn lower_ownership_alias_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        static_arguments: Option<&[dir::StaticArgument]>,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        if symbol.ty() != dir::SymbolType::TypeAlias && symbol.ty() != dir::SymbolType::Newtype {
            return Ok(None);
        }

        let Some(name) = self.symbol_name(symbol) else {
            return Ok(None);
        };
        let name = self.repository.strings.get(name).to_string();

        let kind = match name.as_str() {
            "Managed" | "AsManaged" => Some(mir::ReferenceKind::Managed),
            "Owned" | "AsOwned" => Some(mir::ReferenceKind::Owned),
            "Borrowed" | "AsBorrowed" => Some(mir::ReferenceKind::Borrowed),
            "Raw" | "AsRaw" => Some(mir::ReferenceKind::Raw),
            "Form" => self.ownership_form_kind(static_arguments),
            "Shared" => {
                let inner_type_id =
                    self.first_type_static_argument(static_arguments, type_id, module_id, node)?;
                let inner_type = self.lower_type(types, inner_type_id, module_id, node, builder)?;
                let shared_type = self.rewrite_reference_address_space(
                    inner_type,
                    mir::AddressSpace::Shared,
                    type_id,
                    module_id,
                    node,
                    builder,
                )?;

                return Ok(Some(shared_type));
            }
            _ => None,
        };
        let Some(kind) = kind else {
            return Ok(None);
        };

        let base_type_id =
            self.first_type_static_argument(static_arguments, type_id, module_id, node)?;
        let base_type = self.lower_type(types, base_type_id, module_id, node, builder)?;
        let address_space = self.ownership_form_address_space(static_arguments);
        let mutability = self.default_reference_mutability(kind);

        Ok(Some(builder.type_reference(
            kind,
            base_type,
            mutability,
            address_space,
            false,
        )))
    }

    /// Return the lowered mutability for one ownership kind.
    fn default_reference_mutability(&self, kind: mir::ReferenceKind) -> mir::Mutability {
        match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Raw => mir::Mutability::Immutable,
            mir::ReferenceKind::Owned | mir::ReferenceKind::Borrowed => mir::Mutability::Mutable,
        }
    }

    /// Return the first type static argument for an ownership alias.
    fn first_type_static_argument(
        &self,
        static_arguments: Option<&[dir::StaticArgument]>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<dir::LocalTypeId> {
        let Some(arguments) = static_arguments else {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "ownership form missing base type".to_string(),
            });
        };
        let Some(argument) = arguments.first() else {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "ownership form missing base type".to_string(),
            });
        };
        let dir::StaticArgument::Evaluated {
            value: dir::StaticExpression::Type { ty },
            ..
        } = argument
        else {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "ownership form base must be a type".to_string(),
            });
        };

        Ok(*ty)
    }

    /// Return the reference kind encoded by an ownership form.
    fn ownership_form_kind(
        &self,
        static_arguments: Option<&[dir::StaticArgument]>,
    ) -> Option<mir::ReferenceKind> {
        let ownership = self.static_string_argument(static_arguments?, 1)?;
        let ownership = self.repository.strings.get(ownership);

        match ownership.as_ref() {
            "managed" => Some(mir::ReferenceKind::Managed),
            "owned" => Some(mir::ReferenceKind::Owned),
            "borrowed" => Some(mir::ReferenceKind::Borrowed),
            "raw" => Some(mir::ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Return the address space encoded by an ownership form.
    fn ownership_form_address_space(
        &self,
        static_arguments: Option<&[dir::StaticArgument]>,
    ) -> mir::AddressSpace {
        let Some(space) =
            static_arguments.and_then(|arguments| self.static_string_argument(arguments, 2))
        else {
            return mir::AddressSpace::Local;
        };
        let space = self.repository.strings.get(space);

        mir::AddressSpace::from_name(space.as_ref())
    }

    /// Return one string static argument.
    fn static_string_argument(
        &self,
        arguments: &[dir::StaticArgument],
        index: usize,
    ) -> Option<StringId> {
        let dir::StaticArgument::Evaluated {
            value:
                dir::StaticExpression::ScalarLiteral {
                    value: dir::ScalarLiteral::String(value),
                },
            ..
        } = arguments.get(index)?
        else {
            return None;
        };

        Some(*value)
    }

    /// Rewrite a lowered reference into another address space.
    fn rewrite_reference_address_space(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        address_space: mir::AddressSpace,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let mir_type = builder.tree().get(ty).clone();
        let mir::Type::Reference {
            kind,
            mutability,
            pointee,
            is_nullable,
            ..
        } = mir_type
        else {
            return Ok(builder.type_reference(
                mir::ReferenceKind::Managed,
                ty,
                mir::Mutability::Immutable,
                address_space,
                false,
            ));
        };
        let Some(pointee) = pointee.ty() else {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "reference address-space rewrite requires a concrete pointee".to_string(),
            });
        };

        Ok(builder.type_reference(kind, pointee, mutability, address_space, is_nullable))
    }

    /// Return the source name for one symbol when artifacts are available.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<StringId> {
        let revision = self.artifact_revision?;
        let profile = self.profile?;
        let dir = self
            .repository
            .dir_declared(revision, symbol.module_id, profile)?;
        dir.symbols.get_symbol(symbol.local_id).name()
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

        let mir_type = builder.type_callable(signature);
        let callable_environment_type = builder.tree().callable_environment_type();
        fields[1].ty = callable_environment_type;

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
