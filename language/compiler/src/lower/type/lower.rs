use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use destack_artifact::DiagnosticAnchor;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use super::{FieldInput, FieldLayoutKind, LayoutPolicy, StructLayout, TypeLayoutPolicy};
use crate::lower::static_key_to_field_name;
use crate::{DynamicValueLayout, Compiler, LowerError, LowerResult, UnionLayout};

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
    pub(super) strings: &'a dir::StringPool,
    /// Profile used for cross-module artifact reads.
    pub(super) profile: ProfileId,
    /// DIR tree being lowered.
    pub(super) dir_tree: &'a dir::Tree,
    /// Symbol table for local declaration kind reads.
    pub(super) symbols: &'a dir::BindingTable<'a>,
    /// Cached Vector type symbol for vector lowering.
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
    /// Cached dynamic value layouts by DIR type id.
    pub(crate) dynamic_value_layout_cache: HashMap<dir::LocalTypeId, DynamicValueLayout>,
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
        strings: &'a dir::StringPool,
        profile: ProfileId,
        dir_tree: &'a dir::Tree,
        symbols: &'a dir::BindingTable<'a>,
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
            dynamic_value_layout_cache: HashMap::new(),
            function_signature_types: HashMap::new(),
            remote_nominal_layouts_by_symbol: HashMap::new(),
            remote_nominal_layouts_in_progress: HashSet::new(),
            layout_policy,
        }
    }

    /// Read one symbol record from local or bound DIR.
    pub(crate) fn symbol(&self, symbol: dir::GlobalSymbolId) -> Option<dir::Symbol> {
        if symbol.module_id == self.symbols.module_id {
            Some(self.symbols.get_symbol(symbol.local_id).clone())
        } else {
            let bound = self
                .compiler
                .artifact_reader(self.context)
                .dir_bound(symbol.module_id, self.profile)
                .ok()?;

            Some(bound.bindings.get_symbol(symbol.local_id).clone())
        }
    }

    /// Return the declaration kind for one symbol.
    pub(crate) fn symbol_kind(&self, symbol: dir::GlobalSymbolId) -> Option<dir::SymbolKind> {
        Some(self.symbol(symbol)?.kind)
    }

    /// Return whether one symbol has the given declaration kind.
    pub(crate) fn symbol_kind_matches(&self, symbol: dir::GlobalSymbolId, kind: dir::SymbolKind) -> bool {
        self.symbol_kind(symbol)
            .is_some_and(|actual| actual.matches_kind(kind))
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
        field_name: dir::StringId,
        strings: &dir::StringPool,
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
        types: &dir::TypeTable<'_>,
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
            let parameter_type = self.lower_type(types, parameter.ty, module_id, node, builder)?;
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
        types: &dir::TypeTable<'_>,
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
            && self.symbol_kind_matches(enum_symbol, dir::SymbolKind::Enum)
        {
            let mir_type = self.lower_nominal_enum_type(types, enum_symbol, node, builder)?;
            self.type_cache
                .insert(type_id, TypeCacheEntry::Ready(mir_type));
            return Ok(mir_type);
        }

        let mir_type = match dir_type {
            dir::Type::Named(reference) => self.lower_reference_type(
                types,
                type_id,
                reference.symbol,
                Some(reference.arguments.as_slice()),
                module_id,
                node,
                builder,
            )?,
            dir::Type::Form(form) => self.lower_form_type(types, form, module_id, node, builder)?,
            dir::Type::Shape(object) => {
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
                let mir_element =
                    self.lower_type(types, slice.element, module_id, node, builder)?;

                if mir_element == self.ty_void {
                    return Err(LowerError::UnsupportedType {
                        anchor: self.diagnostic_anchor(node),
                        ty: slice.element.into_global(module_id),
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
            dir::Type::Range(_) => {
                return Err(LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "range types must be reduced before native lowering".to_string(),
                }
                .into());
            }
            dir::Type::Function(_) => {
                self.lower_function_type(types, type_id, module_id, node, builder)?
            }
            dir::Type::Closure(closure) => {
                self.lower_closure_type(types, closure, module_id, node, builder)?
            }
            dir::Type::Dynamic(dynamic) => self.lower_dynamic_value_type(
                types,
                type_id,
                dynamic.constraint,
                module_id,
                node,
                builder,
            )?,
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
        types: &dir::TypeTable<'_>,
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
                        message: "missing language item String layout (load core)".to_string(),
                    })
            }
        }
    }

    /// Lower an enum symbol to its nominal MIR wrapper type.
    fn lower_nominal_enum_type(
        &mut self,
        types: &dir::TypeTable<'_>,
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
        types: &dir::TypeTable<'_>,
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

        if self.symbol_kind_matches(symbol, dir::SymbolKind::Interface) {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "dynamic constraints must be instantiated or erased with Dynamic<T>"
                    .to_string(),
            }
            .into());
        }
        if self.symbol_kind_matches(symbol, dir::SymbolKind::Enum) {
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
        let is_alias_with_target = self.symbol_kind_matches(symbol, dir::SymbolKind::TypeAlias)
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
        if self.symbol_kind_matches(symbol, dir::SymbolKind::Newtype) {
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

        let instance_type =
            self.lower_nominal_instance_type(types, type_id, symbol, module_id, node, builder)?;

        if self.symbol_kind_matches(symbol, dir::SymbolKind::Class) {
            Ok(builder.type_managed_reference(instance_type))
        } else {
            Ok(instance_type)
        }
    }

    /// Lower one canonical storage form.
    fn lower_form_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        form: &dir::FormType,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        match &form.form {
            dir::Form::Owned => {
                self.lower_value_representation_type(types, form.value, module_id, node, builder)
            }
            dir::Form::Managed | dir::Form::Placed { .. } | dir::Form::Readonly => {
                self.lower_type(types, form.value, module_id, node, builder)
            }
            dir::Form::Borrowed { .. } => self.lower_form_reference_type(
                types,
                form,
                mir::ReferenceKind::Borrowed,
                module_id,
                node,
                builder,
            ),
            dir::Form::Raw => self.lower_form_reference_type(
                types,
                form,
                mir::ReferenceKind::Raw,
                module_id,
                node,
                builder,
            ),
        }
    }

    /// Lower one form that produces a reference carrier.
    fn lower_form_reference_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        form: &dir::FormType,
        kind: mir::ReferenceKind,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let base_type =
            self.lower_value_representation_type(types, form.value, module_id, node, builder)?;
        let space = self.form_space(types, form.value, module_id, node)?;
        let access = self
            .form_access(form)
            .unwrap_or_else(|| self.default_reference_access(kind));

        Ok(builder.type_reference(kind, base_type, access, space, mir::Nullability::None))
    }

    /// Lower a type to its value representation, without default class indirection.
    fn lower_value_representation_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let dir::Type::Named(reference) = types.get_type(type_id) else {
            return self.lower_type(types, type_id, module_id, node, builder);
        };

        if !self.symbol_kind_matches(reference.symbol, dir::SymbolKind::Class) {
            return self.lower_type(types, type_id, module_id, node, builder);
        }

        self.lower_nominal_instance_type(types, type_id, reference.symbol, module_id, node, builder)
    }

    /// Lower one nominal type to its direct instance representation.
    fn lower_nominal_instance_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let instance_type_id =
            types
                .get_instance_type_id(symbol)
                .ok_or_else(|| LowerError::UnsupportedType {
                    anchor: self.diagnostic_anchor(node),
                    ty: type_id.into_global(module_id),
                    message: "type reference has no instance type".to_string(),
                })?;

        if instance_type_id != type_id {
            return self.lower_type(types, instance_type_id, module_id, node, builder);
        }

        if !matches!(
            self.symbol_kind(symbol),
            Some(dir::SymbolKind::TypeAlias | dir::SymbolKind::Newtype)
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

        self.lower_type(types, alias_target_id, module_id, node, builder)
    }

    /// Lower a remote nominal struct layout when its checked artifact is available.
    fn lower_remote_nominal_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        current_module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        if symbol.module_id == current_module_id || !self.symbol_kind_matches(symbol, dir::SymbolKind::Struct)
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

        let artifacts = self.compiler.artifact_reader(self.context);
        let bound = match artifacts.dir_bound(symbol.module_id, self.profile) {
            Ok(bound) => bound,
            Err(_) => {
                self.remote_nominal_layouts_in_progress.remove(&symbol);
                return Ok(None);
            }
        };
        let parsed = match artifacts.dir_parsed(symbol.module_id) {
            Ok(parsed) => parsed,
            Err(_) => {
                self.remote_nominal_layouts_in_progress.remove(&symbol);
                return Ok(None);
            }
        };
        let checked = match artifacts.dir_checked(symbol.module_id, self.profile) {
            Ok(checked) => checked,
            Err(_) => {
                self.remote_nominal_layouts_in_progress.remove(&symbol);
                return Ok(None);
            }
        };
        let expanded = match artifacts.dir_expanded(symbol.module_id, self.profile) {
            Ok(expanded) => expanded,
            Err(_) => {
                self.remote_nominal_layouts_in_progress.remove(&symbol);
                return Ok(None);
            }
        };
        let bindings = expanded.binding_table(&bound);
        let types = checked.type_table(&bound, &expanded);
        let Some(members) = self.struct_members_for_symbol(symbol, &bindings, &parsed.tree) else {
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
            &parsed.tree,
            &bindings,
            self.vector_symbol,
        );
        let mut fields = Vec::new();

        for (source_index, member_id) in members.iter().enumerate() {
            let dir::Member::Field {
                key, declared_type, ..
            } = parsed.tree.get(*member_id)
            else {
                continue;
            };
            let Some(key) = key.static_key(&parsed.tree) else {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(symbol.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "unsupported non-public field key in remote nominal layout"
                        .to_string(),
                }
                .into());
            };
            let Some(declared_type) = declared_type else {
                continue;
            };
            let type_id = types
                .get_declared_or_inferred_type_id(declared_type.into_global_any(symbol.module_id))
                .ok_or_else(|| LowerError::MissingType {
                    anchor: self.diagnostic_anchor(
                        declared_type
                            .into_global_any(symbol.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                })?;
            let field_type =
                field_lowerer.lower_type(&types, type_id, symbol.module_id, node, builder)?;
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
        symbols: &dir::BindingTable<'_>,
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

    /// Lower an intrinsic ownership alias into a MIR reference type.
    fn lower_ownership_alias_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        symbol: dir::GlobalSymbolId,
        static_arguments: Option<&[dir::StaticArgument]>,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        if !self.symbol_kind_matches(symbol, dir::SymbolKind::TypeAlias)
            && !self.symbol_kind_matches(symbol, dir::SymbolKind::Newtype)
        {
            return Ok(None);
        }

        let Some(name) = self.symbol_name(symbol) else {
            return Ok(None);
        };
        let name = self.strings.get(name).to_string();

        if name == "Managed" {
            let base_type_id =
                self.first_type_static_argument(static_arguments, type_id, module_id, node)?;

            return self
                .lower_type(types, base_type_id, module_id, node, builder)
                .map(Some);
        }

        if name == "Owned" {
            let base_type_id =
                self.first_type_static_argument(static_arguments, type_id, module_id, node)?;

            return self
                .lower_value_representation_type(types, base_type_id, module_id, node, builder)
                .map(Some);
        }

        if name == "Form" {
            return self.lower_static_form_alias_type(
                types,
                type_id,
                static_arguments,
                module_id,
                node,
                builder,
            );
        }

        let kind = match name.as_str() {
            "Unique" => Some(mir::ReferenceKind::Unique),
            "Borrowed" => Some(mir::ReferenceKind::Borrowed),
            "Raw" => Some(mir::ReferenceKind::Raw),
            "Shared" => {
                let inner_type_id =
                    self.first_type_static_argument(static_arguments, type_id, module_id, node)?;
                let inner_type = self.lower_type(types, inner_type_id, module_id, node, builder)?;
                let shared_type = self.rewrite_reference_space(
                    inner_type,
                    mir::Space::Shared,
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
        let base_type =
            self.lower_value_representation_type(types, base_type_id, module_id, node, builder)?;
        let space = self.ownership_form_space(static_arguments, type_id, module_id, node)?;
        let access = self.default_reference_access(kind);

        Ok(Some(builder.type_reference(
            kind,
            base_type,
            access,
            space,
            mir::Nullability::None,
        )))
    }

    /// Lower a statically parameterized `Form` alias.
    fn lower_static_form_alias_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        static_arguments: Option<&[dir::StaticArgument]>,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        let base_type_id =
            self.first_type_static_argument(static_arguments, type_id, module_id, node)?;
        let Some(arguments) = static_arguments else {
            return Ok(None);
        };
        let ownership = self
            .static_string_argument(arguments, 1)
            .map(|ownership| self.strings.get(ownership));

        match ownership.as_deref() {
            Some("owned") => self
                .lower_value_representation_type(types, base_type_id, module_id, node, builder)
                .map(Some),
            Some("managed") | None => self
                .lower_type(types, base_type_id, module_id, node, builder)
                .map(Some),
            Some("borrowed") => self
                .lower_static_form_reference_type(
                    types,
                    base_type_id,
                    arguments,
                    mir::ReferenceKind::Borrowed,
                    module_id,
                    node,
                    builder,
                )
                .map(Some),
            Some("raw") => self
                .lower_static_form_reference_type(
                    types,
                    base_type_id,
                    arguments,
                    mir::ReferenceKind::Raw,
                    module_id,
                    node,
                    builder,
                )
                .map(Some),
            _ => Ok(None),
        }
    }

    /// Lower a statically parameterized `Form` reference carrier.
    fn lower_static_form_reference_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        base_type_id: dir::LocalTypeId,
        static_arguments: &[dir::StaticArgument],
        kind: mir::ReferenceKind,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let base_type =
            self.lower_value_representation_type(types, base_type_id, module_id, node, builder)?;
        let space = self.static_form_space(static_arguments, base_type_id, module_id, node)?;
        let access = self
            .static_form_access(static_arguments)
            .unwrap_or_else(|| self.default_reference_access(kind));

        Ok(builder.type_reference(kind, base_type, access, space, mir::Nullability::None))
    }

    /// Return the default access for one ownership kind.
    fn default_reference_access(&self, kind: mir::ReferenceKind) -> mir::Access {
        match kind {
            mir::ReferenceKind::Managed | mir::ReferenceKind::Raw => mir::Access::Readonly,
            mir::ReferenceKind::Unique | mir::ReferenceKind::Borrowed => mir::Access::Mutable,
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
        let dir::StaticArgument {
            value: dir::StaticTerm::Type { ty },
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

    /// Return the space encoded by an ownership form.
    fn ownership_form_space(
        &self,
        static_arguments: Option<&[dir::StaticArgument]>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Space> {
        let Some(arguments) = static_arguments else {
            return Ok(mir::Space::Local);
        };

        self.static_form_space(arguments, type_id, module_id, node)
    }

    /// Return the space encoded by static `Form` arguments.
    fn static_form_space(
        &self,
        arguments: &[dir::StaticArgument],
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Space> {
        let Some(space) = self.static_string_argument(arguments, 2) else {
            return Ok(mir::Space::Local);
        };
        let space = self.strings.get(space);

        if space == "ambient" {
            return Ok(mir::Space::Local);
        }

        mir::Space::from_name(space.as_ref()).ok_or_else(|| {
            LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: format!("unknown space '{space}'"),
            }
            .into()
        })
    }

    /// Return the access encoded by static `Form` arguments.
    fn static_form_access(&self, arguments: &[dir::StaticArgument]) -> Option<mir::Access> {
        let access = self.static_string_argument(arguments, 4)?;
        let access = self.strings.get(access);

        match access.as_ref() {
            "readonly" => Some(mir::Access::Readonly),
            "mutable" => Some(mir::Access::Mutable),
            "exclusive" => Some(mir::Access::Exclusive),
            _ => None,
        }
    }

    /// Return the space encoded by a resolved `Form` type.
    fn form_space(
        &self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::Space> {
        let dir::Type::Form(form) = types.get_type(type_id) else {
            return Ok(mir::Space::Local);
        };
        let dir::Form::Placed { place } = &form.form else {
            return Ok(mir::Space::Local);
        };
        let Some(place) = self.static_string_term(place) else {
            return Ok(mir::Space::Local);
        };
        let place_text = self.strings.get(place);

        if place_text == "ambient" {
            return Ok(mir::Space::Local);
        }

        mir::Space::from_name(place_text.as_ref()).ok_or_else(|| {
            LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: format!("unknown space '{place_text}'"),
            }
            .into()
        })
    }

    /// Return the access encoded by a resolved `Form` type.
    fn form_access(&self, form: &dir::FormType) -> Option<mir::Access> {
        let dir::Form::Borrowed { access, .. } = &form.form else {
            return None;
        };
        let access = self.static_string_term(access)?;
        let access = self.strings.get(access);

        match access.as_ref() {
            "readonly" => Some(mir::Access::Readonly),
            "mutable" => Some(mir::Access::Mutable),
            "exclusive" => Some(mir::Access::Exclusive),
            _ => None,
        }
    }

    /// Return a string literal encoded as a static term.
    fn static_string_term(&self, term: &dir::StaticTerm) -> Option<dir::StringId> {
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::String(value),
        } = term
        else {
            return None;
        };

        Some(*value)
    }

    /// Return one string static argument.
    fn static_string_argument(
        &self,
        arguments: &[dir::StaticArgument],
        index: usize,
    ) -> Option<dir::StringId> {
        let dir::StaticArgument {
            value:
                dir::StaticTerm::ScalarLiteral {
                    value: dir::ScalarLiteral::String(value),
                },
            ..
        } = arguments.get(index)?
        else {
            return None;
        };

        Some(*value)
    }

    /// Rewrite a lowered reference into another space.
    fn rewrite_reference_space(
        &self,
        ty: mir::LocalNodeId<mir::Type>,
        space: mir::Space,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let mir_type = builder.tree().get(ty).clone();
        let mir::Type::Reference {
            kind,
            access,
            pointee,
            nullability,
            ..
        } = mir_type
        else {
            return Ok(ty);
        };
        let Some(pointee) = pointee.ty() else {
            return Err(LowerError::UnsupportedType {
                anchor: self.diagnostic_anchor(node),
                ty: type_id.into_global(module_id),
                message: "reference address-space rewrite requires a concrete pointee".to_string(),
            }
            .into());
        };

        Ok(builder.type_reference_with_lifetime(
            kind,
            mir::Lifetime::empty(),
            pointee,
            access,
            space,
            nullability,
        ))
    }

    /// Return the source name for one symbol when artifacts are available.
    fn symbol_name(&self, symbol: dir::GlobalSymbolId) -> Option<dir::StringId> {
        let dir = self
            .compiler
            .artifact_reader(self.context)
            .dir_bound(symbol.module_id, self.profile)
            .ok()?;
        let expanded = self
            .compiler
            .artifact_reader(self.context)
            .dir_expanded(symbol.module_id, self.profile)
            .ok()?;
        let bindings = expanded.binding_table(&dir);

        bindings.get_symbol(symbol.local_id).name()
    }

    /// Lower a function type into its closure-pair representation.
    fn lower_function_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let signature =
            self.lower_function_signature_type(types, type_id, module_id, node, builder)?;

        let function_pointer_type = builder.type_function_pointer(signature);
        let env_pointer_type = builder.tree_mut().ensure_closure_environment_type();
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

        let mir_type = builder.type_closure(signature, env_pointer_type);
        let layout = Self::compute_struct_layout(fields, LayoutPolicy::Optimized);
        self.set_layout(mir_type, layout);
        Ok(mir_type)
    }

    /// Lower a DIR closure type into its MIR representation.
    fn lower_closure_type(
        &mut self,
        types: &dir::TypeTable<'_>,
        closure: &dir::ClosureType,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let signature =
            self.lower_function_signature_type(types, closure.function, module_id, node, builder)?;
        let environment = self.lower_type(types, closure.environment, module_id, node, builder)?;

        Ok(builder.type_closure(signature, environment))
    }

    /// Lower an intersection type by selecting its primary element.
    fn lower_intersection_type(
        &mut self,
        types: &dir::TypeTable<'_>,
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
            dir::IntegerType::Integer { .. } => todo!("lower generic integer type"),
            dir::IntegerType::Fixed { width, is_signed } => builder.type_int(width, is_signed),
            dir::IntegerType::Pointer { is_signed: true } => self.ty_isize,
            dir::IntegerType::Pointer { is_signed: false } => self.ty_usize,
        }
    }
}
