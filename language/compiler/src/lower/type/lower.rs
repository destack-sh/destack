use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::DiagnosticAnchor;
use destack_ast::{StringId, StringPool};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout, TypeLayoutPolicy};
use crate::lower::static_key_to_field_name;
use crate::{Compiler, InterfaceRefLayout, LowerError, LowerResult, UnionLayout};

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
pub(crate) struct TypeLowerer<'a> {
    /// Compiler facade for artifact backed reads.
    pub(super) compiler: &'a Compiler,
    /// Provider context for artifact dependencies.
    pub(super) context: &'a dyn ProviderContext,
    /// Declared DIR strings for this module.
    pub(super) strings: &'a StringPool,
    /// Profile used for cross-module artifact reads.
    pub(super) profile: ProfileId,
    /// DIR tree being lowered.
    pub(super) dir_tree: &'a dir::Tree,
    /// Symbol table for local declaration form reads.
    pub(super) symbols: &'a dir::BindingTable,
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
}

#[allow(clippy::too_many_arguments)]
impl<'a> TypeLowerer<'a> {
    /// Create a new type lowerer with cached common types.
    pub(crate) fn new(
        builder: &mut mir::ModuleBuilder,
        pointer_bytes: u8,
        compiler: &'a Compiler,
        context: &'a dyn ProviderContext,
        strings: &'a StringPool,
        profile: ProfileId,
        dir_tree: &'a dir::Tree,
        symbols: &'a dir::BindingTable,
        vector_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        let pointer_width_bits = u16::from(pointer_bytes) * 8;
        let layout_policy = TypeLayoutPolicy::for_target(pointer_bytes);
        let ty_void = builder.type_void();
        Self {
            compiler,
            context,
            strings,
            profile,
            dir_tree,
            symbols,
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
            union_cache: HashMap::new(),
            interface_ref_cache: HashMap::new(),
            function_signature_types: HashMap::new(),
            remote_nominal_layouts_by_symbol: HashMap::new(),
            remote_nominal_layouts_in_progress: HashSet::new(),
            layout_policy,
        }
    }

    /// Read one symbol record from local or declared DIR.
    pub(crate) fn symbol(&self, symbol: dir::GlobalSymbolId) -> Option<dir::Symbol> {
        if symbol.module_id == self.symbols.module_id {
            Some(self.symbols.get_symbol(symbol.local_id).clone())
        } else {
            let declared = self
                .compiler
                .dir_declared(self.context, symbol.module_id, self.profile)
                .ok()?;

            Some(declared.bindings.get_symbol(symbol.local_id).clone())
        }
    }

    /// Return the declaration form for one symbol.
    pub(crate) fn symbol_form(&self, symbol: dir::GlobalSymbolId) -> Option<dir::SymbolForm> {
        Some(self.symbol(symbol)?.form)
    }

    /// Return whether one symbol has the given declaration form.
    pub(crate) fn symbol_is(&self, symbol: dir::GlobalSymbolId, form: dir::SymbolForm) -> bool {
        self.symbol_form(symbol)
            .is_some_and(|actual| actual == form)
    }

    /// Return the diagnostic anchor for one DIR node.
    pub(crate) fn diagnostic_anchor(&self, node: dir::AnchoredGlobalNodeId) -> DiagnosticAnchor {
        assert_eq!(
            self.dir_tree.module_id,
            node.module_id(),
            "type diagnostic node belongs to a different module"
        );

        let span = self
            .dir_tree
            .get_span_by_id(node.local_id().id)
            .expect("type diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
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
                anchor: self.diagnostic_anchor(node),
                message: "missing struct layout".to_string(),
            })
    }

    /// Return the pointer width in bits for this lowering session.
    pub(crate) fn pointer_width_bits(&self) -> u16 {
        self.pointer_width_bits
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

        let dir::Type::Function(function) = types.get_type(type_id) else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "function signature expects a function type".to_string(),
            }
            .into());
        };

        // lower the declared parameters
        let mut lowered_parameters = Vec::with_capacity(function.parameters.len());
        for parameter in &function.parameters {
            let parameter_type = self.lower_type(types, *parameter, module_id, node, builder)?;
            lowered_parameters.push(parameter_type);
        }

        // lower the return type
        let result = match function.return_type {
            Some(return_type) => self.lower_type(types, return_type, module_id, node, builder)?,
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
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "cycle detected while lowering type".to_string(),
                }
                .into()),
            };
        }

        self.type_cache.insert(type_id, TypeCacheEntry::InProgress);

        let dir_type = types.get_type(type_id);

        // lower enum instance types as nominal wrappers over their backing representation
        if let Some(enum_symbol) = types.symbol_for_instance_type(type_id)
            && self.symbol_is(enum_symbol, dir::SymbolForm::Enum)
        {
            let mir_type = self.lower_nominal_enum_type(types, enum_symbol, node, builder)?;
            self.type_cache
                .insert(type_id, TypeCacheEntry::Ready(mir_type));
            return Ok(mir_type);
        }

        let mir_type = match dir_type {
            dir::Type::Reference(reference) => self.lower_reference_type(
                types,
                type_id,
                reference.symbol,
                reference.generic_arguments.as_deref(),
                module_id,
                node,
                builder,
            )?,
            dir::Type::Form(form) => self.lower_type(types, form.base, module_id, node, builder)?,
            dir::Type::Object(object) => {
                if !object.index_signatures.is_empty() {
                    return Err(LowerError::UnsupportedType {
                        anchor: self.diagnostic_anchor(node),
                        ty: type_id.into_global(module_id),
                        message: "index signatures are not supported for native lowering"
                            .to_string(),
                    }
                    .into());
                }

                self.lower_object_type(types, &object.fields, module_id, node, builder)?
            }
            dir::Type::Tuple(tuple) => {
                self.lower_tuple_type(types, &tuple.elements, module_id, node, builder)?
            }
            dir::Type::Slice(slice) => {
                let element = slice.element.ok_or_else(|| LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "slice without element type".to_string(),
                })?;
                let mir_element = self.lower_type(types, element, module_id, node, builder)?;

                if mir_element == self.ty_void {
                    return Err(LowerError::UnsupportedType {
                        anchor: self.diagnostic_anchor(node),
                        ty: element.into_global(module_id),
                        message: "void is not allowed in arrays".to_string(),
                    }
                    .into());
                }

                builder.type_slice(mir_element)
            }
            dir::Type::FixedArray(array) => self.lower_array_sized_type(
                types,
                array.element,
                array.count,
                module_id,
                node,
                builder,
            )?,
            dir::Type::Function(_) => {
                self.lower_function_type(types, type_id, module_id, node, builder)?
            }
            dir::Type::Union(union) => {
                self.lower_union_type(types, type_id, &union.elements, module_id, node, builder)?
            }
            dir::Type::Intersection(intersection) => self.lower_intersection_type(
                types,
                &intersection.elements,
                module_id,
                node,
                builder,
            )?,
            _ => self.try_lower_type(dir_type, builder).ok_or_else(|| {
                LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
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
                anchor: self.diagnostic_anchor(node),
                message: "enum missing backing type".to_string(),
            }
        })?;

        // map backing types to mir
        match backing_type {
            dir::EnumBackingType::Integer(int_type) => {
                Ok(self.mir_type_for_int_type(int_type, builder))
            }
            dir::EnumBackingType::String => {
                self.ty_string
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(node),
                        message: "missing well known String layout (load core)".to_string(),
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

        if self.symbol_is(symbol, dir::SymbolForm::Interface) {
            return self.lower_interface_reference_type(types, type_id, module_id, node, builder);
        }
        if self.symbol_is(symbol, dir::SymbolForm::Enum) {
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
        let is_alias_with_target = self.symbol_is(symbol, dir::SymbolForm::TypeAlias)
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
        if self.symbol_is(symbol, dir::SymbolForm::Newtype) {
            if let Some(instance_type_id) = types.get_instance_type_id(symbol)
                && instance_type_id != type_id
            {
                return self.lower_type(types, instance_type_id, module_id, node, builder);
            }

            let Some(alias_target_id) = types.get_alias_target_type_id(symbol) else {
                return Err(LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "newtype missing target type".to_string(),
                }
                .into());
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
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "type reference has no instance type".to_string(),
                })?;

        let instance_type = if instance_type_id == type_id {
            if !matches!(
                self.symbol_form(symbol),
                Some(dir::SymbolForm::TypeAlias | dir::SymbolForm::Newtype)
            ) {
                return Err(LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "non-alias type is self-referential".to_string(),
                }
                .into());
            }
            let Some(alias_target_id) = types.get_alias_target_type_id(symbol) else {
                return Err(LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "type alias has no target type".to_string(),
                }
                .into());
            };
            self.lower_type(types, alias_target_id, module_id, node, builder)?
        } else {
            self.lower_type(types, instance_type_id, module_id, node, builder)?
        };

        if self.symbol_is(symbol, dir::SymbolForm::Class) {
            Ok(builder.type_managed_reference(instance_type))
        } else {
            Ok(instance_type)
        }
    }

    /// Lower a remote nominal struct layout when its checked artifact is available.
    fn lower_remote_nominal_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        current_module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        if symbol.module_id == current_module_id || !self.symbol_is(symbol, dir::SymbolForm::Struct)
        {
            return Ok(None);
        }
        if let Some(mir_type) = self.remote_nominal_layouts_by_symbol.get(&symbol).copied() {
            return Ok(Some(mir_type));
        }
        if !self.remote_nominal_layouts_in_progress.insert(symbol) {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "cycle detected while lowering remote nominal layout".to_string(),
            }
            .into());
        }

        let declared =
            match self
                .compiler
                .dir_declared(self.context, symbol.module_id, self.profile)
            {
                Ok(declared) => declared,
                Err(_) => {
                    self.remote_nominal_layouts_in_progress.remove(&symbol);
                    return Ok(None);
                }
            };
        let checked = match self
            .compiler
            .dir_checked(self.context, symbol.module_id, self.profile)
        {
            Ok(checked) => checked,
            Err(_) => {
                self.remote_nominal_layouts_in_progress.remove(&symbol);
                return Ok(None);
            }
        };
        let Some(members) =
            self.struct_members_for_symbol(symbol, &declared.bindings, &declared.tree)
        else {
            self.remote_nominal_layouts_in_progress.remove(&symbol);
            return Ok(None);
        };

        let mut field_lowerer = TypeLowerer::new(
            builder,
            self.pointer_bytes(),
            self.compiler,
            self.context,
            self.compiler.repository.string_pool().as_ref(),
            self.profile,
            &declared.tree,
            &declared.bindings,
            self.vector_symbol,
        );
        let mut fields = Vec::new();

        for (source_index, member_id) in members.iter().enumerate() {
            let dir::Member::Field {
                key, declared_type, ..
            } = declared.tree.get(*member_id)
            else {
                continue;
            };
            let Some(key) = Self::static_key_from_member_key(key.clone()) else {
                continue;
            };
            let Some(declared_type) = declared_type else {
                continue;
            };
            let type_id = checked
                .types
                .get_declared_or_inferred_type_id(declared_type.into_global_any(symbol.module_id))
                .ok_or_else(|| LowerError::MissingType {
                    anchor: self.diagnostic_anchor(
                        declared_type
                            .into_global_any(symbol.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                })?;
            let field_type = field_lowerer.lower_type(
                &checked.types,
                type_id,
                symbol.module_id,
                node,
                builder,
            )?;
            let mir_type = builder.tree().get(field_type);
            let (size, alignment) = field_lowerer
                .size_and_align_of_type(mir_type, builder.tree())
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node),
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

        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Source);
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
        symbols: &dir::BindingTable,
        tree: &dir::Tree,
    ) -> Option<Vec<dir::LocalNodeId<dir::Member>>> {
        let declaration = symbols.get_symbol(symbol.local_id).declaration?;
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
        if !self.symbol_is(symbol, dir::SymbolForm::TypeAlias)
            && !self.symbol_is(symbol, dir::SymbolForm::Newtype)
        {
            return Ok(None);
        }

        let Some(name) = self.symbol_name(symbol) else {
            return Ok(None);
        };
        let name = self.strings.get(name).to_string();

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
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "ownership form missing base type".to_string(),
            }
            .into());
        };
        let Some(argument) = arguments.first() else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "ownership form missing base type".to_string(),
            }
            .into());
        };
        let dir::StaticArgument::Evaluated {
            value: dir::StaticExpression::Type { ty },
            ..
        } = argument
        else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "ownership form base must be a type".to_string(),
            }
            .into());
        };

        Ok(*ty)
    }

    /// Return the reference kind encoded by an ownership form.
    fn ownership_form_kind(
        &self,
        static_arguments: Option<&[dir::StaticArgument]>,
    ) -> Option<mir::ReferenceKind> {
        let ownership = self.static_string_argument(static_arguments?, 1)?;
        let ownership = self.strings.get(ownership);

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
        let space = self.strings.get(space);

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
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "reference address-space rewrite requires a concrete pointee".to_string(),
            }
            .into());
        };

        Ok(builder.type_reference(kind, pointee, mutability, address_space, is_nullable))
    }

    /// Return the source name for one symbol when artifacts are available.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<StringId> {
        let dir = self
            .compiler
            .dir_declared(self.context, symbol.module_id, self.profile)
            .ok()?;
        dir.bindings.get_symbol(symbol.local_id).name()
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

        let function_pointer_type = builder.type_function_pointer(signature);
        let env_pointer_type = builder.tree_mut().ensure_callable_environment_type();
        let function_pointer = builder.tree().get(function_pointer_type);
        let env_type = builder.tree().get(env_pointer_type);
        let (function_pointer_size, function_pointer_align) = self
            .size_and_align_of_type(function_pointer, builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "function layout requires concrete nested types".to_string(),
            })?;
        let (env_size, env_align) = self
            .size_and_align_of_type(env_type, builder.tree())
            .ok_or_else(|| LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "function layout requires concrete nested types".to_string(),
            })?;

        let fn_name = builder.intern(FUNCTION_PTR_FIELD);
        let env_name = builder.intern(ENV_FIELD);
        let fields = vec![
            FieldInput {
                name: fn_name,
                ty: function_pointer_type,
                size: function_pointer_size,
                alignment: function_pointer_align,
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
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Optimized);
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

    /// Lower a DIR integer type into a MIR type.
    fn mir_type_for_int_type(
        &mut self,
        int_type: dir::IntegerType,
        builder: &mut mir::ModuleBuilder,
    ) -> mir::LocalNodeId<mir::Type> {
        match int_type {
            dir::IntegerType::Fixed { width, is_signed } => builder.type_int(width, is_signed),
            dir::IntegerType::Pointer { is_signed: true } => self.ty_isize,
            dir::IntegerType::Pointer { is_signed: false } => self.ty_usize,
        }
    }
}
