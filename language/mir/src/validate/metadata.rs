use std::collections::HashSet;

use crate::{
    AddressSpace, Instruction, InterfaceDispatchEntry, ItabEntry, LayoutType,
    ManagedReferenceRepresentation, Mutability, NodeType, ProvenanceId, ProvenanceKind,
    ReferenceKind, Type,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

/// One provenance visitation state for cycle checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ProvenanceVisitState {
    /// The record has not been visited yet.
    Unvisited,
    /// The record is currently being visited.
    Visiting,
    /// The record and its parents are fully validated.
    Done,
}

impl<'a> Validator<'a> {
    /// Validate module-level metadata invariants.
    pub(super) fn validate_metadata(&self) -> ValidateResult<()> {
        let anchor = self.module_anchor();

        // native pointer size
        match self.tree.pointer_bytes() {
            4 | 8 => {}
            pointer_bytes => Err(ValidateError::MetadataInvariantViolation {
                message: format!("unsupported pointer size {pointer_bytes} bytes"),
                anchor,
            })?,
        }

        // managed reference storage
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

        // managed reference alignment
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

        // representation contract
        let managed_layout = self.tree.data_layout.managed_reference_layout;
        match managed_layout.representation {
            ManagedReferenceRepresentation::NativePointer => {
                if managed_layout.bytes != self.tree.data_layout.native_pointer_bytes {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "native-pointer managed references must match native pointer size"
                            .to_string(),
                        anchor,
                    });
                }
            }
            ManagedReferenceRepresentation::CompressedOffset32
            | ManagedReferenceRepresentation::Handle32 => {
                if managed_layout.bytes != 4 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "32 bit managed reference representations must use 4 byte storage"
                            .to_string(),
                        anchor,
                    });
                }
            }
            ManagedReferenceRepresentation::Handle64 => {
                if managed_layout.bytes != 8 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "64 bit managed reference representations must use 8 byte storage"
                            .to_string(),
                        anchor,
                    });
                }
            }
        }

        // per-instruction memory metadata
        for (&instruction_id, accesses) in &self.tree.memory_table.memory_accesses_by_instruction_id
        {
            self.ensure_node_type(
                NodeType::Instruction,
                instruction_id.id,
                ValidateAnchor::node(instruction_id),
            )?;

            let instruction = self.tree.get(instruction_id);
            if !matches!(
                instruction,
                Instruction::Load { .. }
                    | Instruction::Store { .. }
                    | Instruction::Intrinsic { .. }
            ) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "memory metadata attached to non memory instruction".to_string(),
                    anchor: ValidateAnchor::node(instruction_id),
                });
            }

            // access records
            for access in accesses {
                self.validate_memory_access_invariants(
                    access,
                    ValidateAnchor::node(instruction_id),
                )?;
            }
        }

        // reference types
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

            // managed references
            if kind == ReferenceKind::Managed && !address_space.is_generic() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "managed references must use addrspace(generic)".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }

            // constant address space
            if address_space == AddressSpace::Constant && mutability != Mutability::Immutable {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "addrspace(constant) references must be readonly".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }

            // owned references
            if kind == ReferenceKind::Owned && address_space == AddressSpace::Constant {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "owned references cannot use addrspace(constant)".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }
        }

        // vtable ownership
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

        // itab ownership
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

        // interface shapes
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

        // itab contents
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

        // type to layout mappings
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

            // type-specific layout contracts
            let ty = self.tree.get(type_id);
            match (ty, &layout.layout_type) {
                (
                    Type::Struct { fields, .. },
                    LayoutType::Struct
                    | LayoutType::Union { .. }
                    | LayoutType::Interface { .. }
                    | LayoutType::FunctionEnvironment,
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
                (Type::FunctionValue { signature }, LayoutType::FunctionValue) => {
                    if layout.fields.len() != 2 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout must have exactly two fields"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }

                    let environment = self.tree.function_value_environment_type();
                    if layout.fields[0].ty != *signature || layout.fields[1].ty != environment {
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

        // node attachments
        self.validate_node_provenance_attachments()?;

        // provenance graph
        self.validate_provenance_records()?;

        // reverse index
        self.validate_provenance_reverse_index()?;

        Ok(())
    }

    /// Validate node-attached provenance ids.
    fn validate_node_provenance_attachments(&self) -> ValidateResult<()> {
        for (node_id, provenance_id) in self.tree.provenance_by_node_id.iter().enumerate() {
            let Some(provenance_id) = provenance_id else {
                continue;
            };

            if !self.tree.provenance_table.contains(*provenance_id) {
                return Err(self.metadata_error(
                    ValidateAnchor::for_raw_node(self.tree, node_id as u32),
                    "node references a missing provenance record",
                ));
            }
        }

        Ok(())
    }

    /// Validate provenance records.
    fn validate_provenance_records(&self) -> ValidateResult<()> {
        let mut states =
            vec![ProvenanceVisitState::Unvisited; self.tree.provenance_table.records.len()];

        for index in 0..self.tree.provenance_table.records.len() {
            self.validate_provenance_record(ProvenanceId::new(index as u32), &mut states)?;
        }

        Ok(())
    }

    /// Validate one provenance record and its parent chain.
    fn validate_provenance_record(
        &self,
        provenance_id: ProvenanceId,
        states: &mut [ProvenanceVisitState],
    ) -> ValidateResult<()> {
        match states[provenance_id.index()] {
            ProvenanceVisitState::Done => return Ok(()),
            ProvenanceVisitState::Visiting => {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "provenance parent chain must be acyclic".to_string(),
                    anchor: self.module_anchor(),
                });
            }
            ProvenanceVisitState::Unvisited => {}
        }

        states[provenance_id.index()] = ProvenanceVisitState::Visiting;
        let record = self.tree.provenance_table.record(provenance_id);

        // parent records
        for &parent_id in &record.parents {
            if !self.tree.provenance_table.contains(parent_id) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "provenance record references a missing parent".to_string(),
                    anchor: self.module_anchor(),
                });
            }

            self.validate_provenance_record(parent_id, states)?;
        }

        // record contract
        let anchor = self.module_anchor();
        match record.kind {
            ProvenanceKind::Direct => {
                if record.reason.is_some() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "direct provenance must not carry a reason".to_string(),
                        anchor,
                    });
                }

                if record.origins.len() != 1 || !record.parents.is_empty() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "direct provenance must have exactly one origin and no parents"
                            .to_string(),
                        anchor,
                    });
                }
            }
            ProvenanceKind::Synthetic => {
                if !record.origins.is_empty() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "synthetic provenance must not carry direct origins".to_string(),
                        anchor,
                    });
                }
            }
            ProvenanceKind::Derived => {
                if record.parents.is_empty() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "derived provenance must carry at least one parent".to_string(),
                        anchor,
                    });
                }
            }
            ProvenanceKind::Merged => {
                let source_count = record.origins.len() + record.parents.len();
                if source_count < 2 {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "merged provenance must carry at least two sources".to_string(),
                        anchor,
                    });
                }
            }
            ProvenanceKind::Inlined | ProvenanceKind::Optimized => {
                if record.origins.is_empty() && record.parents.is_empty() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "inlined and optimized provenance must carry at least one source"
                            .to_string(),
                        anchor,
                    });
                }
            }
        }

        states[provenance_id.index()] = ProvenanceVisitState::Done;

        Ok(())
    }

    /// Validate the reverse provenance index.
    fn validate_provenance_reverse_index(&self) -> ValidateResult<()> {
        // reverse index entries
        for (&origin, records) in &self.tree.provenance_table.records_by_origin {
            let mut seen = HashSet::new();

            for &provenance_id in records {
                if !seen.insert(provenance_id) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "provenance reverse index must not contain duplicates".to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                if !self.tree.provenance_table.contains(provenance_id) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "provenance reverse index references a missing record".to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                let record = self.tree.provenance_table.record(provenance_id);
                if !record.origins.contains(&origin) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "provenance reverse index does not match record origins"
                            .to_string(),
                        anchor: self.module_anchor(),
                    });
                }
            }
        }

        // forward record entries
        for (index, record) in self.tree.provenance_table.records.iter().enumerate() {
            let provenance_id = ProvenanceId::new(index as u32);

            for &origin in &record.origins {
                let Some(records) = self.tree.provenance_table.records_by_origin.get(&origin)
                else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "provenance record origin is missing from the reverse index"
                            .to_string(),
                        anchor: self.module_anchor(),
                    });
                };

                if !records.contains(&provenance_id) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "provenance record origin is missing from the reverse index"
                            .to_string(),
                        anchor: self.module_anchor(),
                    });
                }
            }
        }

        Ok(())
    }
}
