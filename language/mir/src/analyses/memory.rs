use std::collections::{HashMap, HashSet};

use crate as mir;

use crate::{AliasResult, TargetLayout};

use super::{TypeKey, ValueDefinitions, ValueTypes};

/// A reference-backed memory location.
///
/// Alias queries compare these locations to decide whether accesses overlap.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ReferenceLocation {
    /// The reference value being dereferenced.
    pub reference: mir::Value,
    /// Size of the access in bytes, if known.
    pub size: Option<u64>,
    /// Type being accessed, when known.
    pub access_type: Option<TypeKey>,
    /// Reference kind for the reference, when known.
    pub reference_kind: Option<mir::ReferenceKind>,
    /// Space for the reference, when known.
    pub reference_space: Option<mir::Space>,
}

impl ValueDefinitions {
    /// Collect stack allocations that do not escape the function.
    pub fn non_escaping_frame_allocs(
        &self,
        function: &mir::Function,
        tree: &mir::Tree,
        effects: &mir::EffectTable,
    ) -> HashSet<mir::Value> {
        // collect stack allocation bases
        let mut frame_allocs = HashSet::new();

        // scan blocks for stack allocations
        for &block_id in function.blocks() {
            // read the block
            let block = tree.get(block_id);

            // scan instructions in the block
            for &instruction_id in &block.instructions {
                // read the instruction
                let instruction = tree.get(instruction_id);
                if let mir::Instruction::FrameAllocZeroed { destination, .. }
                | mir::Instruction::FrameAllocUninit { destination, .. } = instruction
                {
                    frame_allocs.insert(*destination);
                }
            }
        }

        // collect escaping stack allocations
        let mut escaping = HashSet::new();

        // scan blocks for escaping uses
        for &block_id in function.blocks() {
            // read the block
            let block = tree.get(block_id);

            // scan instructions in the block
            for &instruction_id in &block.instructions {
                // read the instruction
                let instruction = tree.get(instruction_id);
                match instruction {
                    mir::Instruction::Call { .. } => {
                        // capture call effects for escape checks
                        let argument_effects = effects
                            .call(mir::CallSite::Instruction(instruction_id))
                            .map(|tables| tables.arguments.as_slice());

                        // mark stack references passed to calls as escaping
                        if let Some(arg_slice) = instruction.argument_slice() {
                            let arguments = tree.get_values(arg_slice);

                            for (index, arg) in arguments.iter().copied().enumerate() {
                                if call_argument_escapes(argument_effects, index) {
                                    record_stack_escape(
                                        arg,
                                        self,
                                        tree,
                                        &frame_allocs,
                                        &mut escaping,
                                    );
                                }
                            }
                        }
                    }
                    mir::Instruction::Store { value, .. } => {
                        // mark stored stack references as escaping
                        record_stack_escape(*value, self, tree, &frame_allocs, &mut escaping);
                    }
                    _ => {}
                }
            }

            // scan terminators for escaping values
            let terminator = tree.get(block.terminator);
            match terminator {
                mir::Terminator::Error => {
                    panic!("invalid MIR terminator reached memory analysis");
                }
                mir::Terminator::Return { value: Some(value) } => {
                    record_stack_escape(*value, self, tree, &frame_allocs, &mut escaping);
                }
                mir::Terminator::Jump { target } => {
                    for arg in target.arguments(tree).iter().copied() {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::Branch {
                    then_target,
                    else_target,
                    ..
                } => {
                    for arg in then_target
                        .arguments(tree)
                        .iter()
                        .chain(else_target.arguments(tree).iter())
                        .copied()
                    {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::Check {
                    success, failure, ..
                } => {
                    for arg in success
                        .arguments(tree)
                        .iter()
                        .chain(failure.arguments(tree).iter())
                        .copied()
                    {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::NewZeroedTry {
                    success, failure, ..
                }
                | mir::Terminator::NewUninitTry {
                    success, failure, ..
                } => {
                    for arg in success
                        .arguments(tree)
                        .iter()
                        .chain(failure.arguments(tree).iter())
                        .copied()
                    {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::NewSliceZeroedTry {
                    length,
                    success,
                    failure,
                    ..
                }
                | mir::Terminator::NewSliceUninitTry {
                    length,
                    success,
                    failure,
                    ..
                } => {
                    record_stack_escape(*length, self, tree, &frame_allocs, &mut escaping);

                    for arg in success
                        .arguments(tree)
                        .iter()
                        .chain(failure.arguments(tree).iter())
                        .copied()
                    {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::Switch { cases, default, .. } => {
                    for arg in default.arguments(tree).iter().copied() {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                    for case in tree.get_switch_cases(*cases) {
                        for arg in case.target.arguments(tree).iter().copied() {
                            record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                        }
                    }
                }
                mir::Terminator::Yield {
                    value,
                    resume,
                    unwind,
                } => {
                    record_stack_escape(*value, self, tree, &frame_allocs, &mut escaping);
                    for arg in resume.arguments(tree).iter().copied() {
                        record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                    }
                    if let Some(unwind) = unwind {
                        for arg in unwind.arguments(tree).iter().copied() {
                            record_stack_escape(arg, self, tree, &frame_allocs, &mut escaping);
                        }
                    }
                }
                mir::Terminator::Invoke {
                    call,
                    target,
                    unwind,
                } => {
                    for argument in call
                        .callee
                        .uses()
                        .into_iter()
                        .chain(tree.get_values(call.arguments).iter().copied())
                        .chain(target.arguments(tree).iter().copied())
                        .chain(unwind.arguments(tree).iter().copied())
                    {
                        record_stack_escape(argument, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::Trap { .. } => {}
                mir::Terminator::Panic { payload } => {
                    if let Some(payload) = payload {
                        record_stack_escape(*payload, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::UnwindResume => {}
                mir::Terminator::TailCall { call } => {
                    for argument in call
                        .callee
                        .uses()
                        .into_iter()
                        .chain(tree.get_values(call.arguments).iter().copied())
                    {
                        record_stack_escape(argument, self, tree, &frame_allocs, &mut escaping);
                    }
                }
                mir::Terminator::Unreachable | mir::Terminator::Return { value: None } => {}
            }
        }

        // retain only stack allocations that never escaped
        frame_allocs
            .difference(&escaping)
            .copied()
            .collect::<HashSet<_>>()
    }
}

/// Report whether a call argument may escape.
fn call_argument_escapes(arguments: Option<&[mir::CallArgumentEffect]>, index: usize) -> bool {
    // require escape tables before treating an argument as local
    let Some(arguments) = arguments else {
        return true;
    };

    // require escape tables for the specific argument
    let Some(argument) = arguments.get(index) else {
        return true;
    };

    // treat non escaping arguments as local to the call
    !matches!(argument.escape, mir::ArgumentEscape::None)
}

/// Record a stack escape by walking derived values.
fn record_stack_escape(
    value: mir::Value,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
    frame_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
) {
    // record stack escapes by walking value definitions
    let mut visited = HashSet::new();
    record_stack_escape_value(
        value,
        definitions,
        tree,
        frame_allocs,
        escaping,
        &mut visited,
    );
}

/// Record stack escapes from a value and its derived operands.
fn record_stack_escape_value(
    value: mir::Value,
    definitions: &ValueDefinitions,
    tree: &mir::Tree,
    frame_allocs: &HashSet<mir::Value>,
    escaping: &mut HashSet<mir::Value>,
    visited: &mut HashSet<mir::Value>,
) {
    let mut bases = HashSet::new();
    definitions.collect_frame_alloc_bases(value, tree, frame_allocs, visited, &mut bases);

    escaping.extend(bases);
}

impl ReferenceLocation {
    /// Create a location from just a reference with unknown size.
    pub fn from_reference(reference: mir::Value) -> Self {
        Self {
            reference,
            size: None,
            access_type: None,
            reference_kind: None,
            reference_space: None,
        }
    }

    /// Create a location with known size.
    pub fn with_size(reference: mir::Value, size: u64) -> Self {
        Self {
            reference,
            size: Some(size),
            access_type: None,
            reference_kind: None,
            reference_space: None,
        }
    }

    /// Create a location with type information.
    pub fn with_type(reference: mir::Value, access_type: TypeKey) -> Self {
        let (reference_kind, reference_space) = match &access_type {
            TypeKey::Reference { kind, space, .. } | TypeKey::TensorView { kind, space, .. } => {
                (Some(*kind), Some(*space))
            }
            _ => (None, None),
        };
        Self {
            reference,
            size: None,
            access_type: Some(access_type),
            reference_kind,
            reference_space,
        }
    }

    /// Create a fully specified location.
    pub fn new(
        reference: mir::Value,
        size: Option<u64>,
        access_type: Option<TypeKey>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_space: Option<mir::Space>,
    ) -> Self {
        Self {
            reference,
            size,
            access_type,
            reference_kind,
            reference_space,
        }
    }

    /// Return the memory spaces this location can touch.
    pub fn spaces(&self) -> mir::StorageSet {
        self.reference_space
            .as_ref()
            .map(mir::Space::space_set)
            .unwrap_or(mir::StorageSet::ANY)
    }

    /// Return aliasing for another location with the same reference value.
    pub fn alias_same_reference(&self, other: &ReferenceLocation) -> AliasResult {
        match (self.size, other.size) {
            (Some(left), Some(right)) if left == right => AliasResult::MustAlias,
            (Some(_), Some(_)) => AliasResult::PartialAlias,
            _ => AliasResult::PartialAlias,
        }
    }

    /// Return whether both locations are compatible for value forwarding.
    pub fn is_compatible_with(&self, other: &ReferenceLocation) -> bool {
        // compare byte sizes when both sides know them
        if let (Some(left_size), Some(right_size)) = (self.size, other.size)
            && left_size != right_size
        {
            return false;
        }

        // compare access types when both sides know them
        if let (Some(left_type), Some(right_type)) = (&self.access_type, &other.access_type)
            && left_type != right_type
        {
            return false;
        }

        true
    }
}

/// Memory region touched by one reference-like value or memory access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryRegion {
    /// Reference based memory access.
    Reference {
        /// The reference access payload.
        access: ReferenceLocation,
        /// The memory spaces the reference may touch.
        spaces: mir::StorageSet,
    },
    /// A precise memory place.
    Place(MemoryPlace),
    /// Local slot access.
    Local(mir::LocalNodeId<mir::Local>),
    /// Any memory in the given spaces.
    Any {
        /// The memory spaces that may be touched.
        spaces: mir::StorageSet,
    },
}

impl MemoryRegion {
    /// Create an imprecise region for all memory spaces.
    pub fn any() -> Self {
        Self::Any {
            spaces: mir::StorageSet::ANY,
        }
    }

    /// Create an imprecise region for a set of memory spaces.
    pub fn any_spaces(spaces: mir::StorageSet) -> Self {
        Self::Any { spaces }
    }

    /// Create an imprecise region for one memory space.
    pub fn any_space(space: mir::Space) -> Self {
        Self::Any {
            spaces: space.space_set(),
        }
    }

    /// Create a reference access with optional access type and inferred size.
    pub fn from_reference(
        reference: mir::Value,
        access_type: Option<TypeKey>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_space: Option<mir::Space>,
        pointer_width_bits: u16,
    ) -> Self {
        Self::from_reference_with_size(
            reference,
            access_type,
            reference_kind,
            reference_space,
            None,
            pointer_width_bits,
        )
    }

    /// Create a reference access with an explicit size override.
    pub fn from_reference_with_size(
        reference: mir::Value,
        access_type: Option<TypeKey>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_space: Option<mir::Space>,
        size: Option<u64>,
        pointer_width_bits: u16,
    ) -> Self {
        let inferred_size = size.or_else(|| {
            access_type
                .as_ref()
                .and_then(|access_type| access_type.byte_size(pointer_width_bits))
        });

        Self::Reference {
            access: ReferenceLocation::new(
                reference,
                inferred_size,
                access_type,
                reference_kind,
                reference_space,
            ),
            spaces: mir::StorageSet::ANY,
        }
    }

    /// Return the reference location when this is reference backed.
    pub fn reference_location(&self) -> Option<&ReferenceLocation> {
        match self {
            Self::Reference { access, .. } => Some(access),
            _ => None,
        }
    }

    /// Return the memory spaces covered by this region.
    pub fn spaces(&self) -> mir::StorageSet {
        match self {
            MemoryRegion::Reference { spaces, .. } => *spaces,
            MemoryRegion::Place(place) => place.root.spaces(),
            MemoryRegion::Local(_) => mir::StorageSet::FRAME,
            MemoryRegion::Any { spaces } => *spaces,
        }
    }

    /// Set spaces on imprecise or reference regions.
    pub fn set_spaces(&mut self, new_spaces: mir::StorageSet) {
        match self {
            Self::Reference { spaces, .. } | Self::Any { spaces } => {
                *spaces = new_spaces;
            }
            Self::Place(_) | Self::Local(_) => {}
        }
    }

    /// Return whether this region may touch one storage root.
    pub fn may_touch_root(&self, storage: &StorageRoot) -> bool {
        match self {
            MemoryRegion::Place(place) => !place.root.is_disjoint_from(storage),
            MemoryRegion::Local(local) => {
                !storage.is_disjoint_from(&StorageRoot::LocalSlot(*local))
            }
            MemoryRegion::Any { spaces } => !spaces.is_disjoint(storage.spaces()),
            MemoryRegion::Reference { spaces, .. } => !spaces.is_disjoint(storage.spaces()),
        }
    }
}

/// Identified storage root for one memory place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageRoot {
    /// Frame allocation instruction.
    FrameAllocation(mir::LocalNodeId<mir::Instruction>),
    /// Local slot address.
    LocalSlot(mir::LocalNodeId<mir::Local>),
    /// Static storage address.
    Static {
        /// The static global.
        global: mir::LocalNodeId<mir::Global>,
        /// The static storage space.
        space: mir::Space,
    },
    /// Heap allocation instruction.
    Allocation {
        /// The allocation instruction.
        instruction: mir::LocalNodeId<mir::Instruction>,
        /// The allocation storage space.
        space: mir::Space,
        /// The reference kind produced by the allocation.
        kind: mir::ReferenceKind,
    },
    /// Function parameter.
    Parameter {
        /// Parameter index.
        index: u32,
        /// The parameter storage space.
        space: mir::Space,
        /// The parameter reference kind.
        kind: mir::ReferenceKind,
        /// The parameter access.
        access: mir::Access,
    },
}

impl StorageRoot {
    /// Return true if this parameter is exclusive.
    pub fn is_exclusive_parameter(&self) -> bool {
        matches!(
            self,
            StorageRoot::Parameter {
                access: mir::Access::Exclusive,
                ..
            }
        )
    }

    /// Return whether two identified storage roots are disjoint.
    pub fn is_disjoint_from(&self, other: &StorageRoot) -> bool {
        match (self, other) {
            (StorageRoot::FrameAllocation(left), StorageRoot::FrameAllocation(right)) => {
                left != right
            }
            (StorageRoot::LocalSlot(left), StorageRoot::LocalSlot(right)) => left != right,
            (
                StorageRoot::Static { global: left, .. },
                StorageRoot::Static { global: right, .. },
            ) => left != right,
            (
                StorageRoot::Allocation {
                    instruction: left, ..
                },
                StorageRoot::Allocation {
                    instruction: right, ..
                },
            ) => left != right,
            (StorageRoot::Parameter { .. }, _) | (_, StorageRoot::Parameter { .. }) => false,
            _ => true,
        }
    }

    /// Return whether exclusive parameter constraints prove disjointness.
    pub fn exclusive_parameters_are_disjoint(&self, other: &StorageRoot) -> bool {
        match (self, other) {
            (
                StorageRoot::Parameter {
                    access: mir::Access::Exclusive,
                    ..
                },
                StorageRoot::Parameter {
                    access: mir::Access::Exclusive,
                    ..
                },
            ) => self != other,
            _ => false,
        }
    }

    /// Return the memory spaces covered by this storage root.
    pub fn spaces(&self) -> mir::StorageSet {
        match self {
            StorageRoot::FrameAllocation(_) | StorageRoot::LocalSlot(_) => mir::StorageSet::FRAME,
            StorageRoot::Static { space, .. } => space.space_set(),
            StorageRoot::Allocation { space, .. } | StorageRoot::Parameter { space, .. } => {
                space.space_set()
            }
        }
    }
}

/// Indexed byte offset component in reference arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexedOffset {
    /// The index value.
    pub index: mir::Value,
    /// Scale factor (element size in bytes).
    pub scale: u64,
}

/// Precise memory place representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryPlace {
    /// The identified storage root.
    pub root: StorageRoot,
    /// Constant byte offset from storage root.
    pub const_offset: i64,
    /// Indexed offsets with their scales.
    pub indexed_offsets: Vec<IndexedOffset>,
    /// Field path from storage root.
    pub fields: Vec<u32>,
}

impl MemoryPlace {
    /// Create a memory place from one storage root.
    pub fn from_root(root: StorageRoot) -> Self {
        Self {
            root,
            const_offset: 0,
            fields: Vec::new(),
            indexed_offsets: Vec::new(),
        }
    }

    /// Check if this place has only constant offsets.
    pub fn is_constant_offset(&self) -> bool {
        self.indexed_offsets.is_empty()
    }

    /// Add a constant offset.
    pub fn add_const_offset(&mut self, offset: i64) {
        self.const_offset = self.const_offset.saturating_add(offset);
    }

    /// Add a field index to the path.
    pub fn add_field(&mut self, field_index: u32) {
        self.fields.push(field_index);
    }

    /// Add an indexed offset.
    pub fn add_indexed_offset(&mut self, index: mir::Value, scale: u64) {
        self.indexed_offsets.push(IndexedOffset { index, scale });
    }

    /// Return aliasing with another place under known access locations.
    pub fn alias_with(
        &self,
        location: &ReferenceLocation,
        other: &MemoryPlace,
        other_location: &ReferenceLocation,
    ) -> AliasResult {
        // disjoint field paths cannot alias
        if self.fields_are_disjoint_from(other) {
            return AliasResult::NoAlias;
        }

        // constant byte ranges can be compared exactly
        if self.is_constant_offset()
            && other.is_constant_offset()
            && let (Some(size), Some(other_size)) = (location.size, other_location.size)
        {
            let range = ByteRange::new(self.const_offset, size);
            let other_range = ByteRange::new(other.const_offset, other_size);

            return match range.relation(other_range) {
                RangeRelation::Disjoint => AliasResult::NoAlias,
                RangeRelation::Equal => AliasResult::MustAlias,
                _ => AliasResult::PartialAlias,
            };
        }

        // identical indexed offset values can still prove disjoint byte ranges
        if self.indexed_offsets_are_disjoint_from(other, location.size, other_location.size) {
            return AliasResult::NoAlias;
        }

        AliasResult::MayAlias
    }

    /// Return whether two field paths are statically disjoint.
    fn fields_are_disjoint_from(&self, other: &MemoryPlace) -> bool {
        if self.fields.is_empty() || other.fields.is_empty() {
            return false;
        }

        self.fields
            .iter()
            .zip(other.fields.iter())
            .any(|(left, right)| left != right)
    }

    /// Return whether two indexed ranges are statically disjoint.
    fn indexed_offsets_are_disjoint_from(
        &self,
        other: &MemoryPlace,
        size: Option<u64>,
        other_size: Option<u64>,
    ) -> bool {
        if self.indexed_offsets.len() != 1 || other.indexed_offsets.len() != 1 {
            return false;
        }

        let offset = &self.indexed_offsets[0];
        let other_offset = &other.indexed_offsets[0];
        if offset.scale != other_offset.scale {
            return false;
        }

        let (Some(size), Some(other_size)) = (
            size.or(Some(offset.scale)),
            other_size.or(Some(other_offset.scale)),
        ) else {
            return false;
        };

        if offset.index != other_offset.index {
            return false;
        }

        let range = ByteRange::new(self.const_offset, size);
        let other_range = ByteRange::new(other.const_offset, other_size);
        let relation = range.relation(other_range);

        matches!(relation, RangeRelation::Disjoint)
    }
}

/// Builder for resolving memory regions by walking the def chain.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MemoryRegionBuilder<'a> {
    /// Cached region results.
    cache: HashMap<mir::Value, MemoryRegion>,
    /// Value definitions for reference provenance.
    definitions: &'a ValueDefinitions,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// Function parameters for parameter regions.
    parameters: &'a [mir::FunctionParameter],
    /// Value type map for element sizing.
    value_types: &'a ValueTypes,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> MemoryRegionBuilder<'a> {
    /// Create a new region builder.
    pub fn new(
        definitions: &'a ValueDefinitions,
        tree: &'a mir::Tree,
        parameters: &'a [mir::FunctionParameter],
        value_types: &'a ValueTypes,
        target_layout: TargetLayout,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            definitions,
            tree,
            parameters,
            value_types,
            target_layout,
        }
    }

    /// Resolve a reference value to a memory region.
    pub fn region(&mut self, reference: mir::Value) -> MemoryRegion {
        // check cache
        if let Some(cached) = self.cache.get(&reference) {
            return cached.clone();
        }

        let result = self.region_impl(reference);
        self.cache.insert(reference, result.clone());
        result
    }

    /// Resolve a reference value to a memory region.
    fn region_impl(&mut self, reference: mir::Value) -> MemoryRegion {
        // check if it's a parameter
        for (index, parameter) in self.parameters.iter().enumerate() {
            if parameter.value == reference {
                return self.parameter_region(index, parameter);
            }
        }

        // check if it's defined by an instruction
        let Some(instruction_id) = self.definitions.instruction(reference) else {
            return self.any_region(reference);
        };

        let instruction = self.tree.get(instruction_id);

        match instruction {
            // identify fresh allocation storage
            mir::Instruction::FrameAllocZeroed { destination, .. }
            | mir::Instruction::FrameAllocUninit { destination, .. }
                if *destination == reference =>
            {
                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::FrameAllocation(
                    instruction_id,
                )))
            }
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
                if *destination == reference =>
            {
                self.allocation_region(instruction_id, reference)
            }
            mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. }
                if *destination == reference =>
            {
                self.allocation_region(instruction_id, reference)
            }
            mir::Instruction::NewComplete {
                destination, value, ..
            } if *destination == reference => {
                let value = *value;

                self.region(value)
            }

            // identify static storage
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } if *destination == reference => {
                let global_id = *global;
                let global = self.tree.get(global_id);

                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Static {
                    global: global_id,
                    space: global.space,
                }))
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } if *destination == reference => {
                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::LocalSlot(*local)))
            }

            // extend precise region with a field path
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } if *destination == reference => {
                let aggregate = *aggregate;

                let mut region = self.region(aggregate);
                if let MemoryRegion::Place(place) = &mut region {
                    place.add_field(*field);
                }

                region
            }

            // extend precise region with an indexed offset
            mir::Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } if *destination == reference => {
                let base = *base;
                let index = *index;

                let mut region = self.region(base);

                let scale = self.expect_element_size(base);
                if let MemoryRegion::Place(place) = &mut region {
                    place.add_indexed_offset(index, scale);
                }

                region
            }

            // casts preserve provenance
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            } if *destination == reference => {
                let argument = *argument;

                self.region(argument)
            }

            // anything else is imprecise
            _ => self.any_region(reference),
        }
    }

    /// Return a region for a parameter reference.
    fn parameter_region(&self, index: usize, parameter: &mir::FunctionParameter) -> MemoryRegion {
        let ty = self.tree.get(parameter.ty);

        match ty {
            mir::Type::Reference {
                kind,
                space,
                access,
                ..
            }
            | mir::Type::Slice {
                kind,
                space,
                access,
                ..
            }
            | mir::Type::TensorView {
                kind,
                space,
                access,
                ..
            } => MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Parameter {
                index: index as u32,
                space: *space,
                kind: *kind,
                access: *access,
            })),
            _ => MemoryRegion::any(),
        }
    }

    /// Return a region for a heap allocation.
    fn allocation_region(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        reference: mir::Value,
    ) -> MemoryRegion {
        let ty_id = self.value_type(reference);
        let ty = self.tree.get(ty_id);

        match ty {
            mir::Type::Reference { kind, space, .. }
            | mir::Type::Slice { kind, space, .. }
            | mir::Type::TensorView { kind, space, .. } => {
                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Allocation {
                    instruction,
                    space: *space,
                    kind: *kind,
                }))
            }
            _ => MemoryRegion::any(),
        }
    }

    /// Return an imprecise region bounded by a reference type when possible.
    fn any_region(&self, reference: mir::Value) -> MemoryRegion {
        let ty_id = self.value_type(reference);
        let ty = self.tree.get(ty_id);

        match ty {
            mir::Type::Reference { space, .. }
            | mir::Type::Slice { space, .. }
            | mir::Type::TensorView { space, .. } => MemoryRegion::any_space(*space),
            _ => MemoryRegion::any(),
        }
    }

    /// Return the value type for an SSA value.
    fn value_type(&self, value: mir::Value) -> mir::LocalNodeId<mir::Type> {
        self.value_types.expect_value_type(value)
    }

    /// Return the byte stride for one indexed value.
    fn expect_element_size(&self, array: mir::Value) -> u64 {
        let ty_id = self.value_type(array);
        let element_id = match self.tree.get(ty_id) {
            mir::Type::FixedArray { element, .. }
            | mir::Type::Slice { element, .. }
            | mir::Type::Tensor { element, .. }
            | mir::Type::TensorView { element, .. } => *element,
            mir::Type::Reference { pointee, .. } => self.expect_pointee_element(*pointee),
            _ => panic!("element.address requires an indexed value, got {ty_id:?}"),
        };

        let key = TypeKey::from_type(element_id, self.tree);
        match key.byte_size(self.target_layout.pointer_bits()) {
            Some(size) if size > 0 => size,
            _ => panic!("element.address requires a byte-sized element, got {element_id:?}"),
        }
    }

    /// Return the element type for an indexed pointee.
    fn expect_pointee_element(&self, pointee: mir::TypeId) -> mir::TypeId {
        match self.tree.get(pointee) {
            mir::Type::FixedArray { element, .. }
            | mir::Type::Slice { element, .. }
            | mir::Type::Tensor { element, .. }
            | mir::Type::TensorView { element, .. } => *element,
            _ => panic!("element.address requires an indexed pointee, got {pointee:?}"),
        }
    }
}

/// One half-open byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    /// Start offset in bytes.
    pub offset: i64,
    /// Size in bytes.
    pub size: u64,
}

impl ByteRange {
    /// Create a byte range.
    pub fn new(offset: i64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Return the exclusive end offset.
    pub fn end(self) -> i64 {
        self.offset.saturating_add(self.size as i64)
    }

    /// Return whether this range overlaps another range.
    pub fn overlaps(self, other: ByteRange) -> bool {
        let end = self.end();
        let other_end = other.end();

        !(end <= other.offset || other_end <= self.offset)
    }

    /// Return whether this range exactly equals another range.
    pub fn equals(self, other: ByteRange) -> bool {
        self.offset == other.offset && self.size == other.size
    }

    /// Return the relationship between this range and another range.
    pub fn relation(self, other: ByteRange) -> RangeRelation {
        let end = self.end();
        let other_end = other.end();

        // disjoint
        if end <= other.offset || other_end <= self.offset {
            return RangeRelation::Disjoint;
        }

        // equal
        if self.equals(other) {
            return RangeRelation::Equal;
        }

        // containment
        if self.offset <= other.offset && end >= other_end {
            return RangeRelation::Contains;
        }
        if other.offset <= self.offset && other_end >= end {
            return RangeRelation::ContainedBy;
        }

        RangeRelation::Overlaps
    }
}

/// Relationship between two byte ranges.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Ranges that overlap are detected as overlapping.
    #[test]
    fn test_ranges_overlap() {
        assert!(!ByteRange::new(0, 4).overlaps(ByteRange::new(10, 4)));
        assert!(!ByteRange::new(10, 4).overlaps(ByteRange::new(0, 4)));

        assert!(!ByteRange::new(0, 4).overlaps(ByteRange::new(4, 4)));

        assert!(ByteRange::new(0, 8).overlaps(ByteRange::new(4, 8)));
        assert!(ByteRange::new(4, 8).overlaps(ByteRange::new(0, 8)));

        assert!(ByteRange::new(0, 16).overlaps(ByteRange::new(4, 4)));
        assert!(ByteRange::new(4, 4).overlaps(ByteRange::new(0, 16)));

        assert!(ByteRange::new(0, 8).overlaps(ByteRange::new(0, 8)));
    }

    /// Range relationships return the expected classification.
    #[test]
    fn test_range_relation() {
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(10, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(0, 8)),
            RangeRelation::Equal
        );
        assert_eq!(
            ByteRange::new(0, 16).relation(ByteRange::new(4, 4)),
            RangeRelation::Contains
        );
        assert_eq!(
            ByteRange::new(4, 4).relation(ByteRange::new(0, 16)),
            RangeRelation::ContainedBy
        );
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(4, 8)),
            RangeRelation::Overlaps
        );
    }

    /// Storage roots expose their memory spaces.
    #[test]
    fn test_storage_spaces() {
        let stack = StorageRoot::FrameAllocation(mir::LocalNodeId::new(0));
        let local = StorageRoot::LocalSlot(mir::LocalNodeId::new(0));
        let allocation = StorageRoot::Allocation {
            instruction: mir::LocalNodeId::new(1),
            space: mir::Space::Shared,
            kind: mir::ReferenceKind::Managed,
        };
        let parameter = StorageRoot::Parameter {
            index: 0,
            space: mir::Space::Local,
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Mutable,
        };

        assert_eq!(stack.spaces(), mir::StorageSet::FRAME);
        assert_eq!(local.spaces(), mir::StorageSet::FRAME);
        assert_eq!(allocation.spaces(), mir::StorageSet::SHARED);
        assert_eq!(parameter.spaces(), mir::StorageSet::LOCAL);
    }

    /// Memory places track constant and indexed offsets.
    #[test]
    fn test_memory_place_const_offset() {
        let mut place =
            MemoryPlace::from_root(StorageRoot::FrameAllocation(mir::LocalNodeId::new(0)));
        assert!(place.is_constant_offset());

        place.add_const_offset(16);
        assert!(place.is_constant_offset());
        assert_eq!(place.const_offset, 16);

        place.add_indexed_offset(mir::Value::new(0), 4);
        assert!(!place.is_constant_offset());
    }

    /// Exclusive parameters are reported as exclusive.
    #[test]
    fn test_storage_is_exclusive_parameter() {
        let exclusive_parameter = StorageRoot::Parameter {
            index: 0,
            space: mir::Space::Local,
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Exclusive,
        };
        let mutable_parameter = StorageRoot::Parameter {
            index: 1,
            space: mir::Space::Local,
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Mutable,
        };
        let stack = StorageRoot::FrameAllocation(mir::LocalNodeId::new(0));

        assert!(exclusive_parameter.is_exclusive_parameter());
        assert!(!mutable_parameter.is_exclusive_parameter());
        assert!(!stack.is_exclusive_parameter());
    }

    /// Field paths are captured by memory places.
    #[test]
    fn test_memory_place_fields() {
        let mut place =
            MemoryPlace::from_root(StorageRoot::FrameAllocation(mir::LocalNodeId::new(0)));
        assert!(place.fields.is_empty());

        place.add_field(0);
        assert_eq!(place.fields, vec![0]);

        place.add_field(2);
        assert_eq!(place.fields, vec![0, 2]);

        assert!(place.is_constant_offset());
    }

    /// Multiple indexed offsets are tracked.
    #[test]
    fn test_memory_place_multiple_indexed_offsets() {
        let mut place = MemoryPlace::from_root(StorageRoot::Allocation {
            instruction: mir::LocalNodeId::new(0),
            space: mir::Space::Local,
            kind: mir::ReferenceKind::Unique,
        });

        place.add_indexed_offset(mir::Value::new(1), 4);
        place.add_indexed_offset(mir::Value::new(2), 8);

        assert!(!place.is_constant_offset());
        assert_eq!(place.indexed_offsets.len(), 2);
        assert_eq!(place.indexed_offsets[0].index, mir::Value::new(1));
        assert_eq!(place.indexed_offsets[0].scale, 4);
        assert_eq!(place.indexed_offsets[1].index, mir::Value::new(2));
        assert_eq!(place.indexed_offsets[1].scale, 8);
    }

    /// Constant offsets accumulate.
    #[test]
    fn test_memory_place_const_offset_accumulation() {
        let mut place = MemoryPlace::from_root(StorageRoot::Static {
            global: mir::LocalNodeId::new(0),
            space: mir::Space::Static,
        });

        place.add_const_offset(8);
        place.add_const_offset(16);

        assert_eq!(place.const_offset, 24);
    }

    /// Negative offsets are handled consistently.
    #[test]
    fn test_memory_place_negative_offset() {
        let mut place =
            MemoryPlace::from_root(StorageRoot::FrameAllocation(mir::LocalNodeId::new(0)));

        place.add_const_offset(-8);
        assert_eq!(place.const_offset, -8);

        place.add_const_offset(4);
        assert_eq!(place.const_offset, -4);
    }

    /// Equal ranges are detected.
    #[test]
    fn test_ranges_equal() {
        assert!(ByteRange::new(0, 4).equals(ByteRange::new(0, 4)));
        assert!(!ByteRange::new(0, 4).equals(ByteRange::new(0, 8)));
        assert!(!ByteRange::new(0, 4).equals(ByteRange::new(4, 4)));
        assert!(!ByteRange::new(0, 8).equals(ByteRange::new(4, 8)));
    }

    /// Adjacent ranges are disjoint.
    #[test]
    fn test_range_relation_adjacent() {
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(4, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(4, 4).relation(ByteRange::new(0, 4)),
            RangeRelation::Disjoint
        );
    }

    /// Partial overlaps are classified correctly.
    #[test]
    fn test_range_relation_partial_overlap() {
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(4, 8)),
            RangeRelation::Overlaps
        );
        assert_eq!(
            ByteRange::new(4, 8).relation(ByteRange::new(0, 8)),
            RangeRelation::Overlaps
        );
    }

    /// Zero sized ranges use half open range semantics.
    #[test]
    fn test_range_relation_zero_size() {
        assert_eq!(
            ByteRange::new(0, 0).relation(ByteRange::new(0, 0)),
            RangeRelation::Disjoint
        );

        assert_eq!(
            ByteRange::new(0, 0).relation(ByteRange::new(0, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(0, 0)),
            RangeRelation::Disjoint
        );

        assert_eq!(
            ByteRange::new(2, 0).relation(ByteRange::new(0, 8)),
            RangeRelation::ContainedBy
        );

        // handle zero size after the end
        assert_eq!(
            ByteRange::new(10, 0).relation(ByteRange::new(0, 8)),
            RangeRelation::Disjoint
        );
    }
}
