use std::collections::{HashMap, HashSet};

use crate::{Block, Function, LocalNodeId, LocalNodeIdAny, NodeTree, NodeType, Type, Value};

use super::{ValidateAnchor, ValidateError, ValidateResult};

/// Validates MIR invariants for a tree or function.
#[derive(Debug)]
pub struct Validator<'a> {
    /// The MIR tree to validate.
    pub(super) tree: &'a NodeTree,
}

#[allow(clippy::type_complexity)]
impl<'a> Validator<'a> {
    /// Create a new validator for a tree.
    pub fn new(tree: &'a NodeTree) -> Self {
        Self { tree }
    }

    /// Validate a full MIR module.
    pub fn validate(&self) -> ValidateResult<()> {
        // raw tree structure
        self.validate_structure()?;

        // function bodies
        for (global_id, node_type) in self.tree.node_type_by_node_id.iter().enumerate() {
            if *node_type != NodeType::Function {
                continue;
            }

            let function_id = LocalNodeId::<Function>::new(global_id as u32);
            self.validate_function(function_id)?;
        }

        // metadata domains
        self.validate_metadata()?;
        self.validate_dispatch()?;
        self.validate_debug()?;

        Ok(())
    }

    /// Validate the raw node and side-table structure.
    fn validate_structure(&self) -> ValidateResult<()> {
        let node_count = self.tree.node_type_by_node_id.len();
        let anchor = self.module_anchor();

        if self.tree.next_global_id as usize != node_count {
            return Err(
                self.metadata_error(anchor, "next_global_id does not match node table length")
            );
        }

        if self.tree.local_id_by_node_id.len() != node_count {
            return Err(self.metadata_error(
                anchor,
                "local_id_by_node_id length does not match node table length",
            ));
        }

        if self.tree.metadata.provenance.provenance_by_node_id.len() != node_count {
            return Err(self.metadata_error(
                anchor,
                "provenance_by_node_id length does not match node table length",
            ));
        }

        for &node_id in self.tree.attributes_by_node_id.keys() {
            if node_id as usize >= node_count {
                return Err(
                    self.metadata_error(anchor, "attributes reference an out of bounds node id")
                );
            }
        }

        Ok(())
    }

    /// Return one generic module anchor for metadata validation.
    pub(super) fn module_anchor(&self) -> ValidateAnchor {
        self.tree
            .node_type_by_node_id
            .first()
            .copied()
            .map(|ty| ValidateAnchor {
                node: LocalNodeIdAny::new(0, ty),
            })
            .unwrap_or(ValidateAnchor {
                node: LocalNodeIdAny::new(0, NodeType::Type),
            })
    }

    /// Build one metadata invariant violation.
    pub(super) fn metadata_error(
        &self,
        anchor: ValidateAnchor,
        message: impl Into<String>,
    ) -> ValidateError {
        ValidateError::MetadataInvariantViolation {
            message: message.into(),
            anchor,
        }
    }

    /// Ensure a value is defined within the current function.
    pub(super) fn ensure_defined(
        &self,
        value: Value,
        anchor: ValidateAnchor,
        defined_values: &HashSet<Value>,
    ) -> ValidateResult<()> {
        if !defined_values.contains(&value) {
            return Err(ValidateError::UseOfUndefinedValue { value, anchor });
        }

        Ok(())
    }

    /// Ensure a node id points at the expected node type.
    pub(super) fn ensure_node_type(
        &self,
        expected: NodeType,
        node_id: u32,
        anchor: ValidateAnchor,
    ) -> ValidateResult<()> {
        let found = self
            .tree
            .node_type_by_node_id
            .get(node_id as usize)
            .copied();

        if found != Some(expected) {
            return Err(ValidateError::InvalidNodeReference {
                expected,
                found,
                node_id,
                anchor,
            });
        }

        Ok(())
    }

    /// Return a short type kind name.
    pub(super) fn type_kind(&self, ty: &Type) -> &'static str {
        // map types to short labels
        match ty {
            Type::Void => "void",
            Type::Boolean => "boolean",
            Type::Int { .. } => "int",
            Type::Isize => "isize",
            Type::Usize => "usize",
            Type::Float { .. } => "float",
            Type::TypeDescriptor => "typeDescriptor",
            Type::TypeId => "typeId",
            Type::Reference { .. } => "ref",
            Type::Array { .. } => "array",
            Type::Tuple { .. } => "tuple",
            Type::Struct { .. } => "struct",
            Type::Newtype { .. } => "newtype",
            Type::Vector { .. } => "vector",
            Type::Tensor { .. } => "tensor",
            Type::TensorReference { .. } => "tensorRef",
            Type::FunctionPointer { .. } => "fn",
            Type::Closure { .. } => "closure",
        }
    }

    /// Format a block label using function order.
    pub(super) fn block_label(
        &self,
        block_id: LocalNodeId<Block>,
        block_order: &HashMap<LocalNodeId<Block>, usize>,
    ) -> String {
        // format block labels from the function order
        if let Some(index) = block_order.get(&block_id) {
            return format!("block{index}");
        }

        format!("block{}", block_id.id)
    }
}
