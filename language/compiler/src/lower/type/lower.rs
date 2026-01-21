use std::collections::HashMap;
use std::sync::Arc;

use destack_ast::{StringId, StringPool};
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use destack_workspace::{ModuleRegistry, PackageRegistry};
use {destack_dir as dir, destack_mir as mir};

use super::{StructLayout, TypeLayoutPolicy};
use crate::lower::item::lower_mutability;
use crate::{InterfaceRefLayout, LowerError, LowerResult, UnionLayout};

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
    /// Access to module metadata for qualified names.
    pub(super) modules: Arc<ModuleRegistry>,
    /// Access to package metadata for qualified names.
    pub(super) packages: Arc<PackageRegistry>,
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
    /// Policy values for layout decisions.
    pub(crate) layout_policy: TypeLayoutPolicy,
}

impl TypeLowerer {
    /// Create a new type lowerer with cached common types.
    pub(crate) fn new(
        builder: &mut mir::ModuleBuilder,
        pointer_bytes: u8,
        modules: Arc<ModuleRegistry>,
        packages: Arc<PackageRegistry>,
    ) -> Self {
        let pointer_width_bits = u16::from(pointer_bytes) * 8;

        let layout_policy = TypeLayoutPolicy::for_target(pointer_bytes);

        Self {
            modules,
            packages,
            type_cache: HashMap::new(),
            layout_cache: HashMap::new(),
            pointer_width_bits,
            ty_void: builder.type_void(),
            ty_bool: builder.type_bool(),
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
        node: AnchoredGlobalNodeId,
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
                    target_ty = *pointee;
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
    /// This creates the MIR `Type::Struct` with fields that have their offsets
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

        // populate field nodes with computed offsets
        for field in &layout.fields {
            let mir_field = builder.field(Some(field.name), field.ty, field.offset);
            mir_fields.push(mir_field);
        }

        // return the struct type
        builder.type_struct(mir_fields, copyability)
    }

    /// Lower a DIR type to a MIR type.
    pub(crate) fn lower_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: AnchoredGlobalNodeId,
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

        // map enum instance types to their backing representation
        if let Some(enum_symbol) = types.symbol_for_instance_type(type_id)
            && enum_symbol.ty() == dir::SymbolType::Enum
        {
            let mir_type = self.lower_enum_backing_type(types, enum_symbol, node, builder)?;
            self.type_cache
                .insert(type_id, TypeCacheEntry::Ready(mir_type));
            return Ok(mir_type);
        }

        let mir_type = match dir_type {
            dir::Type::Reference { symbol, .. } => {
                if symbol.ty() == dir::SymbolType::Interface {
                    self.lower_interface_reference_type(types, type_id, module_id, node, builder)?
                } else if symbol.ty() == dir::SymbolType::Enum {
                    self.lower_enum_backing_type(types, *symbol, node, builder)?
                } else {
                    // follow the reference to its instance type
                    let instance_type_id =
                        types.get_instance_type_id(*symbol).ok_or_else(|| {
                            LowerError::UnsupportedType {
                                node,
                                ty: type_id.into_global(module_id),
                                message: "type reference has no instance type".to_string(),
                            }
                        })?;

                    // unwrap nominal aliases that point at themselves
                    let instance_type = if instance_type_id == type_id {
                        // check if the type is an invalid / self-referential alias
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
                        let Some(alias_target_id) = types.get_alias_target_type_id(*symbol) else {
                            return Err(LowerError::UnsupportedType {
                                node,
                                ty: type_id.into_global(module_id),
                                message: "type alias has no target type".to_string(),
                            });
                        };

                        // lower the alias target type for layout
                        self.lower_type(types, alias_target_id, module_id, node, builder)?
                    } else {
                        // recursively lower the instance type
                        self.lower_type(types, instance_type_id, module_id, node, builder)?
                    };

                    // wrap class instance types in a managed reference
                    if symbol.ty() == dir::SymbolType::Class {
                        builder.type_managed_reference(instance_type)
                    } else {
                        instance_type
                    }
                }
            }
            dir::Type::ValueOf {
                mutability, right, ..
            } => {
                // lower the owned reference pointee type
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Immutable);
                builder.type_owned_reference(pointee, mutability)
            }
            dir::Type::ReferenceOf {
                mutability, right, ..
            } => {
                // lower the borrowed reference pointee type
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                let mutability = mutability
                    .map(lower_mutability)
                    .unwrap_or(mir::Mutability::Immutable);
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
            dir::Type::ArraySized {
                element,
                count,
                ..
            } => {
                self.lower_array_sized_type(types, *element, *count, module_id, node, builder)?
            }
            dir::Type::Function {
                dynamic_parameters,
                return_type,
                ..
            } => {
                // lower parameter and return types for function pointers
                let mut parameters = Vec::with_capacity(dynamic_parameters.len());
                for parameter in dynamic_parameters {
                    let parameter_type =
                        self.lower_type(types, *parameter, module_id, node, builder)?;
                    parameters.push(parameter_type);
                }

                let result = match return_type {
                    Some(return_type) => {
                        self.lower_type(types, *return_type, module_id, node, builder)?
                    }
                    None => self.ty_void,
                };

                builder.type_function_pointer(parameters, result)
            }
            dir::Type::Union { elements } => {
                self.lower_union_type(types, type_id, elements, module_id, node, builder)?
            }
            dir::Type::Intersection { elements } => {
                let primary =
                    self.select_intersection_primary_type(types, elements, module_id, node)?;
                self.lower_type(types, primary, module_id, node, builder)?
            }
            _ => self
                .try_lower_type(dir_type, builder)
                .ok_or(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "unsupported type".to_string(),
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
        node: AnchoredGlobalNodeId,
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
                        message: "missing builtin String layout (load lib/native)".to_string(),
                    })
            }
        }
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
