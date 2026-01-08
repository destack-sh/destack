use std::collections::HashMap;

use destack_base::StringId;
use destack_dir::GlobalNodeIdAny;
use {destack_dir as dir, destack_mir as mir};

use super::{
    FieldInput, LayoutPolicy, StructLayout, compute_struct_layout, size_and_align_of_type,
};
use crate::{LowerError, LowerResult};

/// Convert a static key to a field name.
///
/// For name and number keys, returns the string directly.
/// For symbol keys, generates a synthetic name with `@` prefix to avoid conflicts.
fn static_key_to_field_name(key: &dir::StaticKey, builder: &mut mir::ModuleBuilder) -> StringId {
    match key {
        dir::StaticKey::Name(s) | dir::StaticKey::Number(s) => *s,
        dir::StaticKey::Symbol(symbol_key) => {
            let synthetic = match symbol_key {
                dir::SymbolKey::WellKnown(well_known) => {
                    format!("@{}", well_known.global_symbol_name())
                }
                dir::SymbolKey::Registry(s) => {
                    let key_str = builder.strings().get(*s);
                    format!("@Symbol.for:{}", &*key_str)
                }
                dir::SymbolKey::Unique(global_id) => {
                    format!("@Symbol#{global_id:?}")
                }
            };
            builder.intern(&synthetic)
        }
    }
}

/// Scalar type classification for lowering decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarType {
    /// Boolean scalar.
    Bool,
    /// Signed integer scalar.
    SignedInt {
        /// Bit width.
        width: u16,
    },
    /// Unsigned integer scalar.
    UnsignedInt {
        /// Bit width.
        width: u16,
    },
    /// Floating point scalar.
    Float {
        /// Bit width.
        width: u16,
    },
}

/// Lowers DIR types into MIR types with a shared cache.
#[derive(Debug)]
pub(crate) struct TypeLowerer {
    /// Cached MIR types by DIR type id.
    pub(crate) type_cache: HashMap<dir::LocalTypeId, mir::LocalNodeId<mir::Type>>,
    /// Pointer width in bits for pointer-sized integers.
    pointer_width_bits: u16,
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
}

impl TypeLowerer {
    /// Create a new type lowerer with cached common types.
    pub(crate) fn new(builder: &mut mir::ModuleBuilder, pointer_bytes: u8) -> Self {
        let pointer_width_bits = u16::from(pointer_bytes) * 8;

        Self {
            type_cache: HashMap::new(),
            pointer_width_bits,
            ty_void: builder.type_void(),
            ty_bool: builder.type_bool(),
            ty_i32: builder.type_i32(),
            ty_i64: builder.type_i64(),
            ty_f32: builder.type_f32(),
            ty_f64: builder.type_f64(),
        }
    }

    /// Lower a DIR type to a MIR type.
    pub(crate) fn lower_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: destack_source::ModuleId,
        node: GlobalNodeIdAny,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(mir_type) = self.type_cache.get(&type_id) {
            return Ok(*mir_type);
        }

        let dir_type = types.get_type(type_id);
        let mir_type = match dir_type {
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

    /// Try to lower a DIR type to a MIR type.
    fn try_lower_type(
        &mut self,
        dir_type: &dir::Type,
        builder: &mut mir::ModuleBuilder,
    ) -> Option<mir::LocalNodeId<mir::Type>> {
        match dir_type {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Void,
            } => Some(self.ty_void),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            } => Some(self.ty_bool),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Number),
            } => Some(self.ty_f64),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            } => match float_type.simplify() {
                dir::FloatType::Float32 => Some(self.ty_f32),
                dir::FloatType::Float64 => Some(self.ty_f64),
                dir::FloatType::Arbitrary { width } => Some(builder.type_float(width)),
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(int_type)),
            } => match int_type.simplify() {
                dir::IntType::Int32 => Some(self.ty_i32),
                dir::IntType::Int64 => Some(self.ty_i64),
                dir::IntType::Uint32 => Some(builder.type_int(32, false)),
                dir::IntType::Uint64 => Some(builder.type_int(64, false)),
                dir::IntType::Int8 => Some(builder.type_int(8, true)),
                dir::IntType::Int16 => Some(builder.type_int(16, true)),
                dir::IntType::Int128 => Some(builder.type_int(128, true)),
                dir::IntType::Int256 => Some(builder.type_int(256, true)),
                dir::IntType::Uint8 => Some(builder.type_int(8, false)),
                dir::IntType::Uint16 => Some(builder.type_int(16, false)),
                dir::IntType::Uint128 => Some(builder.type_int(128, false)),
                dir::IntType::Uint256 => Some(builder.type_int(256, false)),
                dir::IntType::Isize => Some(builder.type_int(self.pointer_width_bits, true)),
                dir::IntType::Usize => Some(builder.type_int(self.pointer_width_bits, false)),
                dir::IntType::Arbitrary { width, is_signed } => {
                    Some(builder.type_int(width, is_signed))
                }
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            } => match literal {
                dir::ScalarLiteral::Boolean(_) => Some(self.ty_bool),
                dir::ScalarLiteral::Integer(_) => Some(self.ty_i32),
                dir::ScalarLiteral::Float(_) => Some(self.ty_f64),
                dir::ScalarLiteral::Character(_)
                | dir::ScalarLiteral::String(_)
                | dir::ScalarLiteral::Bigint(_)
                | dir::ScalarLiteral::RegexString { .. } => None,
            },
            _ => None,
        }
    }

    /// Get the pointer size in bytes for this target.
    pub(crate) fn pointer_bytes(&self) -> u8 {
        (self.pointer_width_bits / 8) as u8
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

        for field in &layout.fields {
            let mir_field = builder.field(Some(field.name), field.ty, field.offset);
            mir_fields.push(mir_field);
        }

        builder.type_struct(mir_fields)
    }

    /// Lower a DIR object type to a MIR struct type.
    ///
    /// This computes the layout for the struct fields and creates the MIR type.
    fn lower_object_type(
        &mut self,
        types: &dir::TypeTable,
        fields: &[dir::TypeField],
        module_id: destack_source::ModuleId,
        node: GlobalNodeIdAny,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let pointer_bytes = self.pointer_bytes();
        let mut field_inputs = Vec::with_capacity(fields.len());

        for (source_index, field) in fields.iter().enumerate() {
            // convert the field key to a name (handles symbols with synthetic names)
            let name = static_key_to_field_name(&field.key, builder);

            // lower the field's type and compute size/alignment
            let field_mir_type = self.lower_type(types, field.ty, module_id, node, builder)?;
            let field_type = builder.tree().get(field_mir_type);
            let (size, alignment) =
                size_and_align_of_type(field_type, builder.tree(), pointer_bytes);

            field_inputs.push(FieldInput {
                name,
                ty: field_mir_type,
                size,
                alignment,
                source_index: source_index as u32,
            });
        }

        // compute the layout and create the MIR struct type
        let layout = compute_struct_layout(field_inputs, LayoutPolicy::default());

        Ok(self.create_struct_type(&layout, builder))
    }

    /// Lower a DIR tuple type to a MIR tuple type.
    fn lower_tuple_type(
        &mut self,
        types: &dir::TypeTable,
        elements: &[dir::TypeElement],
        module_id: destack_source::ModuleId,
        node: GlobalNodeIdAny,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // reject optional and rest elements for now
        for element in elements {
            if element.is_optional {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: element.ty.into_global(module_id),
                    message: "optional tuple elements are not yet supported".to_string(),
                });
            }
            if element.is_rest {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: element.ty.into_global(module_id),
                    message: "rest tuple elements are not yet supported".to_string(),
                });
            }
        }

        // lower each element type
        let mut mir_elements = Vec::with_capacity(elements.len());
        for element in elements {
            let mir_type = self.lower_type(types, element.ty, module_id, node, builder)?;
            mir_elements.push(mir_type);
        }

        Ok(builder.type_tuple(mir_elements))
    }

    /// Lower a DIR sized array type to a MIR array type.
    fn lower_array_sized_type(
        &mut self,
        types: &dir::TypeTable,
        element: dir::LocalTypeId,
        count: dir::LocalNodeId<dir::Expression>,
        module_id: destack_source::ModuleId,
        node: GlobalNodeIdAny,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // lower the element type
        let mir_element = self.lower_type(types, element, module_id, node, builder)?;

        // get the inferred type of the count expression
        let count_node = count.into_global_any(module_id);
        let length = types
            .get_declared_or_inferred_type_id(count_node)
            .and_then(|type_id| {
                let ty = types.get_type(type_id);
                match ty {
                    dir::Type::TypeLiteral {
                        value: dir::TypeLiteral::ScalarLiteral(dir::ScalarLiteral::Integer(n)),
                    } => Some(*n as u64),
                    _ => None,
                }
            })
            .ok_or_else(|| LowerError::UnsupportedType {
                node,
                ty: element.into_global(module_id),
                message: "array size must be a constant integer".to_string(),
            })?;

        Ok(builder.type_array(mir_element, length))
    }

    /// Resolve a scalar type for a given DIR type.
    pub(crate) fn scalar_type_for_dir_type(&self, dir_type: &dir::Type) -> Option<ScalarType> {
        match dir_type {
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Boolean),
            } => Some(ScalarType::Bool),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Number),
            } => Some(ScalarType::Float { width: 64 }),
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Float(float_type)),
            } => match float_type.simplify() {
                dir::FloatType::Float32 => Some(ScalarType::Float { width: 32 }),
                dir::FloatType::Float64 => Some(ScalarType::Float { width: 64 }),
                dir::FloatType::Arbitrary { width } => Some(ScalarType::Float { width }),
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::Primitive(dir::PrimitiveType::Int(int_type)),
            } => match int_type.simplify() {
                dir::IntType::Int8 => Some(ScalarType::SignedInt { width: 8 }),
                dir::IntType::Int16 => Some(ScalarType::SignedInt { width: 16 }),
                dir::IntType::Int32 => Some(ScalarType::SignedInt { width: 32 }),
                dir::IntType::Int64 => Some(ScalarType::SignedInt { width: 64 }),
                dir::IntType::Int128 => Some(ScalarType::SignedInt { width: 128 }),
                dir::IntType::Int256 => Some(ScalarType::SignedInt { width: 256 }),
                dir::IntType::Uint8 => Some(ScalarType::UnsignedInt { width: 8 }),
                dir::IntType::Uint16 => Some(ScalarType::UnsignedInt { width: 16 }),
                dir::IntType::Uint32 => Some(ScalarType::UnsignedInt { width: 32 }),
                dir::IntType::Uint64 => Some(ScalarType::UnsignedInt { width: 64 }),
                dir::IntType::Uint128 => Some(ScalarType::UnsignedInt { width: 128 }),
                dir::IntType::Uint256 => Some(ScalarType::UnsignedInt { width: 256 }),
                dir::IntType::Isize => Some(ScalarType::SignedInt {
                    width: self.pointer_width_bits,
                }),
                dir::IntType::Usize => Some(ScalarType::UnsignedInt {
                    width: self.pointer_width_bits,
                }),
                dir::IntType::Arbitrary { width, is_signed } => {
                    if is_signed {
                        Some(ScalarType::SignedInt { width })
                    } else {
                        Some(ScalarType::UnsignedInt { width })
                    }
                }
            },
            dir::Type::TypeLiteral {
                value: dir::TypeLiteral::ScalarLiteral(literal),
            } => match literal {
                dir::ScalarLiteral::Boolean(_) => Some(ScalarType::Bool),
                dir::ScalarLiteral::Integer(_) => Some(ScalarType::SignedInt { width: 32 }),
                dir::ScalarLiteral::Float(_) => Some(ScalarType::Float { width: 64 }),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Name keys return the string directly.
    #[test]
    fn test_static_key_name() {
        let mut builder = mir::ModuleBuilder::new();
        let name = builder.intern("foo");
        let key = dir::StaticKey::Name(name);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, name);
    }

    /// Number keys return the string directly.
    #[test]
    fn test_static_key_number() {
        let mut builder = mir::ModuleBuilder::new();
        let num = builder.intern("42");
        let key = dir::StaticKey::Number(num);

        let result = static_key_to_field_name(&key, &mut builder);
        assert_eq!(result, num);
    }

    /// Well-known symbol keys get synthetic names with @ prefix.
    #[test]
    fn test_static_key_well_known_symbol() {
        let mut builder = mir::ModuleBuilder::new();
        let key = dir::StaticKey::Symbol(dir::SymbolKey::WellKnown(
            dir::WellKnownSymbolKey::SymbolIterator,
        ));

        let result = static_key_to_field_name(&key, &mut builder);
        let result_str = builder.strings().get(result);
        assert_eq!(&*result_str, "@Symbol.iterator");
    }

    /// Registry symbol keys get synthetic names with @ prefix.
    #[test]
    fn test_static_key_registry_symbol() {
        let mut builder = mir::ModuleBuilder::new();
        let registry_key = builder.intern("myKey");
        let key = dir::StaticKey::Symbol(dir::SymbolKey::Registry(registry_key));

        let result = static_key_to_field_name(&key, &mut builder);
        let result_str = builder.strings().get(result);
        assert_eq!(&*result_str, "@Symbol.for:myKey");
    }
}
