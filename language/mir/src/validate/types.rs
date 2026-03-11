use crate::{InterfaceDispatchEntry, ItabEntry, NodeType, Type};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Validate reference legality and address space contracts.
    pub(super) fn validate_type_legality(&self) -> ValidateResult<()> {
        for (type_id, ty) in self.tree.iter_nodes::<Type>() {
            let (kind, address_space, mutability) = match ty {
                Type::Reference {
                    kind,
                    address_space,
                    mutability,
                    ..
                }
                | Type::TensorReference {
                    kind,
                    address_space,
                    mutability,
                    ..
                } => (*kind, *address_space, *mutability),
                _ => continue,
            };

            if kind == crate::ReferenceKind::Managed && !address_space.is_generic() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "managed references must use addrspace(generic)".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }

            if address_space == crate::AddressSpace::Constant
                && mutability != crate::Mutability::Immutable
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "addrspace(constant) references must be readonly".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }

            if kind == crate::ReferenceKind::Owned && address_space == crate::AddressSpace::Constant
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "owned references cannot use addrspace(constant)".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }
        }

        Ok(())
    }

    /// Validate type table dispatch contracts.
    pub(super) fn validate_type_dispatch_metadata(&self) -> ValidateResult<()> {
        for (&type_id, &vtable_id) in &self.tree.type_table.vtable_by_type {
            let Some(lineage) = self.tree.type_table.lineage(type_id) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "type table vtable mapping is missing lineage".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            };
            if lineage.is_interface {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface type cannot carry a vtable".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }

            let Some(vtable) = self.tree.dispatch_table.vtables.get(vtable_id.index()) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "type table references missing vtable metadata".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            };
            if vtable.ty != type_id {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "type table vtable owner mismatch".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }
        }

        for (&type_id, itabs) in &self.tree.type_table.itabs_by_type {
            for (&interface_type, &itab_id) in itabs {
                let Some(itab) = self.tree.dispatch_table.itabs.get(itab_id.index()) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "type table references missing itab metadata".to_string(),
                        anchor: ValidateAnchor::node(type_id),
                    });
                };
                if itab.concrete != type_id || itab.interface != interface_type {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "type table itab mapping mismatch".to_string(),
                        anchor: ValidateAnchor::node(type_id),
                    });
                }
            }
        }

        for (&interface_type, shape) in &self.tree.dispatch_table.interface_dispatch_shapes {
            if shape.interface != interface_type {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch shape key mismatch".to_string(),
                    anchor: ValidateAnchor::node(interface_type),
                });
            }

            if !matches!(
                shape.entries.first(),
                Some(InterfaceDispatchEntry::TypeDescriptor)
            ) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "interface dispatch shape missing type descriptor prefix".to_string(),
                    anchor: ValidateAnchor::node(interface_type),
                });
            }
        }

        for (itab_id, itab) in self.tree.dispatch_table.iter_itabs() {
            let anchor = ValidateAnchor::node(itab.interface);
            let Some(shape) = self
                .tree
                .dispatch_table
                .interface_dispatch_shape(itab.interface)
            else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "itab missing interface dispatch shape".to_string(),
                    anchor,
                });
            };

            if itab.entries.len() != shape.entries.len() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "itab entry count does not match interface dispatch shape".to_string(),
                    anchor,
                });
            }

            for (shape_entry, itab_entry) in shape.entries.iter().zip(itab.entries.iter()) {
                match (shape_entry, itab_entry) {
                    (InterfaceDispatchEntry::TypeDescriptor, ItabEntry::TypeDescriptor) => {}
                    (
                        InterfaceDispatchEntry::FieldOffset {
                            field: expected_field,
                            field_name: expected,
                        },
                        ItabEntry::FieldOffset {
                            field: actual_field,
                            field_name: actual,
                            offset: _,
                        },
                    ) if expected_field == actual_field && expected == actual => {
                        self.ensure_node_type(NodeType::Field, expected_field.id, anchor)?;
                    }
                    (
                        InterfaceDispatchEntry::Method {
                            declared_method: expected,
                        },
                        ItabEntry::Method {
                            declared_method: actual,
                            ..
                        },
                    ) if expected == actual => {}
                    _ => {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "itab entry does not match interface dispatch shape"
                                .to_string(),
                            anchor,
                        });
                    }
                }
            }

            let Some(mapped_itab_id) = self.tree.type_table.itab_id(itab.concrete, itab.interface)
            else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "itab missing concrete type interface mapping".to_string(),
                    anchor: ValidateAnchor::node(itab.concrete),
                });
            };

            if mapped_itab_id != itab_id {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "itab concrete type interface mapping id mismatch".to_string(),
                    anchor: ValidateAnchor::node(itab.concrete),
                });
            }
        }

        Ok(())
    }
}
