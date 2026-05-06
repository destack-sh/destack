use crate::{
    AddressSpace, Instruction, InterfaceDispatchEntry, ItabEntry, LayoutKind, NodeType,
    ProvenanceAnchor, ProvenanceId, ProvenanceKey, ReferenceKind, Type, TypeReference,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

/// One origin visitation state for cycle checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum OriginVisitState {
    /// The record has not been visited yet.
    Unvisited,
    /// The record is currently being visited.
    Visiting,
    /// The record and its inputs are fully validated.
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

        // per-instruction memory metadata
        for (&instruction_id, accesses) in
            &self.tree.metadata.memory.memory_accesses_by_instruction_id
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
            let (kind, address_space) = match ty {
                Type::Reference {
                    kind,
                    address_space,
                    ..
                }
                | Type::TensorView {
                    kind,
                    address_space,
                    ..
                } => (*kind, address_space.clone()),
                _ => continue,
            };

            // heap references
            if matches!(kind, ReferenceKind::Managed | ReferenceKind::Owned)
                && !matches!(address_space, AddressSpace::Local | AddressSpace::Shared)
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "heap references must use space(local) or space(shared)".to_string(),
                    anchor: ValidateAnchor::node(type_id),
                });
            }
        }

        // interface shapes
        for (&interface_type, shape) in &self.tree.metadata.dispatch.interface_dispatch_shapes {
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
        for itab in self.tree.metadata.dispatch.iter_itabs() {
            let anchor = ValidateAnchor::node(itab.interface);
            let Some(shape) = self
                .tree
                .metadata
                .dispatch
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
        }

        // type to layout mappings
        for (&type_id, &layout_id) in &self.tree.metadata.layout.layout_by_type {
            let Some(layout) = self
                .tree
                .metadata
                .layout
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
            match (ty, &layout.kind) {
                (
                    Type::Struct { fields, .. },
                    LayoutKind::Struct
                    | LayoutKind::Object { .. }
                    | LayoutKind::Union { .. }
                    | LayoutKind::Interface { .. }
                    | LayoutKind::CallableEnvironment,
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
                        if field.ty != TypeReference::Type(layout_field.ty) {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message: "struct field type does not match layout field type"
                                    .to_string(),
                                anchor: ValidateAnchor::node(type_id),
                            });
                        }
                    }
                }
                (Type::Callable { signature }, LayoutKind::Callable) => {
                    if layout.fields.len() != 2 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout must have exactly two fields"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }

                    let TypeReference::Type(signature) = *signature else {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout signature must be concrete".to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    };

                    let environment = self.tree.callable_environment_type();
                    if layout.fields[0].ty != signature || layout.fields[1].ty != environment {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "function value layout field types do not match signature and environment"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }
                }
                (Type::Tuple { elements, .. }, LayoutKind::Tuple) => {
                    if elements.len() != layout.fields.len() {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "tuple element count does not match layout field count"
                                .to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }
                }
                (Type::Slice { .. }, LayoutKind::Slice) => {
                    if layout.fields.len() != 2 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "slice layout must have exactly two fields".to_string(),
                            anchor: ValidateAnchor::node(type_id),
                        });
                    }
                }
                (Type::Array { .. }, LayoutKind::Array { .. }) => {}
                _ => {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "type table layout kind mismatch".to_string(),
                        anchor: ValidateAnchor::node(type_id),
                    });
                }
            }
        }

        // node attachments
        self.validate_node_origin_attachments()?;

        // origin graph
        self.validate_origin_records()?;

        Ok(())
    }

    /// Validate node-attached origin ids.
    fn validate_node_origin_attachments(&self) -> ValidateResult<()> {
        for (node_id, origin_id) in self
            .tree
            .metadata
            .provenance
            .provenance_by_node_id
            .iter()
            .enumerate()
        {
            let Some(origin_id) = origin_id else {
                continue;
            };

            if !self.tree.metadata.provenance.contains(*origin_id) {
                return Err(self.metadata_error(
                    ValidateAnchor::for_raw_node(self.tree, node_id as u32),
                    "node references a missing origin record",
                ));
            }
        }

        Ok(())
    }

    /// Validate origin records.
    fn validate_origin_records(&self) -> ValidateResult<()> {
        let mut states =
            vec![OriginVisitState::Unvisited; self.tree.metadata.provenance.record_by_id.len()];

        for index in 0..self.tree.metadata.provenance.record_by_id.len() {
            self.validate_origin_record(ProvenanceId::new(index as u32), &mut states)?;
        }

        Ok(())
    }

    /// Validate one origin record and its MIR input chain.
    fn validate_origin_record(
        &self,
        origin_id: ProvenanceId,
        states: &mut [OriginVisitState],
    ) -> ValidateResult<()> {
        match states[origin_id.index()] {
            OriginVisitState::Done => return Ok(()),
            OriginVisitState::Visiting => {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "origin graph must be acyclic".to_string(),
                    anchor: self.module_anchor(),
                });
            }
            OriginVisitState::Unvisited => {}
        }

        states[origin_id.index()] = OriginVisitState::Visiting;
        let record = self.tree.metadata.provenance.record(origin_id);

        // primary MIR anchor
        if let ProvenanceAnchor::Mir(parent_id) = record.anchor {
            if !self.tree.metadata.provenance.contains(parent_id) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "origin record references a missing MIR anchor".to_string(),
                    anchor: self.module_anchor(),
                });
            }

            self.validate_origin_record(parent_id, states)?;
        }

        // input MIR anchors
        for input in &record.contributors {
            let ProvenanceKey::Mir(parent_id) = *input else {
                continue;
            };

            if !self.tree.metadata.provenance.contains(parent_id) {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "origin record references a missing MIR input".to_string(),
                    anchor: self.module_anchor(),
                });
            }

            self.validate_origin_record(parent_id, states)?;
        }

        states[origin_id.index()] = OriginVisitState::Done;

        Ok(())
    }
}
