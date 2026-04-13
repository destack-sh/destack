use std::collections::HashMap;

use crate::{
    Block, DebugInlineSiteId, DebugLocation, DebugRangeStart, DebugScopeId, DebugScopeKind,
    DebugValueLocation, Function, Instruction, Local, LocalNodeId, NodeType, ProvenanceId, Type,
};

use super::{ValidateAnchor, ValidateError, ValidateResult, Validator};

/// One instruction position within one owning function.
#[derive(Clone, Copy)]
pub(super) struct InstructionPosition {
    /// The owning function.
    pub(super) function_id: LocalNodeId<Function>,
    /// The source order ordinal within the function.
    pub(super) ordinal: usize,
}

impl<'a> Validator<'a> {
    /// Validate debug metadata.
    pub(super) fn validate_debug(&self) -> ValidateResult<()> {
        let scope_functions = self.resolve_debug_scope_functions()?;

        // type references
        for debug_type in &self.tree.metadata.debug.types {
            self.ensure_node_type(NodeType::Type, debug_type.ty.id, self.module_anchor())?;
            self.validate_debug_provenance(debug_type.provenance, self.module_anchor())?;
        }

        // scopes
        for index in 0..self.tree.metadata.debug.scopes.len() {
            let scope_id = DebugScopeId::new(index as u32);
            let scope = self.tree.metadata.debug.scope(scope_id);
            self.validate_debug_provenance(scope.provenance, self.module_anchor())?;

            if let Some(parent_id) = scope.parent
                && parent_id.index() >= self.tree.metadata.debug.scopes.len()
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug scope references a missing parent scope".to_string(),
                    anchor: self.module_anchor(),
                });
            }
        }

        // inline sites
        let mut validated_inline_sites = vec![false; self.tree.metadata.debug.inline_sites.len()];
        for index in 0..self.tree.metadata.debug.inline_sites.len() {
            let inline_site_id = DebugInlineSiteId::new(index as u32);
            if validated_inline_sites[inline_site_id.index()] {
                continue;
            }

            let mut inline_site_path = Vec::<DebugInlineSiteId>::new();
            let mut current_inline_site = Some(inline_site_id);

            while let Some(inline_site_id) = current_inline_site {
                if validated_inline_sites[inline_site_id.index()] {
                    break;
                }

                if inline_site_path.contains(&inline_site_id) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug inline site parent chain must be acyclic".to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                inline_site_path.push(inline_site_id);

                let inline_site = self.tree.metadata.debug.inline_site(inline_site_id);
                self.validate_debug_provenance(inline_site.provenance, self.module_anchor())?;
                let Some(callee_scope) = self
                    .tree
                    .metadata
                    .debug
                    .scopes
                    .get(inline_site.callee_scope.index())
                else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug inline site references a missing callee scope".to_string(),
                        anchor: self.module_anchor(),
                    });
                };

                if callee_scope.kind != DebugScopeKind::Function {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug inline site callee scope must be a function scope"
                            .to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                self.validate_debug_location(&inline_site.call_location, None, &scope_functions)?;

                if inline_site.call_location.inline_site != inline_site.parent {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message:
                            "debug inline site call location must point at the parent inline site"
                                .to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                current_inline_site = match inline_site.parent {
                    Some(parent_inline_site_id) => {
                        if parent_inline_site_id.index()
                            >= self.tree.metadata.debug.inline_sites.len()
                        {
                            return Err(ValidateError::MetadataInvariantViolation {
                                message:
                                    "debug inline site references a missing parent inline site"
                                        .to_string(),
                                anchor: self.module_anchor(),
                            });
                        }

                        Some(parent_inline_site_id)
                    }
                    None => None,
                };
            }

            for inline_site_id in inline_site_path {
                validated_inline_sites[inline_site_id.index()] = true;
            }
        }

        // function-owned nodes
        let mut block_functions = HashMap::<LocalNodeId<Block>, LocalNodeId<Function>>::new();
        let mut local_functions = HashMap::<LocalNodeId<Local>, LocalNodeId<Function>>::new();
        let mut instruction_positions =
            HashMap::<LocalNodeId<Instruction>, InstructionPosition>::new();

        for (function_id, function) in self.tree.iter_nodes::<Function>() {
            // locals
            for &local_id in &function.locals {
                if local_functions.insert(local_id, function_id).is_some() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug validation found one local owned by multiple functions"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }
            }

            // blocks and instructions
            let mut ordinal = 0usize;
            for &block_id in &function.blocks {
                if block_functions.insert(block_id, function_id).is_some() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug validation found one block owned by multiple functions"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                let block = self.tree.get(block_id);
                for &instruction_id in &block.instructions {
                    let position = InstructionPosition {
                        function_id,
                        ordinal,
                    };

                    if instruction_positions
                        .insert(instruction_id, position)
                        .is_some()
                    {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message:
                                "debug validation found one instruction owned by multiple functions"
                                    .to_string(),
                            anchor: ValidateAnchor::node(function_id),
                        });
                    }

                    ordinal += 1;
                }
            }
        }

        // block scopes
        for (&block_id, &scope_id) in &self.tree.metadata.debug.block_scopes {
            self.ensure_node_type(NodeType::Block, block_id.id, ValidateAnchor::node(block_id))?;

            let Some(&function_id) = block_functions.get(&block_id) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug block scope references a block outside every function"
                        .to_string(),
                    anchor: ValidateAnchor::node(block_id),
                });
            };

            if scope_functions[scope_id.index()] != function_id {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug block scope must belong to the owning function".to_string(),
                    anchor: ValidateAnchor::node(block_id),
                });
            }
        }

        // instruction locations
        for (&instruction_id, location) in &self.tree.metadata.debug.instruction_locations {
            self.ensure_node_type(
                NodeType::Instruction,
                instruction_id.id,
                ValidateAnchor::node(instruction_id),
            )?;

            let Some(position) = instruction_positions.get(&instruction_id) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug instruction location references an instruction outside every function"
                        .to_string(),
                    anchor: ValidateAnchor::node(instruction_id),
                });
            };

            self.validate_debug_location(location, Some(position.function_id), &scope_functions)?;
        }

        // bindings
        for binding in &self.tree.metadata.debug.bindings {
            self.ensure_node_type(NodeType::Type, binding.ty.id, self.module_anchor())?;
            self.validate_debug_provenance(binding.provenance, self.module_anchor())?;

            if binding.scope.index() >= scope_functions.len() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug binding scope references a missing scope".to_string(),
                    anchor: self.module_anchor(),
                });
            }
        }

        for (&binding_id, ranges) in &self.tree.metadata.debug.binding_location_ranges {
            if binding_id.index() >= self.tree.metadata.debug.bindings.len() {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug binding range references a missing binding".to_string(),
                    anchor: self.module_anchor(),
                });
            }

            let binding = self.tree.metadata.debug.binding(binding_id);
            let function_id = scope_functions[binding.scope.index()];
            let mut previous_end = 0usize;

            for range in ranges {
                if range.binding != binding_id {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug binding range key must match the embedded binding id"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                self.validate_debug_value_location(
                    &range.location,
                    function_id,
                    binding.ty,
                    &local_functions,
                    &instruction_positions,
                )?;

                let start = self.debug_range_start_ordinal(
                    range.start,
                    function_id,
                    &instruction_positions,
                    ValidateAnchor::node(function_id),
                )?;
                let end = self.debug_range_end_ordinal(
                    range.end,
                    function_id,
                    &instruction_positions,
                    ValidateAnchor::node(function_id),
                )?;

                if let Some(end) = end
                    && start >= end
                {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug binding ranges must be non-empty and ordered".to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                if start < previous_end {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug binding ranges must be sorted and non-overlapping"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                previous_end = end.unwrap_or(usize::MAX);
            }
        }

        // coroutine states
        for coroutine_state in &self.tree.metadata.debug.coroutine_states {
            let function_id = scope_functions[coroutine_state.scope.index()];
            self.validate_debug_provenance(
                coroutine_state.provenance,
                ValidateAnchor::node(function_id),
            )?;

            self.validate_debug_location(
                &coroutine_state.suspend_location,
                Some(function_id),
                &scope_functions,
            )?;

            for &binding_id in &coroutine_state.lifted_bindings {
                let Some(binding) = self.tree.metadata.debug.bindings.get(binding_id.index())
                else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug coroutine state references a missing binding".to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                };

                if scope_functions[binding.scope.index()] != function_id {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug coroutine state bindings must belong to the same function"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }
            }
        }

        Ok(())
    }

    /// Resolve the owning function for every debug scope.
    fn resolve_debug_scope_functions(&self) -> ValidateResult<Vec<LocalNodeId<Function>>> {
        let mut root_functions = vec![None; self.tree.metadata.debug.scopes.len()];

        for (&function_id, &scope_id) in &self.tree.metadata.debug.function_scopes {
            self.ensure_node_type(
                NodeType::Function,
                function_id.id,
                ValidateAnchor::node(function_id),
            )?;

            let Some(scope) = self.tree.metadata.debug.scopes.get(scope_id.index()) else {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "function debug scope references a missing scope".to_string(),
                    anchor: ValidateAnchor::node(function_id),
                });
            };

            if scope.kind != DebugScopeKind::Function {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "function debug scope must use DebugScopeKind::Function".to_string(),
                    anchor: ValidateAnchor::node(function_id),
                });
            }

            if root_functions[scope_id.index()]
                .replace(function_id)
                .is_some()
            {
                return Err(ValidateError::MetadataInvariantViolation {
                    message: "debug function scopes must be owned by exactly one function"
                        .to_string(),
                    anchor: ValidateAnchor::node(function_id),
                });
            }
        }

        let mut scope_functions = vec![None; self.tree.metadata.debug.scopes.len()];
        for index in 0..self.tree.metadata.debug.scopes.len() {
            let mut scope_path = Vec::<DebugScopeId>::new();
            let mut current_scope = DebugScopeId::new(index as u32);

            loop {
                if scope_path.contains(&current_scope) {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug scope parent chain must be acyclic".to_string(),
                        anchor: self.module_anchor(),
                    });
                }

                if let Some(function_id) =
                    scope_functions[current_scope.index()].or(root_functions[current_scope.index()])
                {
                    scope_functions[current_scope.index()] = Some(function_id);

                    for &scope_id in &scope_path {
                        scope_functions[scope_id.index()] = Some(function_id);
                    }

                    break;
                }

                scope_path.push(current_scope);

                let scope = self.tree.metadata.debug.scope(current_scope);
                let Some(parent_id) = scope.parent else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug lexical scopes must be rooted in a function scope"
                            .to_string(),
                        anchor: self.module_anchor(),
                    });
                };

                current_scope = parent_id;
            }
        }

        Ok(scope_functions.into_iter().map(Option::unwrap).collect())
    }

    /// Validate one debug location.
    fn validate_debug_location(
        &self,
        location: &DebugLocation,
        expected_function: Option<LocalNodeId<Function>>,
        scope_functions: &[LocalNodeId<Function>],
    ) -> ValidateResult<()> {
        self.validate_debug_provenance(
            location.provenance,
            expected_function
                .map(ValidateAnchor::node)
                .unwrap_or_else(|| self.module_anchor()),
        )?;

        let function_id = scope_functions[location.scope.index()];

        if let Some(expected_function) = expected_function
            && function_id != expected_function
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "debug location must belong to the owning function".to_string(),
                anchor: ValidateAnchor::node(expected_function),
            });
        }

        if let Some(inline_site_id) = location.inline_site
            && inline_site_id.index() >= self.tree.metadata.debug.inline_sites.len()
        {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "debug location references a missing inline site".to_string(),
                anchor: expected_function
                    .map(ValidateAnchor::node)
                    .unwrap_or_else(|| self.module_anchor()),
            });
        }

        Ok(())
    }

    /// Validate one optional provenance link on a debug record.
    fn validate_debug_provenance(
        &self,
        provenance_id: Option<ProvenanceId>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let Some(provenance_id) = provenance_id else {
            return Ok(());
        };

        if self.tree.metadata.provenance.contains(provenance_id) {
            return Ok(());
        }

        Err(ValidateError::MetadataInvariantViolation {
            message: "debug metadata references a missing provenance record".to_string(),
            anchor,
        })
    }

    /// Validate one debug value location against one function and binding type.
    pub(super) fn validate_debug_value_location(
        &self,
        location: &DebugValueLocation,
        function_id: LocalNodeId<Function>,
        binding_type: LocalNodeId<Type>,
        local_functions: &HashMap<LocalNodeId<Local>, LocalNodeId<Function>>,
        _instruction_positions: &HashMap<LocalNodeId<Instruction>, InstructionPosition>,
    ) -> ValidateResult<()> {
        match location {
            DebugValueLocation::Value(value) => {
                let function = self.tree.get(function_id);
                let Some(value_type) = function.value_type(*value) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location references an undefined SSA value"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                };

                if value_type != binding_type {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location type does not match binding type"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }
            }
            DebugValueLocation::Local(local_id) => {
                self.ensure_node_type(
                    NodeType::Local,
                    local_id.id,
                    ValidateAnchor::node(function_id),
                )?;

                let Some(&owner) = local_functions.get(local_id) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location references a local outside every function"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                };

                if owner != function_id {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location local must belong to the owning function"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                if self.tree.get(*local_id).ty != binding_type.into() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location type does not match binding type"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }
            }
            DebugValueLocation::Global(global_id) => {
                self.ensure_node_type(
                    NodeType::Global,
                    global_id.id,
                    ValidateAnchor::node(function_id),
                )?;

                if self.tree.get(*global_id).ty != binding_type.into() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug value location type does not match binding type"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }
            }
            DebugValueLocation::Constant(_) => {}
            DebugValueLocation::Composite(fragments) => {
                // fragments
                if fragments.is_empty() {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug composite locations must contain at least one fragment"
                            .to_string(),
                        anchor: ValidateAnchor::node(function_id),
                    });
                }

                let mut next_offset = 0u32;
                for fragment in fragments {
                    if fragment.size_bytes == 0 {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "debug value fragments must have non-zero size".to_string(),
                            anchor: ValidateAnchor::node(function_id),
                        });
                    }

                    if fragment.offset_bytes < next_offset {
                        return Err(ValidateError::MetadataInvariantViolation {
                            message: "debug value fragments must be sorted and non-overlapping"
                                .to_string(),
                            anchor: ValidateAnchor::node(function_id),
                        });
                    }

                    self.validate_debug_value_location(
                        fragment.location.as_ref(),
                        function_id,
                        binding_type,
                        local_functions,
                        _instruction_positions,
                    )?;
                    next_offset = fragment.offset_bytes.saturating_add(fragment.size_bytes);
                }
            }
            DebugValueLocation::State(_) => {}
        }

        Ok(())
    }

    /// Return the ordinal for one debug range start.
    pub(super) fn debug_range_start_ordinal(
        &self,
        start: DebugRangeStart,
        function_id: LocalNodeId<Function>,
        instruction_positions: &HashMap<LocalNodeId<Instruction>, InstructionPosition>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<usize> {
        match start {
            DebugRangeStart::FunctionEntry => Ok(0),
            DebugRangeStart::Instruction(instruction_id) => {
                self.ensure_node_type(NodeType::Instruction, instruction_id.id, anchor)?;

                let Some(position) = instruction_positions.get(&instruction_id) else {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug range start instruction is not owned by any function"
                            .to_string(),
                        anchor,
                    });
                };

                if position.function_id != function_id {
                    return Err(ValidateError::MetadataInvariantViolation {
                        message: "debug range start instruction must belong to the owning function"
                            .to_string(),
                        anchor,
                    });
                }

                Ok(position.ordinal + 1)
            }
        }
    }

    /// Return the ordinal for one debug range end.
    pub(super) fn debug_range_end_ordinal(
        &self,
        end: Option<LocalNodeId<Instruction>>,
        function_id: LocalNodeId<Function>,
        instruction_positions: &HashMap<LocalNodeId<Instruction>, InstructionPosition>,
        anchor: ValidateAnchor,
    ) -> ValidateResult<Option<usize>> {
        let Some(end) = end else {
            return Ok(None);
        };

        self.ensure_node_type(NodeType::Instruction, end.id, anchor)?;

        let Some(position) = instruction_positions.get(&end) else {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "debug range end instruction is not owned by any function".to_string(),
                anchor,
            });
        };

        if position.function_id != function_id {
            return Err(ValidateError::MetadataInvariantViolation {
                message: "debug range end instruction must belong to the owning function"
                    .to_string(),
                anchor,
            });
        }

        Ok(Some(position.ordinal + 1))
    }
}
