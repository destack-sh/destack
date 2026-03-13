use crate::{LayoutType, LocalNodeIdAny, NodeType, Type};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate module data layout metadata invariants.
    pub(super) fn validate_data_layout_metadata(&self) -> ValidateResult<()> {
        let anchor = self
            .tree
            .node_type_by_node_id
            .first()
            .copied()
            .map(|ty| ValidateAnchor {
                node: LocalNodeIdAny::new(0, ty),
            })
            .unwrap_or(ValidateAnchor {
                node: LocalNodeIdAny::new(0, NodeType::Type),
            });

        match self.tree.pointer_bytes() {
            4 | 8 => {}
            pointer_bytes => Err(ValidateError::MetadataInvariantViolation {
                message: format!("unsupported pointer size {pointer_bytes} bytes"),
                anchor,
            })?,
        }

        match self.tree.data_layout.managed_reference_layout.bytes {
            4 | 8 => {}
            managed_reference_bytes => {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: format!(
                        "unsupported managed reference size {managed_reference_bytes} bytes"
                    ),
                    anchor,
                });
            }
        }

        if self.tree.data_layout.managed_reference_layout.alignment == 0 {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "managed reference alignment must be non zero".to_string(),
                anchor,
            });
        }

        if !self
            .tree
            .data_layout
            .managed_reference_layout
            .alignment
            .is_power_of_two()
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "managed reference alignment must be a power of two".to_string(),
                anchor,
            });
        }

        let managed_layout = self.tree.data_layout.managed_reference_layout;
        match managed_layout.representation {
            crate::ManagedReferenceRepresentation::NativePointer => {
                if managed_layout.bytes != self.tree.data_layout.native_pointer_bytes {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "native-pointer managed references must match native pointer size"
                            .to_string(),
                        anchor,
                    });
                }
            }
            crate::ManagedReferenceRepresentation::CompressedOffset32
            | crate::ManagedReferenceRepresentation::Handle32 => {
                if managed_layout.bytes != 4 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "32 bit managed reference representations must use 4 byte storage"
                            .to_string(),
                        anchor,
                    });
                }
            }
            crate::ManagedReferenceRepresentation::Handle64 => {
                if managed_layout.bytes != 8 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "64 bit managed reference representations must use 8 byte storage"
                            .to_string(),
                        anchor,
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate layout metadata consistency for aggregate types.
    pub(super) fn validate_layout_metadata_consistency(&self) -> ValidateResult<()> {
        for (&type_id, &layout_id) in &self.tree.type_table.layout_by_type {
            let Some(layout) = self
                .tree
                .type_table
                .layout_table
                .layouts
                .get(layout_id.index())
            else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "type table references missing layout metadata".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            };

            let ty = self.tree.get(type_id);
            match (ty, &layout.layout_type) {
                (
                    Type::Struct { fields, .. },
                    LayoutType::Struct
                    | LayoutType::Union { .. }
                    | LayoutType::Interface { .. }
                    | LayoutType::ClosureEnv,
                ) => {
                    if fields.len() != layout.fields.len() {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "struct field count does not match layout field count"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }

                    for (field_id, layout_field) in fields.iter().zip(layout.fields.iter()) {
                        let field = self.tree.get(*field_id);
                        if field.ty != layout_field.ty {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "struct field type does not match layout field type"
                                    .to_string(),
                                anchor: ValidateAnchor::node(type_id),
                            });
                        }
                    }
                }
                (
                    Type::FunctionValue {
                        signature,
                        environment,
                    },
                    LayoutType::FunctionValue,
                ) => {
                    if layout.fields.len() != 2 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout must have exactly two fields"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }

                    if layout.fields[0].ty != *signature || layout.fields[1].ty != *environment {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout field types do not match signature and environment"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }
                }
                (Type::Tuple { elements, .. }, LayoutType::Tuple) => {
                    if elements.len() != layout.fields.len() {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "tuple element count does not match layout field count"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }
                }
                (Type::Array { .. }, LayoutType::Array { .. }) => {}
                _ => {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "type table layout kind mismatch".to_string(),
                        anchor: ValidateAnchor::node(type_id),
                    });
                }
            }
        }

        Ok(())
    }
}
