use std::collections::HashMap;

use crate::{
    Block, CallMetadata, Function, Instruction, LocalNodeId, NodeTree, Type, TypedValue, Value,
};

use crate::parse::{ParseError, ParseResult};

/// Helper for inferring call.indirect metadata.
pub(super) struct IndirectCallInference<'a> {
    /// The node tree being updated.
    tree: &'a mut NodeTree,
    /// Map from values to their concrete types.
    value_types: HashMap<Value, LocalNodeId<Type>>,
    /// Map from pointer values to their pointee types.
    pointer_pointee_types: HashMap<Value, LocalNodeId<Type>>,
}

impl<'a> IndirectCallInference<'a> {
    /// Create inference state seeded from function and block parameters.
    pub(super) fn new(
        tree: &'a mut NodeTree,
        parameters: &[TypedValue],
        blocks: &[LocalNodeId<Block>],
    ) -> Self {
        // initialize type maps
        let mut inference = Self {
            tree,
            value_types: HashMap::new(),
            pointer_pointee_types: HashMap::new(),
        };

        // seed value types from function parameters
        for param in parameters {
            inference.register_value_type(param.value, param.ty);
        }

        // seed value types from block parameters
        for &block_id in blocks {
            let parameters = inference.tree.get(block_id).parameters.clone();
            for param in parameters {
                inference.register_value_type(param.value, param.ty);
            }
        }

        inference
    }

    /// Walk instructions in the provided blocks and infer call metadata.
    pub(super) fn infer_for_blocks(
        &mut self,
        blocks: &[LocalNodeId<Block>],
        fallback_position: usize,
    ) -> ParseResult<()> {
        // walk instructions to infer indirect call signatures
        for &block_id in blocks {
            let instructions = self.tree.get(block_id).instructions.clone();
            for instruction_id in instructions {
                let position = self
                    .tree
                    .get_span(instruction_id)
                    .map(|span| span.start as usize)
                    .unwrap_or(fallback_position);
                let instruction = self.tree.get(instruction_id).clone();

                self.handle_instruction(instruction_id, instruction, position)?;
            }
        }

        Ok(())
    }

    /// Record a value type and any associated pointee type.
    pub(super) fn register_value_type(&mut self, value: Value, ty_id: LocalNodeId<Type>) {
        // record the explicit value type
        self.value_types.insert(value, ty_id);

        // track pointee types for reference values
        if let Type::Reference { pointee, .. } = self.tree.get(ty_id) {
            self.pointer_pointee_types.insert(value, *pointee);
        }
    }

    /// Return the pointee type for a value when it is known.
    pub(super) fn pointee_type_for_value(&self, value: Value) -> Option<LocalNodeId<Type>> {
        // derive pointee from explicit reference types
        if let Some(&type_id) = self.value_types.get(&value)
            && let Type::Reference { pointee, .. } = self.tree.get(type_id)
        {
            return Some(*pointee);
        }

        // fall back to tracked pointer pointee types
        self.pointer_pointee_types.get(&value).copied()
    }

    /// Update inference state for a single instruction.
    pub(super) fn handle_instruction(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: Instruction,
        position: usize,
    ) -> ParseResult<()> {
        // track type producers so call indirect can be inferred
        match &instruction {
            Instruction::Cast {
                destination,
                to_type,
                ..
            } => {
                self.register_value_type(*destination, *to_type);
            }
            Instruction::Struct {
                destination, ty, ..
            }
            | Instruction::Tuple {
                destination, ty, ..
            }
            | Instruction::Array {
                destination, ty, ..
            } => {
                self.register_value_type(*destination, *ty);
            }
            Instruction::LocalGet { destination, local } => {
                let local_decl = self.tree.get(*local);
                self.register_value_type(*destination, local_decl.ty);
            }
            Instruction::GlobalConst {
                destination,
                global,
            } => {
                let global_decl = self.tree.get(*global);
                self.register_value_type(*destination, global_decl.ty);
            }
            Instruction::GlobalAddr {
                destination,
                global,
            } => {
                let global_decl = self.tree.get(*global);
                self.pointer_pointee_types
                    .insert(*destination, global_decl.ty);
            }
            Instruction::ManagedAlloc {
                destination,
                layout,
            }
            | Instruction::RawAlloc {
                destination,
                layout,
            }
            | Instruction::StackAlloc {
                destination,
                layout,
            } => {
                self.pointer_pointee_types.insert(*destination, *layout);
            }
            Instruction::ManagedAllocArray {
                destination,
                element,
                ..
            } => {
                self.pointer_pointee_types.insert(*destination, *element);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                self.track_field_get(*destination, *aggregate, *index);
            }
            Instruction::ElementGet {
                destination, array, ..
            } => {
                self.track_element_get(*destination, *array);
            }
            Instruction::FieldAddr {
                destination,
                aggregate,
                index,
            } => {
                self.track_field_addr(*destination, *aggregate, *index);
            }
            Instruction::ElementAddr {
                destination, array, ..
            } => {
                self.track_element_addr(*destination, *array);
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                ..
            }
            | Instruction::ElementSet {
                destination,
                array: aggregate,
                ..
            } => {
                self.track_element_set(*destination, *aggregate);
            }
            Instruction::Load {
                destination,
                pointer,
            } => {
                self.track_load(*destination, *pointer);
            }
            Instruction::Call {
                destination,
                function,
                ..
            } => {
                self.track_call_destination(*destination, *function);
            }
            Instruction::CallIndirect { .. } => {
                self.infer_call_indirect(instruction_id, &instruction, position)?;
            }
            _ => {}
        }

        Ok(())
    }

    /// Propagate the result type of a field get.
    fn track_field_get(&mut self, destination: Value, aggregate: Value, index: u32) {
        // resolve the aggregate type
        let Some(aggregate_type) = self.value_types.get(&aggregate).copied() else {
            return;
        };

        // unwrap references so we can inspect the aggregate layout
        let base_type = match self.tree.get(aggregate_type) {
            Type::Reference { pointee, .. } => Some(*pointee),
            _ => Some(aggregate_type),
        };

        let Some(base_type) = base_type else {
            return;
        };

        // map the field to its type
        match self.tree.get(base_type) {
            Type::Struct { fields, .. } => {
                if let Some(&field_id) = fields.get(index as usize) {
                    let field = self.tree.get(field_id);
                    self.register_value_type(destination, field.ty);
                }
            }
            Type::Tuple { elements, .. } => {
                if let Some(&element_type) = elements.get(index as usize) {
                    self.register_value_type(destination, element_type);
                }
            }
            _ => {}
        }
    }

    /// Propagate the result type of an element get.
    fn track_element_get(&mut self, destination: Value, array: Value) {
        // resolve the array type
        let Some(array_type) = self.value_types.get(&array).copied() else {
            return;
        };

        // look through references for the array element type
        let element_type = match self.tree.get(array_type) {
            Type::Array { element, .. } => Some(*element),
            Type::Reference { pointee, .. } => {
                let pointee = self.tree.get(*pointee);
                if let Type::Array { element, .. } = pointee {
                    Some(*element)
                } else {
                    None
                }
            }
            _ => None,
        };

        // record the element type
        if let Some(element_type) = element_type {
            self.register_value_type(destination, element_type);
        }
    }

    /// Record the pointee type produced by a field address.
    fn track_field_addr(&mut self, destination: Value, aggregate: Value, index: u32) {
        // resolve pointee type from the aggregate
        let Some(pointee) = self.pointee_type_for_value(aggregate) else {
            return;
        };

        // resolve the field type
        let field_type = match self.tree.get(pointee) {
            Type::Struct { fields, .. } => fields
                .get(index as usize)
                .map(|field_id| self.tree.get(*field_id).ty),
            Type::Tuple { elements, .. } => elements.get(index as usize).copied(),
            _ => None,
        };

        // record the field pointee type
        if let Some(field_type) = field_type {
            self.pointer_pointee_types.insert(destination, field_type);
        }
    }

    /// Record the pointee type produced by an element address.
    fn track_element_addr(&mut self, destination: Value, array: Value) {
        // resolve pointee type from the array
        let Some(pointee) = self.pointee_type_for_value(array) else {
            return;
        };

        // resolve the element type for array references
        let element_type = match self.tree.get(pointee) {
            Type::Array { element, .. } => *element,
            _ => pointee,
        };

        // record the element pointee type
        self.pointer_pointee_types.insert(destination, element_type);
    }

    /// Propagate aggregate types through set instructions.
    fn track_element_set(&mut self, destination: Value, aggregate: Value) {
        // propagate the aggregate type when it is known
        if let Some(&type_id) = self.value_types.get(&aggregate) {
            self.register_value_type(destination, type_id);
        }
    }

    /// Propagate the loaded type from the pointer value.
    fn track_load(&mut self, destination: Value, pointer: Value) {
        // resolve the pointee type for the load
        let Some(pointee) = self.pointee_type_for_value(pointer) else {
            return;
        };

        // record the loaded type
        self.register_value_type(destination, pointee);
    }

    /// Propagate a call result type into its destination.
    fn track_call_destination(
        &mut self,
        destination: Option<Value>,
        function: LocalNodeId<Function>,
    ) {
        // record the call result type when there is a destination
        let Some(dest) = destination else {
            return;
        };

        // record the function return type
        let callee = self.tree.get(function);
        self.register_value_type(dest, callee.return_type);
    }

    /// Insert call metadata for an indirect call when it is missing.
    fn infer_call_indirect(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        position: usize,
    ) -> ParseResult<()> {
        // skip call sites that already have metadata
        if self.tree.call_table.call_metadata(instruction_id).is_some() {
            return Ok(());
        }

        // unpack the instruction payload
        let Instruction::CallIndirect {
            destination,
            callee,
            ..
        } = instruction
        else {
            return Ok(());
        };

        // require a typed callee
        let Some(callee_type) = self.value_types.get(callee).copied() else {
            return Err(ParseError::new(
                "call.indirect requires a typed callee value",
                position,
            ));
        };

        // require a function pointer callee type
        let result_type = match self.tree.get(callee_type) {
            Type::FunctionPointer { result, .. } => *result,
            _ => {
                return Err(ParseError::new(
                    "call.indirect callee must be a function pointer type",
                    position,
                ));
            }
        };

        // store call metadata
        self.tree
            .call_table
            .insert_call_metadata(instruction_id, CallMetadata::indirect(callee_type));

        // record the destination type when the call returns a value
        let returns_void = matches!(self.tree.get(result_type), Type::Void);

        if let Some(dest) = destination
            && !returns_void
        {
            self.register_value_type(*dest, result_type);
        }

        Ok(())
    }
}
