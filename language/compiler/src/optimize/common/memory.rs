use std::collections::{HashMap, HashSet};

use destack_mir as mir;

use crate::optimize::TypeContext;
use crate::optimize::analyses::{AliasAnalysis, MemoryAccessEffect, MemoryAccessLocation};

use super::{TypeKey, ValueTypeMap};

/// A memory location being accessed.
///
/// Represents a specific region of memory with an optional known size and type.
/// This is the fundamental unit for alias queries: "do these two locations overlap?"
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// The pointer value being dereferenced.
    pub ptr: mir::Value,
    /// Size of the access in bytes, if known.
    pub size: Option<u64>,
    /// Type being accessed, for TBAA.
    pub access_type: Option<TypeKey>,
    /// Reference kind for the pointer, when known.
    pub pointer_kind: Option<mir::ReferenceKind>,
    /// Address space for the pointer, when known.
    pub pointer_address_space: Option<mir::AddressSpace>,
}

/// Return true when a memory effect is trackable by optimizations.
///
/// Trackable effects have known locations and are not volatile or barriers.
pub fn effect_is_trackable(effect: &MemoryAccessEffect) -> bool {
    !effect.is_volatile
        && !effect.is_barrier
        && !matches!(effect.location, MemoryAccessLocation::Unknown)
}

/// Return the stack allocation base for a derived pointer value.
///
/// This walks through address computations to find the original stack alloc.
pub fn stack_alloc_base(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    tree: &mir::Tree,
) -> Option<mir::Value> {
    // walk pointer definitions to find the base allocation
    let mut current = value;
    let mut visited = HashSet::new();

    // iterate through pointer derivations until a base is found
    loop {
        // stop on cycles
        if !visited.insert(current) {
            return None;
        }

        // read the defining instruction
        let instruction_id = definitions.get(&current)?;
        let instruction = tree.get(*instruction_id);

        // walk through address computations
        match instruction {
            mir::Instruction::StackAlloc { destination, .. }
                if destination.value() == Some(current) =>
            {
                return Some(current);
            }
            mir::Instruction::FieldAddr { aggregate, .. } => {
                current = aggregate.value()?;
            }
            mir::Instruction::ElementAddr { array, .. } => {
                current = array.value()?;
            }
            mir::Instruction::Cast { argument, .. } => {
                current = argument.value()?;
            }
            _ => return None,
        }
    }
}

/// Collect stack allocations that do not escape the function.
pub fn collect_non_escaping_stack_allocs(
    function: &mir::Function,
    tree: &mir::Tree,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> HashSet<mir::Value> {
    // collect stack allocation bases
    let mut stack_allocs = HashSet::new();

    // scan blocks for stack allocations
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the instruction
            let instruction = tree.get(instruction_id);
            if let mir::Instruction::StackAlloc { destination, .. } = instruction
                && let Some(destination) = destination.value()
            {
                stack_allocs.insert(destination);
            }
        }
    }

    // collect escaping stack allocations
    let mut escaping = HashSet::new();
    let local_defs = collect_local_defs(function, tree);
    let param_defs = collect_block_param_defs(function, tree);

    // scan blocks for escaping uses
    for &block_id in &function.blocks {
        // read the block
        let block = tree.get(block_id);

        // scan instructions in the block
        for &instruction_id in &block.instructions {
            // read the instruction
            let instruction = tree.get(instruction_id);
            match instruction {
                mir::Instruction::Call { .. }
                | mir::Instruction::CallVirtual { .. }
                | mir::Instruction::CallInterface { .. }
                | mir::Instruction::CallIndirect { .. } => {
                    // capture call effects for escape checks
                    let argument_attributes = instruction.call_argument_attributes();

                    // mark stack pointers passed to calls as escaping
                    if let Some(arg_slice) = instruction.argument_slice() {
                        let arguments = tree.get_arguments(arg_slice);

                        for (index, arg) in arguments.iter().copied().enumerate() {
                            if call_argument_escapes(argument_attributes, index) {
                                record_stack_escape_reference(
                                    arg,
                                    definitions,
                                    &local_defs,
                                    &param_defs,
                                    tree,
                                    &stack_allocs,
                                    &mut escaping,
                                );
                            }
                        }
                    }
                }
                mir::Instruction::Store { value, .. } => {
                    // mark stored stack pointers as escaping
                    record_stack_escape_reference(
                        *value,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
                _ => {}
            }
        }

        // scan terminators for escaping values
        let terminator = tree.get(block.terminator);
        match terminator {
            mir::Terminator::Error => return HashSet::new(),
            mir::Terminator::Return { value: Some(value) } => {
                record_stack_escape_reference(
                    *value,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
            }
            mir::Terminator::Jump { target } => {
                for arg in target.arguments.iter().copied() {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                for arg in then_target
                    .arguments
                    .iter()
                    .chain(else_target.arguments.iter())
                    .copied()
                {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                for arg in success
                    .arguments
                    .iter()
                    .chain(failure.arguments.iter())
                    .copied()
                {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Switch { cases, default, .. } => {
                for arg in default.arguments.iter().copied() {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
                for case in cases {
                    for arg in case.target.arguments.iter().copied() {
                        record_stack_escape_reference(
                            arg,
                            definitions,
                            &local_defs,
                            &param_defs,
                            tree,
                            &stack_allocs,
                            &mut escaping,
                        );
                    }
                }
            }
            mir::Terminator::Yield { value, resume, .. } => {
                record_stack_escape_reference(
                    *value,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
                for arg in resume.arguments.iter().copied() {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Invoke {
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                for arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                    .copied()
                {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::InvokeIndirect {
                callee,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                record_stack_escape_reference(
                    *callee,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
                for arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                    .copied()
                {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::InvokeVirtual {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            }
            | mir::Terminator::InvokeInterface {
                receiver,
                call,
                normal_target,
                unwind_target,
                ..
            } => {
                record_stack_escape_reference(
                    *receiver,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
                for arg in call
                    .arguments
                    .iter()
                    .chain(normal_target.arguments.iter())
                    .chain(unwind_target.arguments.iter())
                    .copied()
                {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Throw { value } => {
                record_stack_escape_reference(
                    *value,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
            }
            mir::Terminator::Trap { payload, .. } => {
                if let Some(payload) = payload {
                    record_stack_escape_reference(
                        *payload,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::TailCall { call, .. }
            | mir::Terminator::TailCallVirtual { call, .. }
            | mir::Terminator::TailCallInterface { call, .. } => {
                for arg in call.arguments.iter().copied() {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::TailCallIndirect { callee, call, .. } => {
                record_stack_escape_reference(
                    *callee,
                    definitions,
                    &local_defs,
                    &param_defs,
                    tree,
                    &stack_allocs,
                    &mut escaping,
                );
                for arg in call.arguments.iter().copied() {
                    record_stack_escape_reference(
                        arg,
                        definitions,
                        &local_defs,
                        &param_defs,
                        tree,
                        &stack_allocs,
                        &mut escaping,
                    );
                }
            }
            mir::Terminator::Unreachable | mir::Terminator::Return { value: None } => {}
        }
    }

    // retain only stack allocations that never escaped
    stack_allocs
        .difference(&escaping)
        .copied()
        .collect::<HashSet<_>>()
}

/// Report whether a call argument may escape.
fn call_argument_escapes(
    argument_attributes: Option<&[mir::ArgumentAttribute]>,
    index: usize,
) -> bool {
    // default to escaping when argument attributes are missing
    let Some(argument_attributes) = argument_attributes else {
        return true;
    };

    // default to escaping when argument attributes are missing
    let Some(argument_attribute) = argument_attributes.get(index) else {
        return true;
    };

    // treat no capture arguments as non escaping
    !matches!(
        argument_attribute.attributes.capture,
        mir::CaptureKind::NoCapture
    )
}

/// Record a stack escape by walking derived values.
fn record_stack_escape(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    local_defs: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    param_defs: &HashMap<mir::Value, Vec<mir::Value>>,
    tree: &mir::Tree,
    stack_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
) {
    // record stack escapes by walking value definitions
    let mut visited = HashSet::new();
    record_stack_escape_value(
        value,
        definitions,
        local_defs,
        param_defs,
        tree,
        stack_allocs,
        escaping,
        &mut visited,
    );
}

/// Record a stack escape for a recoverable value reference.
fn record_stack_escape_reference(
    value: mir::ValueReference,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    local_defs: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    param_defs: &HashMap<mir::Value, Vec<mir::Value>>,
    tree: &mir::Tree,
    stack_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
) {
    let Some(value) = value.value() else {
        return;
    };

    record_stack_escape(
        value,
        definitions,
        local_defs,
        param_defs,
        tree,
        stack_allocs,
        escaping,
    );
}

/// Record stack escapes from a value and its derived operands.
#[allow(clippy::too_many_arguments)]
fn record_stack_escape_value(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    local_defs: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    param_defs: &HashMap<mir::Value, Vec<mir::Value>>,
    tree: &mir::Tree,
    stack_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
    visited: &mut HashSet<mir::Value>,
) {
    let mut bases = HashSet::new();
    collect_stack_alloc_bases_for_value(
        value,
        definitions,
        local_defs,
        param_defs,
        tree,
        stack_allocs,
        visited,
        &mut bases,
    );

    escaping.extend(bases);
}

/// Collect local definitions for stack escape tracking.
pub(crate) fn collect_local_defs(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>> {
    let mut defs: HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            if let mir::Instruction::LocalSet { local, value } = tree.get(instruction_id) {
                let Some(local) = local.local() else {
                    continue;
                };
                let Some(value) = value.value() else {
                    continue;
                };

                defs.entry(local).or_default().push(value);
            }
        }
    }

    defs
}

/// Collect block parameter definitions from predecessor arguments.
pub(crate) fn collect_block_param_defs(
    function: &mir::Function,
    tree: &mir::Tree,
) -> HashMap<mir::Value, Vec<mir::Value>> {
    let mut defs: HashMap<mir::Value, Vec<mir::Value>> = HashMap::new();

    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        let terminator = tree.get(block.terminator);

        match terminator {
            mir::Terminator::Jump { target } => {
                add_param_defs(&mut defs, target, tree);
            }
            mir::Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                add_param_defs(&mut defs, then_target, tree);
                add_param_defs(&mut defs, else_target, tree);
            }
            mir::Terminator::Check {
                success, failure, ..
            } => {
                add_param_defs(&mut defs, success, tree);
                add_param_defs(&mut defs, failure, tree);
            }
            mir::Terminator::Switch { cases, default, .. } => {
                add_param_defs(&mut defs, default, tree);
                for case in cases {
                    add_param_defs(&mut defs, &case.target, tree);
                }
            }
            mir::Terminator::Yield { resume, .. } => {
                add_param_defs(&mut defs, resume, tree);
            }
            _ => {}
        }
    }

    defs
}

/// Add predecessor arguments as block parameter definitions.
fn add_param_defs(
    defs: &mut HashMap<mir::Value, Vec<mir::Value>>,
    target: &mir::BlockTarget,
    tree: &mir::Tree,
) {
    let Some(block_id) = target.block.block() else {
        return;
    };

    let target_block = tree.get(block_id);
    let target_params = &target_block.parameters;
    for (param, arg) in target_params.iter().zip(target.arguments.iter()) {
        let Some(param) = param.value.value() else {
            continue;
        };
        let Some(arg) = arg.value() else {
            continue;
        };

        defs.entry(param).or_default().push(arg);
    }
}

/// Collect stack allocation bases reachable from a value.
#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_stack_alloc_bases_for_value(
    value: mir::Value,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    local_defs: &HashMap<mir::LocalNodeId<mir::Local>, Vec<mir::Value>>,
    param_defs: &HashMap<mir::Value, Vec<mir::Value>>,
    tree: &mir::Tree,
    stack_allocs: &HashSet<mir::Value>,
    visited: &mut HashSet<mir::Value>,
    bases: &mut HashSet<mir::Value>,
) {
    // avoid repeating work for values
    if !visited.insert(value) {
        return;
    }

    // resolve the stack base
    if let Some(base) = stack_alloc_base(value, definitions, tree) {
        if stack_allocs.contains(&base) {
            bases.insert(base);
        }
        return;
    }

    // look through derived values
    let Some(instruction_id) = definitions.get(&value) else {
        // walk block parameter definitions when present
        if let Some(params) = param_defs.get(&value) {
            for &arg in params {
                collect_stack_alloc_bases_for_value(
                    arg,
                    definitions,
                    local_defs,
                    param_defs,
                    tree,
                    stack_allocs,
                    visited,
                    bases,
                );
            }
        }
        return;
    };

    let instruction = tree.get(*instruction_id);
    match instruction {
        mir::Instruction::Struct { fields, .. } => {
            let args = tree.get_arguments(*fields);
            for arg in args.iter().copied().filter_map(|value| value.value()) {
                collect_stack_alloc_bases_for_value(
                    arg,
                    definitions,
                    local_defs,
                    param_defs,
                    tree,
                    stack_allocs,
                    visited,
                    bases,
                );
            }
        }
        mir::Instruction::Tuple { elements, .. } | mir::Instruction::Array { elements, .. } => {
            let args = tree.get_arguments(*elements);
            for arg in args.iter().copied().filter_map(|value| value.value()) {
                collect_stack_alloc_bases_for_value(
                    arg,
                    definitions,
                    local_defs,
                    param_defs,
                    tree,
                    stack_allocs,
                    visited,
                    bases,
                );
            }
        }
        mir::Instruction::Select {
            then_value,
            else_value,
            ..
        } => {
            let Some(then_value) = then_value.value() else {
                return;
            };
            let Some(else_value) = else_value.value() else {
                return;
            };

            collect_stack_alloc_bases_for_value(
                then_value,
                definitions,
                local_defs,
                param_defs,
                tree,
                stack_allocs,
                visited,
                bases,
            );
            collect_stack_alloc_bases_for_value(
                else_value,
                definitions,
                local_defs,
                param_defs,
                tree,
                stack_allocs,
                visited,
                bases,
            );
        }
        mir::Instruction::FieldGet { aggregate, .. } => {
            let Some(aggregate) = aggregate.value() else {
                return;
            };

            collect_stack_alloc_bases_for_value(
                aggregate,
                definitions,
                local_defs,
                param_defs,
                tree,
                stack_allocs,
                visited,
                bases,
            );
        }
        mir::Instruction::ElementGet { array, .. } => {
            let Some(array) = array.value() else {
                return;
            };

            collect_stack_alloc_bases_for_value(
                array,
                definitions,
                local_defs,
                param_defs,
                tree,
                stack_allocs,
                visited,
                bases,
            );
        }
        mir::Instruction::LocalGet { local, .. } => {
            let Some(local) = local.local() else {
                return;
            };

            if let Some(values) = local_defs.get(&local) {
                for &arg in values {
                    collect_stack_alloc_bases_for_value(
                        arg,
                        definitions,
                        local_defs,
                        param_defs,
                        tree,
                        stack_allocs,
                        visited,
                        bases,
                    );
                }
            }
        }
        _ => {}
    }
}

impl MemoryLocation {
    /// Create a location from just a pointer (unknown size).
    pub fn from_ptr(ptr: mir::Value) -> Self {
        Self {
            ptr,
            size: None,
            access_type: None,
            pointer_kind: None,
            pointer_address_space: None,
        }
    }

    /// Create a location with known size.
    pub fn with_size(ptr: mir::Value, size: u64) -> Self {
        Self {
            ptr,
            size: Some(size),
            access_type: None,
            pointer_kind: None,
            pointer_address_space: None,
        }
    }

    /// Create a location with type information.
    pub fn with_type(ptr: mir::Value, access_type: TypeKey) -> Self {
        let (pointer_kind, pointer_address_space) = match &access_type {
            TypeKey::Reference {
                kind,
                address_space,
                ..
            }
            | TypeKey::TensorView {
                kind,
                address_space,
                ..
            } => (Some(*kind), Some(address_space.clone())),
            _ => (None, None),
        };
        Self {
            ptr,
            size: None,
            access_type: Some(access_type),
            pointer_kind,
            pointer_address_space,
        }
    }

    /// Create a fully specified location.
    pub fn new(
        ptr: mir::Value,
        size: Option<u64>,
        access_type: Option<TypeKey>,
        pointer_kind: Option<mir::ReferenceKind>,
        pointer_address_space: Option<mir::AddressSpace>,
    ) -> Self {
        Self {
            ptr,
            size,
            access_type,
            pointer_kind,
            pointer_address_space,
        }
    }
}

/// Check whether alias scopes permit two accesses to alias.
pub fn alias_scopes_may_alias(
    alias_scopes_a: &[mir::MemoryAliasScopeId],
    noalias_scopes_a: &[mir::MemoryAliasScopeId],
    alias_scopes_b: &[mir::MemoryAliasScopeId],
    noalias_scopes_b: &[mir::MemoryAliasScopeId],
) -> bool {
    // check noalias scopes from the first access
    if scopes_intersect(noalias_scopes_a, alias_scopes_b) {
        return false;
    }

    // check noalias scopes from the second access
    if scopes_intersect(noalias_scopes_b, alias_scopes_a) {
        return false;
    }

    // allow aliasing when no disambiguation applies
    true
}

/// Check whether two memory access effects describe the same location.
pub fn effects_match_location(
    tree: &mir::Tree,
    alias: &AliasAnalysis,
    current: &MemoryAccessEffect,
    previous: &MemoryAccessEffect,
) -> bool {
    // check alias scopes and noalias scopes
    if !alias_scopes_may_alias(
        &current.alias_scopes,
        &current.noalias_scopes,
        &previous.alias_scopes,
        &previous.noalias_scopes,
    ) {
        return false;
    }

    // check location sets
    if !space_sets_may_alias(current.space_set, previous.space_set) {
        return false;
    }

    // check address spaces
    if !address_spaces_may_alias(&current.address_spaces, &previous.address_spaces) {
        return false;
    }

    // check type-alias disambiguation
    if !type_alias_tags_may_alias(
        &tree.metadata.memory.type_alias,
        current.type_alias_tag,
        previous.type_alias_tag,
    ) {
        return false;
    }

    // compare concrete locations
    match (&current.location, &previous.location) {
        (MemoryAccessLocation::Local(local), MemoryAccessLocation::Local(other_local)) => {
            local == other_local
        }
        (MemoryAccessLocation::Pointer(current_ptr), MemoryAccessLocation::Pointer(other_ptr)) => {
            if !memory_locations_compatible(current_ptr, other_ptr) {
                return false;
            }

            alias.alias(current_ptr, other_ptr).is_must_alias()
        }
        _ => false,
    }
}

/// Check whether two access effects may alias.
pub fn effects_may_alias(
    tree: &mir::Tree,
    alias: &AliasAnalysis,
    left: &MemoryAccessEffect,
    right: &MemoryAccessEffect,
) -> bool {
    // check alias scopes and noalias scopes
    if !alias_scopes_may_alias(
        &left.alias_scopes,
        &left.noalias_scopes,
        &right.alias_scopes,
        &right.noalias_scopes,
    ) {
        return false;
    }

    // check location sets
    if !space_sets_may_alias(left.space_set, right.space_set) {
        return false;
    }

    // check address spaces
    if !address_spaces_may_alias(&left.address_spaces, &right.address_spaces) {
        return false;
    }

    // check type-alias disambiguation
    if !type_alias_tags_may_alias(
        &tree.metadata.memory.type_alias,
        left.type_alias_tag,
        right.type_alias_tag,
    ) {
        return false;
    }

    // compare concrete locations
    match (&left.location, &right.location) {
        (MemoryAccessLocation::Unknown, _) | (_, MemoryAccessLocation::Unknown) => true,
        (MemoryAccessLocation::Local(local), MemoryAccessLocation::Local(other)) => local == other,
        (MemoryAccessLocation::Pointer(left_ptr), MemoryAccessLocation::Pointer(right_ptr)) => {
            if !memory_locations_compatible(left_ptr, right_ptr) {
                return false;
            }

            alias.alias(left_ptr, right_ptr).may_alias()
        }
        _ => false,
    }
}

/// Return true when an instruction has ordered memory access metadata.
pub fn instruction_has_atomic_ordering(
    tree: &mir::Tree,
    instruction: mir::LocalNodeId<mir::Instruction>,
) -> bool {
    // atomic instructions carry ordering on the instruction
    if matches!(
        tree.get(instruction),
        mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::Barrier { .. }
    ) {
        return true;
    }

    // read access metadata for this instruction
    let Some(accesses) = tree.metadata.memory.memory_accesses(instruction) else {
        return false;
    };

    // check for ordered or fenced accesses
    accesses.iter().any(|access| {
        access.ordering.is_some()
            || access.semantics.is_some()
            || matches!(access.kind, mir::MemoryAccessKind::Fence)
    })
}

/// Return true when an instruction requires exact memory access semantics.
pub fn instruction_requires_exact_access(
    tree: &mir::Tree,
    instruction: mir::LocalNodeId<mir::Instruction>,
) -> bool {
    // atomic instructions must preserve exact access semantics
    if matches!(
        tree.get(instruction),
        mir::Instruction::AtomicLoad { .. }
            | mir::Instruction::AtomicStore { .. }
            | mir::Instruction::AtomicCompareExchange { .. }
            | mir::Instruction::AtomicRmw { .. }
            | mir::Instruction::AtomicFence { .. }
            | mir::Instruction::Barrier { .. }
    ) {
        return true;
    }

    // read memory access metadata for the instruction
    let Some(accesses) = tree.metadata.memory.memory_accesses(instruction) else {
        return false;
    };

    // require exact access semantics for volatile, ordered, or fenced operations
    accesses.iter().any(|access| {
        access.is_volatile
            || access.ordering.is_some()
            || access.semantics.is_some()
            || matches!(access.kind, mir::MemoryAccessKind::Fence)
    })
}

/// Check if two memory locations are compatible for value forwarding.
pub fn memory_locations_compatible(a: &MemoryLocation, b: &MemoryLocation) -> bool {
    if let (Some(size_a), Some(size_b)) = (a.size, b.size)
        && size_a != size_b
    {
        return false;
    }

    if let (Some(type_a), Some(type_b)) = (&a.access_type, &b.access_type)
        && type_a != type_b
    {
        return false;
    }

    true
}

/// Check whether two location sets may alias.
pub fn space_sets_may_alias(a: mir::MemorySpaceSet, b: mir::MemorySpaceSet) -> bool {
    !a.is_disjoint(b)
}

/// Check whether two address space sets may alias.
pub fn address_spaces_may_alias(
    a: &Option<mir::AddressSpaceSet>,
    b: &Option<mir::AddressSpaceSet>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => !a.is_disjoint(b),
        _ => true,
    }
}

/// Check whether two type-alias tags may alias.
pub fn type_alias_tags_may_alias(
    type_alias: &mir::TypeAliasTable,
    tag_a: Option<mir::TypeAliasTagId>,
    tag_b: Option<mir::TypeAliasTagId>,
) -> bool {
    // require both tags for disambiguation
    let (Some(tag_a), Some(tag_b)) = (tag_a, tag_b) else {
        return true;
    };

    // resolve tags to base and access nodes
    let tag_a = type_alias.tag(tag_a);
    let tag_b = type_alias.tag(tag_b);

    // disjoint offsets within the same base access never alias
    if tag_a.base == tag_b.base
        && tag_a.access == tag_b.access
        && tag_a.size != 0
        && tag_b.size != 0
        && ranges_disjoint(tag_a.offset, tag_a.size, tag_b.offset, tag_b.size)
    {
        return false;
    }

    // base nodes must be compatible
    if !type_alias_nodes_may_alias(type_alias, tag_a.base, tag_b.base) {
        return false;
    }

    // access nodes must be compatible
    if !type_alias_nodes_may_alias(type_alias, tag_a.access, tag_b.access) {
        return false;
    }

    true
}

/// Check whether two alias scope slices intersect.
fn scopes_intersect(
    scopes_a: &[mir::MemoryAliasScopeId],
    scopes_b: &[mir::MemoryAliasScopeId],
) -> bool {
    // scan for any matching scope id
    for scope_a in scopes_a {
        // check for a matching id in the other list
        if scopes_b.iter().any(|scope_b| scope_b == scope_a) {
            return true;
        }
    }

    false
}

/// Check if two half open byte ranges are disjoint.
fn ranges_disjoint(offset_a: u64, size_a: u64, offset_b: u64, size_b: u64) -> bool {
    // compute end offsets with saturation
    let end_a = offset_a.saturating_add(size_a);
    let end_b = offset_b.saturating_add(size_b);

    end_a <= offset_b || end_b <= offset_a
}

/// Check whether two type-alias nodes may alias.
fn type_alias_nodes_may_alias(
    type_alias: &mir::TypeAliasTable,
    node_a: mir::TypeAliasNodeId,
    node_b: mir::TypeAliasNodeId,
) -> bool {
    // fast path for identical nodes
    if node_a == node_b {
        return true;
    }

    // allow aliasing when a is an ancestor of b
    if type_alias_node_is_ancestor(type_alias, node_a, node_b) {
        return true;
    }

    // allow aliasing when b is an ancestor of a
    type_alias_node_is_ancestor(type_alias, node_b, node_a)
}

/// Check whether one type-alias node is an ancestor of another node.
fn type_alias_node_is_ancestor(
    type_alias: &mir::TypeAliasTable,
    ancestor: mir::TypeAliasNodeId,
    node: mir::TypeAliasNodeId,
) -> bool {
    // walk up the parent chain
    let mut current = Some(node);
    let mut visited = HashSet::new();

    while let Some(node_id) = current {
        // break on cycles
        if !visited.insert(node_id) {
            break;
        }

        // report when the ancestor is reached
        if node_id == ancestor {
            return true;
        }

        // climb to the parent node
        current = type_alias.node(node_id).parent;
    }

    false
}

/// Resolve a pointer's pointee type when it is statically known.
pub fn resolve_pointer_pointee_type(
    pointer: mir::Value,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // resolve the reference pointee type
    let type_id = value_types.require_value_type(pointer);
    let ty = tree.get(type_id);
    match ty {
        mir::Type::Reference { pointee, .. } => pointee.ty(),
        mir::Type::TensorView { element, .. } => element.ty(),
        _ => None,
    }
}

/// Resolve a pointer's address space when it is statically known.
pub fn resolve_pointer_address_space(
    pointer: mir::Value,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) -> Option<mir::AddressSpace> {
    // resolve the reference address space
    let ty_id = value_types.require_value_type(pointer);
    let ty = tree.get(ty_id);
    match ty {
        mir::Type::Reference { address_space, .. } => Some(address_space.clone()),
        mir::Type::TensorView { address_space, .. } => Some(address_space.clone()),
        _ => None,
    }
}

/// Resolve a pointer's reference kind when it is statically known.
pub fn resolve_pointer_kind(
    pointer: mir::Value,
    tree: &mir::Tree,
    value_types: &ValueTypeMap,
) -> Option<mir::ReferenceKind> {
    // resolve the reference kind from the pointer type
    let ty_id = value_types.require_value_type(pointer);
    let ty = tree.get(ty_id);
    match ty {
        mir::Type::Reference { kind, .. } => Some(*kind),
        mir::Type::TensorView { kind, .. } => Some(*kind),
        _ => None,
    }
}

/// Base object that a pointer ultimately derives from.
///
/// Pointers with different identified bases cannot alias.
/// This is the foundation of provenance-based alias analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerBase {
    /// Stack allocation instruction.
    StackAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Local slot address.
    Local(mir::LocalNodeId<mir::Local>),
    /// Heap allocation instruction.
    HeapAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Raw heap allocation instruction.
    RawAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Global variable address.
    Global(mir::LocalNodeId<mir::Global>),
    /// Function parameter.
    Parameter {
        /// Parameter index.
        index: u32,
        /// Whether this parameter has noalias semantics.
        noalias: bool,
    },
    /// Return value from a call instruction.
    CallResult(mir::LocalNodeId<mir::Instruction>),
    /// Unknown base (conservative).
    Unknown,
}

impl PointerBase {
    /// Check if this is an identified object (known unique allocation).
    pub fn is_identified(&self) -> bool {
        matches!(
            self,
            PointerBase::StackAlloc(_)
                | PointerBase::Local(_)
                | PointerBase::HeapAlloc(_)
                | PointerBase::RawAlloc(_)
                | PointerBase::Global(_)
        )
    }

    /// Check if this is a noalias parameter.
    pub fn is_noalias_param(&self) -> bool {
        matches!(self, PointerBase::Parameter { noalias: true, .. })
    }

    /// Check if this base is from a local allocation (stack or heap).
    pub fn is_local_alloc(&self) -> bool {
        matches!(
            self,
            PointerBase::StackAlloc(_)
                | PointerBase::Local(_)
                | PointerBase::HeapAlloc(_)
                | PointerBase::RawAlloc(_)
        )
    }
}

/// Variable offset component in pointer arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VarOffset {
    /// The index value.
    pub index: mir::Value,
    /// Scale factor (element size in bytes).
    pub scale: u64,
}

/// Decomposed pointer representation.
///
/// A pointer is decomposed into: base + const_offset + sum(var_offset * scale)
/// This enables precise offset-based alias analysis.
#[derive(Debug, Clone)]
pub struct DecomposedPointer {
    /// The underlying base object.
    pub base: PointerBase,
    /// Constant byte offset from base.
    pub const_offset: i64,
    /// Variable offsets with their scales.
    pub var_offsets: Vec<VarOffset>,
    /// Field path from base (for struct accesses).
    pub field_path: Vec<u32>,
}

impl DecomposedPointer {
    /// Create a decomposed pointer from just a base.
    pub fn from_base(base: PointerBase) -> Self {
        Self {
            base,
            const_offset: 0,
            field_path: Vec::new(),
            var_offsets: Vec::new(),
        }
    }

    /// Check if this pointer has only constant offsets (no variable indexing).
    pub fn is_constant_offset(&self) -> bool {
        self.var_offsets.is_empty()
    }

    /// Add a constant offset.
    pub fn add_const_offset(&mut self, offset: i64) {
        self.const_offset = self.const_offset.saturating_add(offset);
    }

    /// Add a field index to the path.
    pub fn add_field(&mut self, field_index: u32) {
        self.field_path.push(field_index);
    }

    /// Add a variable offset.
    pub fn add_var_offset(&mut self, index: mir::Value, scale: u64) {
        self.var_offsets.push(VarOffset { index, scale });
    }
}

/// Builder for decomposing pointers by walking the def chain.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PointerDecomposer<'a> {
    /// Cached decomposition results.
    cache: HashMap<mir::Value, DecomposedPointer>,
    /// Map from values to their constant integer values.
    constants: &'a HashMap<mir::Value, i64>,
    /// Map from values to their defining instructions.
    definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Function parameters for noalias checking.
    parameters: &'a [mir::Parameter],
    /// Whether strict borrow mode is enabled.
    strict_borrow_mode: bool,
    /// Value type map for element sizing.
    value_types: &'a ValueTypeMap,
    /// Type context for layout sensitive operations.
    type_context: TypeContext,
}

impl<'a> PointerDecomposer<'a> {
    /// Create a new decomposer.
    pub fn new(
        constants: &'a HashMap<mir::Value, i64>,
        definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
        tree: &'a mir::Tree,
        parameters: &'a [mir::Parameter],
        strict_borrow_mode: bool,
        value_types: &'a ValueTypeMap,
        type_context: TypeContext,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            constants,
            definitions,
            tree,
            parameters,
            strict_borrow_mode,
            value_types,
            type_context,
        }
    }

    /// Decompose a pointer value.
    pub fn decompose(&mut self, ptr: mir::Value) -> DecomposedPointer {
        // check cache
        if let Some(cached) = self.cache.get(&ptr) {
            return cached.clone();
        }

        let result = self.decompose_impl(ptr);
        self.cache.insert(ptr, result.clone());
        result
    }

    /// Internal decomposition logic.
    fn decompose_impl(&mut self, ptr: mir::Value) -> DecomposedPointer {
        // check if it's a parameter
        for (index, parameter) in self.parameters.iter().enumerate() {
            if parameter.value.value() == Some(ptr) {
                let noalias = self.is_parameter_noalias(parameter);
                return DecomposedPointer::from_base(PointerBase::Parameter {
                    index: index as u32,
                    noalias,
                });
            }
        }

        // check if it's defined by an instruction
        let Some(&instruction_id) = self.definitions.get(&ptr) else {
            return DecomposedPointer::from_base(PointerBase::Unknown);
        };

        let inst = self.tree.get(instruction_id);

        match inst {
            // allocations are base objects
            mir::Instruction::StackAlloc { destination, .. }
                if destination.value() == Some(ptr) =>
            {
                DecomposedPointer::from_base(PointerBase::StackAlloc(instruction_id))
            }
            mir::Instruction::New { destination, .. } if destination.value() == Some(ptr) => {
                DecomposedPointer::from_base(PointerBase::HeapAlloc(instruction_id))
            }
            mir::Instruction::NewSlice { destination, .. } if destination.value() == Some(ptr) => {
                DecomposedPointer::from_base(PointerBase::HeapAlloc(instruction_id))
            }
            mir::Instruction::RawAlloc { destination, .. } if destination.value() == Some(ptr) => {
                DecomposedPointer::from_base(PointerBase::RawAlloc(instruction_id))
            }

            // global address is a base
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } if destination.value() == Some(ptr) => {
                let Some(global) = global.global() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };

                DecomposedPointer::from_base(PointerBase::Global(global))
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } if destination.value() == Some(ptr) => {
                let Some(local) = local.local() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };

                DecomposedPointer::from_base(PointerBase::Local(local))
            }

            // field address: decompose base and add field offset
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
                ..
            } if destination.value() == Some(ptr) => {
                let Some(aggregate) = aggregate.value() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };

                let mut base_decomp = self.decompose(aggregate);
                base_decomp.add_field(*index);
                base_decomp
            }

            // element address: decompose base and add index offset
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
                ..
            } if destination.value() == Some(ptr) => {
                let Some(array) = array.value() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };
                let Some(index) = index.value() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };

                let mut base_decomp = self.decompose(array);

                let scale = self.element_size(array).unwrap_or(1).max(1);
                base_decomp.add_var_offset(index, scale);
                base_decomp
            }

            // casts preserve provenance
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            } if destination.value() == Some(ptr) => {
                let Some(argument) = argument.value() else {
                    return DecomposedPointer::from_base(PointerBase::Unknown);
                };

                self.decompose(argument)
            }

            // calls return unknown pointers
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallVirtual { destination, .. }
            | mir::Instruction::CallInterface { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. }
                if destination.and_then(|value| value.value()) == Some(ptr) =>
            {
                DecomposedPointer::from_base(PointerBase::CallResult(instruction_id))
            }

            // loads produce unknown pointers
            mir::Instruction::Load { destination, .. } if destination.value() == Some(ptr) => {
                DecomposedPointer::from_base(PointerBase::Unknown)
            }

            // anything else is unknown
            _ => DecomposedPointer::from_base(PointerBase::Unknown),
        }
    }

    /// Check if a parameter has noalias semantics.
    fn is_parameter_noalias(&self, parameter: &mir::Parameter) -> bool {
        // in strict borrow mode, &mut T parameters are noalias
        if self.strict_borrow_mode {
            let Some(ty) = parameter.ty.ty() else {
                return false;
            };
            let ty = self.tree.get(ty);
            ty.is_mutable_borrowed_reference()
        } else {
            false
        }
    }

    /// Return the value type for an SSA value.
    fn value_type(&self, value: mir::Value) -> mir::LocalNodeId<mir::Type> {
        self.value_types.require_value_type(value)
    }

    /// Resolve the element size for an array value when possible.
    fn element_size(&self, array: mir::Value) -> Option<u64> {
        let ty_id = self.value_type(array);
        let ty = self.tree.get(ty_id);

        let element_id = match ty {
            mir::Type::Array { element, .. } => element.ty()?,
            mir::Type::Reference { pointee, .. } => {
                let pointee = pointee.ty()?;
                let pointee_ty = self.tree.get(pointee);
                if let mir::Type::Array { element, .. } = pointee_ty {
                    element.ty()?
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        let key = TypeKey::from_type(element_id, self.tree);
        key.byte_size(self.type_context.pointer_width_bits)
    }
}

/// Check if two byte ranges overlap.
///
/// Returns true if [off1, off1+size1) overlaps with [off2, off2+size2).
pub fn ranges_overlap(off1: i64, size1: u64, off2: i64, size2: u64) -> bool {
    let end1 = off1.saturating_add(size1 as i64);
    let end2 = off2.saturating_add(size2 as i64);
    !(end1 <= off2 || end2 <= off1)
}

/// Check if two byte ranges are exactly equal.
pub fn ranges_equal(off1: i64, size1: u64, off2: i64, size2: u64) -> bool {
    off1 == off2 && size1 == size2
}

/// Compute the relationship between two ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeRelation {
    /// Ranges are disjoint.
    Disjoint,
    /// Ranges are exactly equal.
    Equal,
    /// First range contains second.
    Contains,
    /// Second range contains first.
    ContainedBy,
    /// Ranges partially overlap.
    Overlaps,
}

/// Determine the relationship between two byte ranges.
pub fn range_relation(off1: i64, size1: u64, off2: i64, size2: u64) -> RangeRelation {
    let end1 = off1.saturating_add(size1 as i64);
    let end2 = off2.saturating_add(size2 as i64);

    // disjoint
    if end1 <= off2 || end2 <= off1 {
        return RangeRelation::Disjoint;
    }

    // equal
    if off1 == off2 && size1 == size2 {
        return RangeRelation::Equal;
    }

    // containment
    if off1 <= off2 && end1 >= end2 {
        return RangeRelation::Contains;
    }
    if off2 <= off1 && end2 >= end1 {
        return RangeRelation::ContainedBy;
    }

    RangeRelation::Overlaps
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ranges that overlap are detected as overlapping.
    #[test]
    fn test_ranges_overlap() {
        assert!(!ranges_overlap(0, 4, 10, 4));
        assert!(!ranges_overlap(10, 4, 0, 4));

        assert!(!ranges_overlap(0, 4, 4, 4));

        assert!(ranges_overlap(0, 8, 4, 8));
        assert!(ranges_overlap(4, 8, 0, 8));

        assert!(ranges_overlap(0, 16, 4, 4));
        assert!(ranges_overlap(4, 4, 0, 16));

        assert!(ranges_overlap(0, 8, 0, 8));
    }

    /// Range relationships return the expected classification.
    #[test]
    fn test_range_relation() {
        assert_eq!(range_relation(0, 4, 10, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(0, 8, 0, 8), RangeRelation::Equal);
        assert_eq!(range_relation(0, 16, 4, 4), RangeRelation::Contains);
        assert_eq!(range_relation(4, 4, 0, 16), RangeRelation::ContainedBy);
        assert_eq!(range_relation(0, 8, 4, 8), RangeRelation::Overlaps);
    }

    /// Pointer bases report identification status.
    #[test]
    fn test_pointer_base_is_identified() {
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));
        let param = PointerBase::Parameter {
            index: 0,
            noalias: false,
        };
        let unknown = PointerBase::Unknown;

        assert!(stack.is_identified());
        assert!(!param.is_identified());
        assert!(!unknown.is_identified());
    }

    /// Decomposed pointers track constant and variable offsets.
    #[test]
    fn test_decomposed_pointer_const_offset() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::StackAlloc(mir::LocalNodeId::new(0)));
        assert!(ptr.is_constant_offset());

        ptr.add_const_offset(16);
        assert!(ptr.is_constant_offset());
        assert_eq!(ptr.const_offset, 16);

        ptr.add_var_offset(mir::Value::new(0), 4);
        assert!(!ptr.is_constant_offset());
    }

    /// Memory location constructors fill the expected fields.
    #[test]
    fn test_memory_location_constructors() {
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(0));
        assert_eq!(loc1.ptr, mir::Value::new(0));
        assert!(loc1.size.is_none());
        assert!(loc1.access_type.is_none());

        let loc2 = MemoryLocation::with_size(mir::Value::new(1), 8);
        assert_eq!(loc2.ptr, mir::Value::new(1));
        assert_eq!(loc2.size, Some(8));
        assert!(loc2.access_type.is_none());

        let ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let loc3 = MemoryLocation::with_type(mir::Value::new(2), ty.clone());
        assert_eq!(loc3.ptr, mir::Value::new(2));
        assert!(loc3.size.is_none());
        assert_eq!(loc3.access_type, Some(ty.clone()));

        let loc4 = MemoryLocation::new(mir::Value::new(3), Some(4), Some(ty.clone()), None, None);
        assert_eq!(loc4.ptr, mir::Value::new(3));
        assert_eq!(loc4.size, Some(4));
        assert_eq!(loc4.access_type, Some(ty));
    }

    /// Noalias parameters are reported as noalias.
    #[test]
    fn test_pointer_base_is_noalias_param() {
        let noalias_param = PointerBase::Parameter {
            index: 0,
            noalias: true,
        };
        let regular_param = PointerBase::Parameter {
            index: 1,
            noalias: false,
        };
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));

        assert!(noalias_param.is_noalias_param());
        assert!(!regular_param.is_noalias_param());
        assert!(!stack.is_noalias_param());
    }

    /// Local allocation bases are detected accurately.
    #[test]
    fn test_pointer_base_is_local_alloc() {
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));
        let heap = PointerBase::HeapAlloc(mir::LocalNodeId::new(1));
        let raw = PointerBase::RawAlloc(mir::LocalNodeId::new(2));
        let global = PointerBase::Global(mir::LocalNodeId::new(0));
        let param = PointerBase::Parameter {
            index: 0,
            noalias: false,
        };
        let call = PointerBase::CallResult(mir::LocalNodeId::new(3));
        let unknown = PointerBase::Unknown;

        assert!(stack.is_local_alloc());
        assert!(heap.is_local_alloc());
        assert!(raw.is_local_alloc());
        assert!(!global.is_local_alloc());
        assert!(!param.is_local_alloc());
        assert!(!call.is_local_alloc());
        assert!(!unknown.is_local_alloc());
    }

    /// Identified pointer bases report the expected status.
    #[test]
    fn test_pointer_base_all_variants_identified() {
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));
        let heap = PointerBase::HeapAlloc(mir::LocalNodeId::new(1));
        let raw = PointerBase::RawAlloc(mir::LocalNodeId::new(2));
        let global = PointerBase::Global(mir::LocalNodeId::new(0));

        assert!(stack.is_identified());
        assert!(heap.is_identified());
        assert!(raw.is_identified());
        assert!(global.is_identified());

        let param = PointerBase::Parameter {
            index: 0,
            noalias: false,
        };
        let call = PointerBase::CallResult(mir::LocalNodeId::new(3));
        let unknown = PointerBase::Unknown;

        assert!(!param.is_identified());
        assert!(!call.is_identified());
        assert!(!unknown.is_identified());
    }

    /// Field paths are captured by decomposed pointers.
    #[test]
    fn test_decomposed_pointer_field_path() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::StackAlloc(mir::LocalNodeId::new(0)));
        assert!(ptr.field_path.is_empty());

        ptr.add_field(0);
        assert_eq!(ptr.field_path, vec![0]);

        ptr.add_field(2);
        assert_eq!(ptr.field_path, vec![0, 2]);

        assert!(ptr.is_constant_offset());
    }

    /// Multiple variable offsets are tracked.
    #[test]
    fn test_decomposed_pointer_multiple_var_offsets() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::HeapAlloc(mir::LocalNodeId::new(0)));

        ptr.add_var_offset(mir::Value::new(1), 4);
        ptr.add_var_offset(mir::Value::new(2), 8);

        assert!(!ptr.is_constant_offset());
        assert_eq!(ptr.var_offsets.len(), 2);
        assert_eq!(ptr.var_offsets[0].index, mir::Value::new(1));
        assert_eq!(ptr.var_offsets[0].scale, 4);
        assert_eq!(ptr.var_offsets[1].index, mir::Value::new(2));
        assert_eq!(ptr.var_offsets[1].scale, 8);
    }

    /// Constant offsets accumulate.
    #[test]
    fn test_decomposed_pointer_const_offset_accumulation() {
        let mut ptr = DecomposedPointer::from_base(PointerBase::Global(mir::LocalNodeId::new(0)));

        ptr.add_const_offset(8);
        ptr.add_const_offset(16);

        assert_eq!(ptr.const_offset, 24);
    }

    /// Negative offsets are handled consistently.
    #[test]
    fn test_decomposed_pointer_negative_offset() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::StackAlloc(mir::LocalNodeId::new(0)));

        ptr.add_const_offset(-8);
        assert_eq!(ptr.const_offset, -8);

        ptr.add_const_offset(4);
        assert_eq!(ptr.const_offset, -4);
    }

    /// Equal ranges are detected.
    #[test]
    fn test_ranges_equal() {
        assert!(ranges_equal(0, 4, 0, 4));
        assert!(!ranges_equal(0, 4, 0, 8));
        assert!(!ranges_equal(0, 4, 4, 4));
        assert!(!ranges_equal(0, 8, 4, 8));
    }

    /// Adjacent ranges are disjoint.
    #[test]
    fn test_range_relation_adjacent() {
        assert_eq!(range_relation(0, 4, 4, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(4, 4, 0, 4), RangeRelation::Disjoint);
    }

    /// Partial overlaps are classified correctly.
    #[test]
    fn test_range_relation_partial_overlap() {
        assert_eq!(range_relation(0, 8, 4, 8), RangeRelation::Overlaps);
        assert_eq!(range_relation(4, 8, 0, 8), RangeRelation::Overlaps);
    }

    /// Zero sized ranges are classified conservatively.
    #[test]
    fn test_range_relation_zero_size() {
        assert_eq!(range_relation(0, 0, 0, 0), RangeRelation::Disjoint);

        assert_eq!(range_relation(0, 0, 0, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(0, 4, 0, 0), RangeRelation::Disjoint);

        assert_eq!(range_relation(2, 0, 0, 8), RangeRelation::ContainedBy);

        // zero-size range after the end: disjoint
        assert_eq!(range_relation(10, 0, 0, 8), RangeRelation::Disjoint);
    }
}
