use std::collections::{HashMap, HashSet};

use crate::{
    AttributeArgs, AttributeIdentifier, AttributeValue, Block, BlockReference, Function,
    FunctionReference, Global, GlobalReference, IntegerReference, Local, LocalNodeId,
    LocalNodeIdAny, LocalReference, NodeTree, NodeType, Type, TypeReference, Value, ValueReference,
};

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
        self.validate_attributes()?;

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

    /// Validate attached attributes.
    fn validate_attributes(&self) -> ValidateResult<()> {
        for (&node_id, attributes) in &self.tree.attributes_by_node_id {
            let anchor = ValidateAnchor::for_raw_node(self.tree, node_id);

            for attribute in attributes {
                self.validate_attribute_name(attribute.name, anchor, "attribute name")?;

                match &attribute.args {
                    AttributeArgs::None => {}
                    AttributeArgs::Value(value) => {
                        self.validate_attribute_value(value, anchor, "attribute value")?;
                    }
                    AttributeArgs::Values(values) => {
                        for value in values {
                            self.validate_attribute_value(value, anchor, "attribute value")?;
                        }
                    }
                    AttributeArgs::KeyValues(pairs) => {
                        for pair in pairs {
                            self.validate_attribute_name(pair.key, anchor, "attribute key")?;
                            self.validate_attribute_value(&pair.value, anchor, "attribute value")?;
                        }
                    }
                }
            }
        }

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

    /// Resolve one value reference or return a validation error.
    pub(super) fn require_value_reference(
        &self,
        value: ValueReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<Value> {
        match value {
            ValueReference::Value(value) => Ok(value),
            ValueReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            ValueReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one type reference or return a validation error.
    pub(super) fn require_type_reference(
        &self,
        ty: TypeReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Type>> {
        match ty {
            TypeReference::Type(ty) => Ok(ty),
            TypeReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            TypeReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one block reference or return a validation error.
    pub(super) fn require_block_reference(
        &self,
        block: BlockReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Block>> {
        match block {
            BlockReference::Block(block) => Ok(block),
            BlockReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            BlockReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one function reference or return a validation error.
    pub(super) fn require_function_reference(
        &self,
        function: FunctionReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Function>> {
        match function {
            FunctionReference::Function(function) => Ok(function),
            FunctionReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            FunctionReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one local reference or return a validation error.
    pub(super) fn require_local_reference(
        &self,
        local: LocalReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Local>> {
        match local {
            LocalReference::Local(local) => Ok(local),
            LocalReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            LocalReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one global reference or return a validation error.
    pub(super) fn require_global_reference(
        &self,
        global: GlobalReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<LocalNodeId<Global>> {
        match global {
            GlobalReference::Global(global) => Ok(global),
            GlobalReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            GlobalReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Resolve one integer reference or return a validation error.
    pub(super) fn require_integer_reference(
        &self,
        value: IntegerReference,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<i64> {
        match value {
            IntegerReference::Integer(value) => Ok(value),
            IntegerReference::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            IntegerReference::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Validate one attribute identifier.
    fn validate_attribute_name(
        &self,
        identifier: AttributeIdentifier,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<()> {
        match identifier {
            AttributeIdentifier::Identifier(_) => Ok(()),
            AttributeIdentifier::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            AttributeIdentifier::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
    }

    /// Validate one attribute value recursively.
    fn validate_attribute_value(
        &self,
        value: &AttributeValue,
        anchor: ValidateAnchor,
        label: &'static str,
    ) -> ValidateResult<()> {
        match value {
            AttributeValue::Identifier(identifier) => {
                self.validate_attribute_name(*identifier, anchor, label)
            }
            AttributeValue::Type(ty) => {
                let ty = self.require_type_reference(*ty, anchor, label)?;
                self.ensure_node_type(NodeType::Type, ty.id, anchor)
            }
            AttributeValue::Integer(value) => {
                let _ = self.require_integer_reference(*value, anchor, label)?;
                Ok(())
            }
            AttributeValue::Float(_) | AttributeValue::Boolean(_) | AttributeValue::String(_) => {
                Ok(())
            }
            AttributeValue::List(values) => {
                for value in values {
                    self.validate_attribute_value(value, anchor, label)?;
                }

                Ok(())
            }
            AttributeValue::Missing => {
                Err(self.metadata_error(anchor, format!("{label} is missing")))
            }
            AttributeValue::Error => {
                Err(self.metadata_error(anchor, format!("{label} is malformed")))
            }
        }
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
            Type::Slice { .. } => "slice",
            Type::Tuple { .. } => "tuple",
            Type::Struct { .. } => "struct",
            Type::Newtype { .. } => "newtype",
            Type::Vector { .. } => "vector",
            Type::Tensor { .. } => "tensor",
            Type::TensorView { .. } => "tensorView",
            Type::FunctionSignature { .. } => "signature",
            Type::FunctionPointer { .. } => "fn",
            Type::Closure { .. } => "callable",
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
