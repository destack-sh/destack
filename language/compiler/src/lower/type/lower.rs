use std::collections::HashMap;

use destack_ast::{StringId, StringPool};
use destack_dir::AnchoredGlobalNodeId;
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::StructLayout;
use crate::{LowerError, LowerResult};

/// Lowers DIR types into MIR types with a shared cache.
#[derive(Debug)]
pub(crate) struct TypeLowerer {
    /// Cached MIR types by DIR type id.
    pub(crate) type_cache: HashMap<dir::LocalTypeId, mir::LocalNodeId<mir::Type>>,
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
    /// Cached MIR f32 type.
    pub(crate) ty_f32: mir::LocalNodeId<mir::Type>,
    /// Cached MIR f64 type.
    pub(crate) ty_f64: mir::LocalNodeId<mir::Type>,
    /// Cached MIR string reference type.
    pub(crate) ty_string: Option<mir::LocalNodeId<mir::Type>>,
}

impl TypeLowerer {
    /// Create a new type lowerer with cached common types.
    pub(crate) fn new(builder: &mut mir::ModuleBuilder, pointer_bytes: u8) -> Self {
        let pointer_width_bits = u16::from(pointer_bytes) * 8;

        Self {
            type_cache: HashMap::new(),
            layout_cache: HashMap::new(),
            pointer_width_bits,
            ty_void: builder.type_void(),
            ty_bool: builder.type_bool(),
            ty_i32: builder.type_i32(),
            ty_i64: builder.type_i64(),
            ty_f32: builder.type_f32(),
            ty_f64: builder.type_f64(),
            ty_string: None,
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
    /// For structs, uses the cached layout. For tuples, parses the numeric field name.
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
        let mut mir_fields = Vec::with_capacity(layout.fields.len());

        // compute copyability from field types
        let mut copyability = mir::Copyability::Trivial;
        for field in &layout.fields {
            let field_type = builder.tree().get(field.ty);
            copyability = copyability.combine(field_type.copyability());
        }

        for field in &layout.fields {
            let mir_field = builder.field(Some(field.name), field.ty, field.offset);
            mir_fields.push(mir_field);
        }

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
        if let Some(mir_type) = self.type_cache.get(&type_id) {
            return Ok(*mir_type);
        }

        let dir_type = types.get_type(type_id);
        let mir_type = match dir_type {
            dir::Type::Reference { symbol, .. } => {
                // follow the reference to its instance type
                let instance_type_id = types.get_instance_type_id(*symbol).ok_or_else(|| {
                    LowerError::UnsupportedType {
                        node,
                        ty: type_id.into_global(module_id),
                        message: "type reference has no instance type".to_string(),
                    }
                })?;
                // recursively lower the instance type
                let instance_type =
                    self.lower_type(types, instance_type_id, module_id, node, builder)?;

                // wrap class instance types in a managed reference
                let mir_type = if symbol.ty() == dir::SymbolType::Class {
                    builder.type_managed_reference(instance_type)
                } else {
                    instance_type
                };

                // cache this reference type id as well so we don't re-resolve next time
                self.type_cache.insert(type_id, mir_type);
                return Ok(mir_type);
            }
            dir::Type::PointerOf { right, .. } => {
                let pointee = self.lower_type(types, *right, module_id, node, builder)?;
                builder.type_raw_pointer(pointee)
            }
            dir::Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                // skip types with call/construct/index signatures for now
                if !call_signatures.is_empty()
                    || !construct_signatures.is_empty()
                    || !index_signatures.is_empty()
                {
                    return Err(LowerError::UnsupportedType {
                        node,
                        ty: type_id.into_global(module_id),
                        message: "object types with call, construct, or index signatures are not yet supported".to_string(),
                    });
                }

                self.lower_object_type(types, fields, module_id, node, builder)?
            }
            dir::Type::Tuple { elements } => {
                self.lower_tuple_type(types, elements, module_id, node, builder)?
            }
            dir::Type::ArraySized { element, count } => {
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
            _ => self
                .try_lower_type(dir_type, builder)
                .ok_or(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "unsupported type".to_string(),
                })?,
        };
        self.type_cache.insert(type_id, mir_type);
        Ok(mir_type)
    }
}
