use destack_mir::{
    BlockId, BlockTarget, Call, FunctionId, Instruction, Multiplicity, Place, PlaceType, Reference,
    Space, Substitution, Successor, Terminator, Tree, Type, TypeId, Value, is_copy,
};
use smallvec::{SmallVec, smallvec};

use crate::verify::VerifyState;

impl VerifyState<'_> {
    /// Validate the MIR invariants of one function, returning whether it holds them all.
    pub(in crate::verify) fn validate(&mut self, function_id: FunctionId) -> bool {
        let tree = self.tree;
        let violations = self.invalid_mir.len();

        // validate every instruction and terminator
        for &block_id in tree.get(function_id).blocks() {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                self.validate_instruction(function_id, tree.get(instruction_id));
            }
            self.validate_terminator(function_id, block_id, tree.get(block.terminator));
        }

        self.invalid_mir.len() == violations
    }

    /// Validate the place, stores, address, allocation, select, view, and call of one instruction.
    fn validate_instruction(&mut self, function_id: FunctionId, instruction: &Instruction) {
        let tree = self.tree;
        let function = tree.get(function_id);

        // type the place one memory operation selects
        if let Some(place) = instruction.place()
            && place.ty(function_id, tree).is_none()
        {
            self.reject_mir(
                function_id,
                "a memory operation on an untyped place".to_string(),
            );
            return;
        }

        // fill every destination with a value of its type
        match stored_values(tree, function_id, instruction) {
            Ok(stored) => {
                for (value, destination) in stored {
                    let actual = function.expect_value_type(value);
                    self.validate_fill(function_id, actual, destination, "store");
                }
            }
            Err(message) => self.reject_mir(function_id, message.to_string()),
        }

        match instruction {
            // address a place as a reference to its own type
            Instruction::Address {
                place, result_type, ..
            } => self.validate_address(function_id, place, *result_type),
            // allocate a handle in the heap it names
            Instruction::NewZeroed {
                result_type, space, ..
            }
            | Instruction::NewUninit {
                result_type, space, ..
            }
            | Instruction::NewSliceZeroed {
                result_type, space, ..
            }
            | Instruction::NewSliceUninit {
                result_type, space, ..
            } => self.validate_allocation(function_id, *result_type, *space),
            // build a slice descriptor over an element address and a length
            Instruction::Aggregate {
                destination,
                values,
            } => {
                let ty = tree.storage_type(function.expect_value_type(*destination));
                if let Type::Slice { element, .. } = tree.type_definition(ty) {
                    self.validate_slice(function_id, *element, tree.get_values(*values));
                }
            }
            // select one of two operands, so both copy
            Instruction::Select { destination, .. }
                if !is_copy(
                    tree,
                    function.expect_value_type(*destination),
                    &function.generics,
                ) =>
            {
                self.reject_mir(function_id, "a select of move-only operands".to_string());
            }
            // view uninitialized struct storage as a struct of at most the fields it holds
            Instruction::Cast {
                destination,
                argument,
                ..
            } => {
                let held = uninitialized_value(tree, function.expect_value_type(*argument))
                    .and_then(|held| struct_field_count(tree, held));
                let viewed = uninitialized_value(tree, function.expect_value_type(*destination));
                if let (Some(held), Some(viewed)) = (held, viewed)
                    && struct_field_count(tree, viewed).is_none_or(|viewed| viewed > held)
                {
                    self.reject_mir(
                        function_id,
                        format!("a view of uninitialized struct storage past its {held} fields"),
                    );
                }
            }
            // pass each argument at its parameter's type
            Instruction::Call { call, .. } => self.validate_call(function_id, call),
            _ => {}
        }
    }

    /// Validate that one address result references the type of the place it addresses.
    ///
    /// A result that cannot write may also weaken the access of the referenced value.
    fn validate_address(&mut self, function_id: FunctionId, place: &Place, result_type: TypeId) {
        let tree = self.tree;
        let result = tree.type_definition(tree.storage_type(result_type));
        let Some(place_type) = place.ty(function_id, tree) else {
            unreachable!("a validated address has a typed place");
        };

        // compare the referenced value, a referenced sequence's element, else the referent identity
        let is_covariant = result
            .reference_access()
            .is_some_and(|access| !access.can_write());
        let (expected, actual, referenced) = match (place_type, place_type.element(tree)) {
            (PlaceType::Value(ty), _) => (result.pointee_type(), Some(ty), ty),
            (PlaceType::Referent(descriptor), Some(element)) => {
                (result.pointee_type(), Some(element), descriptor)
            }
            (PlaceType::Referent(descriptor), None) => {
                let descriptor_referent =
                    referent(tree.type_definition(tree.storage_type(descriptor)));

                (referent(result), descriptor_referent, descriptor)
            }
        };
        let is_valid = expected
            .zip(actual)
            .is_some_and(|(expected, actual)| fills(tree, expected, actual, is_covariant));
        if !is_valid {
            let result = self.format_type(function_id, result_type);
            let referenced = self.format_type(function_id, referenced);
            self.reject_mir(
                function_id,
                format!("an address of type '{result}' over a place of type '{referenced}'"),
            );
        }
    }

    /// Validate the call arguments, returned value, and edge values of one terminator.
    fn validate_terminator(
        &mut self,
        function_id: FunctionId,
        block_id: BlockId,
        terminator: &Terminator,
    ) {
        let tree = self.tree;
        match terminator {
            Terminator::Invoke { call, .. } | Terminator::TailCall { call } => {
                self.validate_call(function_id, call);
            }
            Terminator::Return { value: Some(value) } => {
                let function = tree.get(function_id);
                let actual = function.expect_value_type(*value);
                self.validate_fill(function_id, actual, function.return_type, "return");
            }
            _ => {}
        }

        // pass each edge's values at its target block's parameter types
        for (edge, target) in terminator.targets(tree, block_id) {
            self.validate_edge(function_id, terminator, edge.successor, target);
        }
    }

    /// Validate the values one edge produces and passes against its target block's parameters.
    ///
    /// A normal invoke edge of a non-void call produces the call result as the first parameter.
    /// A fallible allocation's success edge produces the allocation as the first parameter.
    /// Every other edge produces nothing, and the explicit arguments fill the remaining parameters.
    fn validate_edge(
        &mut self,
        function_id: FunctionId,
        terminator: &Terminator,
        successor: Successor,
        target: &BlockTarget,
    ) {
        let tree = self.tree;
        let function = tree.get(function_id);
        let parameters = &tree.get(target.block).parameters;
        let produced = terminator.target_result_count(tree, successor);

        // fill the call result or hold an allocation of the named heap
        match (terminator, &parameters[..produced]) {
            (Terminator::Invoke { call, .. }, [result]) => {
                let Some((_, _, actual)) = tree.get(call.signature).function_signature_parts()
                else {
                    unreachable!("an invoke producing a result has a function signature");
                };
                self.validate_fill(function_id, actual, result.ty, "call result");
            }
            (
                Terminator::NewZeroedTry { space, .. }
                | Terminator::NewUninitTry { space, .. }
                | Terminator::NewSliceZeroedTry { space, .. }
                | Terminator::NewSliceUninitTry { space, .. },
                [allocation],
            ) => self.validate_allocation(function_id, allocation.ty, *space),
            _ => {}
        }

        // fill each remaining parameter with its explicit argument
        let Some(explicit) = terminator.target_parameters(tree, successor, target) else {
            unreachable!("an analysed block target has an invalid argument count");
        };
        for (parameter, argument) in explicit.iter().zip(target.arguments(tree)) {
            let actual = function.expect_value_type(*argument);
            self.validate_fill(function_id, actual, parameter.ty, "block argument");
        }
    }

    /// Validate one call's signature, its arity, and each argument against its parameter type.
    fn validate_call(&mut self, function_id: FunctionId, call: &Call) {
        let tree = self.tree;
        let Some((_, parameters, _)) = tree.get(call.signature).function_signature_parts() else {
            self.reject_mir(
                function_id,
                "a call without a function signature".to_string(),
            );
            return;
        };
        let arguments = tree.get_values(call.arguments);
        if arguments.len() != parameters.len() {
            self.reject_mir(
                function_id,
                format!(
                    "a call of {} arguments for {} parameters",
                    arguments.len(),
                    parameters.len()
                ),
            );
            return;
        }

        for (parameter, argument) in parameters.iter().zip(arguments) {
            let actual = tree.get(function_id).expect_value_type(*argument);
            self.validate_fill(function_id, actual, parameter.ty, "call argument");
        }
    }

    /// Validate one value type against the type of the destination it fills.
    fn validate_fill(
        &mut self,
        function_id: FunctionId,
        actual: TypeId,
        destination: TypeId,
        role: &str,
    ) {
        if !fills(self.tree, destination, actual, true) {
            let expected = self.format_type(function_id, destination);
            let actual = self.format_type(function_id, actual);
            self.reject_mir(
                function_id,
                format!("a {role} of type '{actual}' where '{expected}' is expected"),
            );
        }
    }

    /// Validate that one allocation results in a unique owner or a handle of the heap it names.
    fn validate_allocation(&mut self, function_id: FunctionId, result_type: TypeId, space: Space) {
        let result = self
            .tree
            .type_definition(self.tree.storage_type(result_type));
        match result.reference_kind() {
            // reject a handle of another heap
            Some(Reference::Managed(declared)) if declared != space => {
                self.reject_mir(
                    function_id,
                    format!(
                        "an allocation of a {} heap handle in the {} heap",
                        declared.label(),
                        space.label()
                    ),
                );
            }
            // accept a handle of this heap or a unique owner
            Some(Reference::Managed(_) | Reference::Unique) => {}
            // reject a result owning nothing
            _ => {
                let result = self.format_type(function_id, result_type);
                self.reject_mir(
                    function_id,
                    format!("an allocation of a non-owning result '{result}'"),
                );
            }
        }
    }

    /// Validate that one slice descriptor pairs an element address with a length.
    fn validate_slice(&mut self, function_id: FunctionId, element: TypeId, values: &[Value]) {
        let tree = self.tree;
        let function = tree.get(function_id);
        let is_valid = match values {
            [address, length] => {
                let address = tree.storage_type(function.expect_value_type(*address));
                let length = tree.storage_type(function.expect_value_type(*length));

                matches!(
                    tree.type_definition(address),
                    Type::Pointer { pointee, .. } if fills(tree, element, *pointee, true)
                ) && matches!(tree.type_definition(length), Type::Usize)
            }
            _ => false,
        };
        if !is_valid {
            self.reject_mir(
                function_id,
                "a slice descriptor of other than an element address and a length".to_string(),
            );
        }
    }
}

/// Return each value one instruction stores with the type of the storage it fills.
///
/// A slice descriptor stores an untyped address and length, which `validate_slice` checks.
pub(super) fn stored_values(
    tree: &Tree,
    function_id: FunctionId,
    instruction: &Instruction,
) -> Result<SmallVec<[(Value, TypeId); 4]>, &'static str> {
    let function = tree.get(function_id);
    match instruction {
        // store one value into the place it names
        Instruction::Store { place, value } => match place.ty(function_id, tree) {
            Some(PlaceType::Value(ty)) => Ok(smallvec![(*value, ty)]),
            _ => Err("a store into no sized value"),
        },
        // fill each slot of a struct, tuple, array, or newtype
        Instruction::Aggregate {
            destination,
            values,
        } => {
            let ty = Substitution::resolve(function.expect_value_type(*destination), tree);
            if matches!(
                tree.type_definition(tree.storage_type(ty)),
                Type::Slice { .. }
            ) {
                return Ok(SmallVec::new());
            }
            let definition = tree.type_definition(ty);

            tree.get_values(*values)
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    let slot = match definition {
                        Type::Struct { fields, .. } => {
                            fields.get(index).map(|field| tree.get(*field).ty)
                        }
                        Type::Tuple { elements, .. } => elements.get(index).copied(),
                        Type::FixedArray { element, .. } => Some(*element),
                        Type::Newtype { value, .. } if index == 0 => Some(*value),
                        _ => None,
                    };

                    slot.map(|slot| (*value, slot))
                        .ok_or("an aggregate value past its destination's slots")
                })
                .collect()
        }
        // fill one field, element, or payload
        Instruction::FieldSet {
            aggregate,
            field,
            value,
            ..
        } => {
            let ty = Substitution::resolve(function.expect_value_type(*aggregate), tree);
            let field = tree
                .get(ty)
                .field_type(*field, tree)
                .ok_or("a field set past its aggregate's fields")?;

            Ok(smallvec![(*value, field)])
        }
        Instruction::ElementSet {
            aggregate, value, ..
        } => {
            let ty = Substitution::resolve(function.expect_value_type(*aggregate), tree);
            let Type::FixedArray { element, .. } = tree.get(ty) else {
                return Err("an element set of no fixed array");
            };

            Ok(smallvec![(*value, *element)])
        }
        Instruction::VariantNew {
            payload: Some(payload),
            case,
            result_type,
            ..
        } => {
            let Type::Variant { cases, .. } = tree.get(Substitution::resolve(*result_type, tree))
            else {
                return Err("a variant.new of no variant type");
            };
            let case = cases
                .get(*case as usize)
                .ok_or("a variant.new of a case out of range")?;

            Ok(smallvec![(*payload, case.ty)])
        }
        _ => Ok(SmallVec::new()),
    }
}

/// Return the signature or dynamic constraint one function or dynamic descriptor references.
fn referent(descriptor: &Type) -> Option<TypeId> {
    match descriptor {
        Type::Function { signature, .. } => Some(*signature),
        Type::Dynamic { constraint, .. } => Some(*constraint),
        _ => None,
    }
}

/// Return the uninitialized value type one reference type addresses.
pub(super) fn uninitialized_value(tree: &Tree, reference: TypeId) -> Option<TypeId> {
    let Type::Reference { pointee, .. } = tree.type_definition(tree.storage_type(reference)) else {
        return None;
    };
    let Type::Uninit { value } = tree.type_definition(*pointee) else {
        return None;
    };

    Some(*value)
}

/// Return the field count of one struct type.
pub(super) fn struct_field_count(tree: &Tree, ty: TypeId) -> Option<usize> {
    let Type::Struct { fields, .. } = tree.type_definition(ty) else {
        return None;
    };

    Some(fields.len())
}

/// Return whether one value type fills a destination type up to lifetimes.
///
/// The outermost layer also takes `never` and weakens reference access and multiplicity.
fn fills(tree: &Tree, expected: TypeId, actual: TypeId, is_outermost: bool) -> bool {
    let expected = Substitution::resolve(expected, tree);
    let actual = Substitution::resolve(actual, tree);
    if expected == actual {
        return true;
    }

    // compare declarations by identity, which also ends recursive definitions
    if tree.is_identified_type(expected) || tree.is_identified_type(actual) {
        return false;
    }

    // take `never` outermost, a parameter for its referent, and a value for its storage
    match (tree.get(expected), tree.get(actual)) {
        (_, Type::Never) if is_outermost => return true,
        (
            Type::Parameter {
                index: expected, ..
            },
            Type::Parameter { index: actual, .. },
        ) => {
            return expected == actual;
        }
        (Type::Uninit { value: expected }, Type::Uninit { value: actual }) => {
            return fills(tree, *expected, *actual, is_outermost);
        }
        (Type::Uninit { value }, _) => return fills(tree, *value, actual, is_outermost),
        _ => {}
    }

    // strip lifetimes and region binders from both shells
    let placeholder = expected;
    let mut expected = shell(tree.get(expected));
    let mut actual = shell(tree.get(actual));

    // weaken the outermost access and multiplicity the destination asks for
    if is_outermost {
        if let (Some(required), Some(granted)) =
            (expected.reference_access(), actual.reference_access())
        {
            if !granted.grants(required) {
                return false;
            }
            actual = actual.with_reference_access(required);
        }
        if let (
            Type::Function {
                multiplicity: Multiplicity::Once,
                ..
            },
            Type::Function { multiplicity, .. },
        ) = (&expected, &mut actual)
        {
            *multiplicity = Multiplicity::Once;
        }
    }

    // compare struct fields by name, attributes, and type
    if let (Type::Struct { fields: expected }, Type::Struct { fields: actual }) =
        (&expected, &actual)
    {
        return expected.len() == actual.len()
            && expected.iter().zip(actual).all(|(expected, actual)| {
                let (expected, actual) = (tree.get(*expected), tree.get(*actual));
                expected.name == actual.name
                    && expected.attributes == actual.attributes
                    && fills(tree, expected.ty, actual.ty, false)
            });
    }

    // compare the shells with their children replaced by one placeholder, then each child pair
    let mut expected_children = Vec::new();
    let mut actual_children = Vec::new();
    expected.map_child_type_ids(&mut |child| {
        expected_children.push(child);
        placeholder
    });
    actual.map_child_type_ids(&mut |child| {
        actual_children.push(child);
        placeholder
    });

    expected == actual
        && expected_children
            .iter()
            .zip(&actual_children)
            .all(|(expected, actual)| fills(tree, *expected, *actual, false))
}

/// Return one type definition without lifetimes or region binders.
fn shell(ty: &Type) -> Type {
    let mut shell = ty.erased_lifetimes();
    if let Type::FunctionSignature { lifetimes, .. } = &mut shell {
        lifetimes.clear();
    }

    shell
}
