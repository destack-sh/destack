use std::collections::HashMap;

use destack_mir as mir;

use crate::optimize::analyses::OwnershipAnalysis;

use super::TypeKey;

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
}

impl MemoryLocation {
    /// Create a location from just a pointer (unknown size).
    pub fn from_ptr(ptr: mir::Value) -> Self {
        Self {
            ptr,
            size: None,
            access_type: None,
        }
    }

    /// Create a location with known size.
    pub fn with_size(ptr: mir::Value, size: u64) -> Self {
        Self {
            ptr,
            size: Some(size),
            access_type: None,
        }
    }

    /// Create a location with type information.
    pub fn with_type(ptr: mir::Value, access_type: TypeKey) -> Self {
        Self {
            ptr,
            size: None,
            access_type: Some(access_type),
        }
    }

    /// Create a fully specified location.
    pub fn new(ptr: mir::Value, size: Option<u64>, access_type: Option<TypeKey>) -> Self {
        Self {
            ptr,
            size,
            access_type,
        }
    }
}

/// Resolve a pointer's pointee type when it is statically known.
pub fn resolve_pointer_pointee_type(
    pointer: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
    ownership: &OwnershipAnalysis,
    definitions: &HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // check parameter types first
    if let Some(param_type) = parameter_type(pointer, function, tree)
        && let mir::Type::Reference { pointee, .. } = tree.get(param_type)
    {
        return Some(*pointee);
    }

    // check ownership-derived value types
    if let Some(ty_id) = ownership.value_type(pointer)
        && let mir::Type::Reference { pointee, .. } = tree.get(ty_id)
    {
        return Some(*pointee);
    }

    // resolve from defining instruction
    let instruction_id = definitions.get(&pointer)?;
    let instruction = tree.get(*instruction_id);

    match instruction {
        mir::Instruction::StackAlloc { layout, .. }
        | mir::Instruction::RawAlloc { layout, .. }
        | mir::Instruction::ManagedAlloc { layout, .. } => Some(*layout),
        mir::Instruction::ManagedAllocArray { element, .. } => Some(*element),
        mir::Instruction::GlobalAddr { global, .. } => {
            let global_def = tree.get(*global);
            Some(global_def.ty)
        }
        mir::Instruction::FieldAddr {
            aggregate, index, ..
        } => {
            let aggregate_type = ownership.value_type(*aggregate)?;
            match tree.get(aggregate_type) {
                mir::Type::Struct { fields, .. } => fields
                    .get(*index as usize)
                    .map(|field_id| tree.get(*field_id).ty),
                mir::Type::Tuple { elements, .. } => elements.get(*index as usize).copied(),
                _ => None,
            }
        }
        mir::Instruction::ElementAddr { array, .. } => {
            let array_type = ownership.value_type(*array)?;
            match tree.get(array_type) {
                mir::Type::Array { element, .. } => Some(*element),
                _ => None,
            }
        }
        mir::Instruction::Cast { to_type, .. } => {
            let ty = tree.get(*to_type);
            if let mir::Type::Reference { pointee, .. } = ty {
                Some(*pointee)
            } else {
                None
            }
        }
        mir::Instruction::Call { function, .. } => {
            let callee = tree.get(*function);
            let ty = tree.get(callee.return_type);
            if let mir::Type::Reference { pointee, .. } = ty {
                Some(*pointee)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Resolve the declared type for a value defined as a parameter.
fn parameter_type(
    value: mir::Value,
    function: &mir::Function,
    tree: &mir::NodeTree,
) -> Option<mir::LocalNodeId<mir::Type>> {
    // search function parameters first
    for param in &function.parameters {
        if param.value == value {
            return Some(param.ty);
        }
    }

    // search block parameters next
    for &block_id in &function.blocks {
        let block = tree.get(block_id);
        for param in &block.parameters {
            if param.value == value {
                return Some(param.ty);
            }
        }
    }

    None
}

/// Base object that a pointer ultimately derives from.
///
/// Pointers with different identified bases cannot alias.
/// This is the foundation of provenance-based alias analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerBase {
    /// Stack allocation instruction.
    StackAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Managed heap allocation instruction.
    ManagedAlloc(mir::LocalNodeId<mir::Instruction>),
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
                | PointerBase::ManagedAlloc(_)
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
            PointerBase::StackAlloc(_) | PointerBase::ManagedAlloc(_) | PointerBase::RawAlloc(_)
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
    /// The MIR node tree.
    tree: &'a mir::NodeTree,
    /// Function parameters for noalias checking.
    parameters: &'a [mir::TypedValue],
    /// Whether strict borrow mode is enabled.
    strict_borrow_mode: bool,
}

impl<'a> PointerDecomposer<'a> {
    /// Create a new decomposer.
    pub fn new(
        constants: &'a HashMap<mir::Value, i64>,
        definitions: &'a HashMap<mir::Value, mir::LocalNodeId<mir::Instruction>>,
        tree: &'a mir::NodeTree,
        parameters: &'a [mir::TypedValue],
        strict_borrow_mode: bool,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            constants,
            definitions,
            tree,
            parameters,
            strict_borrow_mode,
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
            if parameter.value == ptr {
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
            mir::Instruction::StackAlloc { destination, .. } if *destination == ptr => {
                DecomposedPointer::from_base(PointerBase::StackAlloc(instruction_id))
            }
            mir::Instruction::ManagedAlloc { destination, .. } if *destination == ptr => {
                DecomposedPointer::from_base(PointerBase::ManagedAlloc(instruction_id))
            }
            mir::Instruction::ManagedAllocArray { destination, .. } if *destination == ptr => {
                DecomposedPointer::from_base(PointerBase::ManagedAlloc(instruction_id))
            }
            mir::Instruction::RawAlloc { destination, .. } if *destination == ptr => {
                DecomposedPointer::from_base(PointerBase::RawAlloc(instruction_id))
            }

            // global address is a base
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } if *destination == ptr => DecomposedPointer::from_base(PointerBase::Global(*global)),

            // field address: decompose base and add field offset
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
            } if *destination == ptr => {
                let mut base_decomp = self.decompose(*aggregate);
                base_decomp.add_field(*index);
                base_decomp
            }

            // element address: decompose base and add index offset
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
            } if *destination == ptr => {
                let mut base_decomp = self.decompose(*array);

                // NOTE: add variable offset with scale=1 (element size unknown without layout info)
                // we can still prove NoAlias when indices are provably different constants,
                // but we can't reason about partial overlaps (yet, #Incomplete)
                base_decomp.add_var_offset(*index, 1);
                base_decomp
            }

            // casts preserve provenance
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            } if *destination == ptr => self.decompose(*argument),

            // calls return unknown pointers
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. }
                if destination.is_some_and(|d| d == ptr) =>
            {
                DecomposedPointer::from_base(PointerBase::CallResult(instruction_id))
            }

            // loads produce unknown pointers
            mir::Instruction::Load { destination, .. } if *destination == ptr => {
                DecomposedPointer::from_base(PointerBase::Unknown)
            }

            // anything else is unknown
            _ => DecomposedPointer::from_base(PointerBase::Unknown),
        }
    }

    /// Check if a parameter has noalias semantics.
    fn is_parameter_noalias(&self, parameter: &mir::TypedValue) -> bool {
        // in strict borrow mode, &mut T parameters are noalias
        if self.strict_borrow_mode {
            let ty = self.tree.get(parameter.ty);
            matches!(
                ty,
                mir::Type::Reference {
                    kind: mir::ReferenceKind::Borrowed,
                    mutability: mir::Mutability::Mutable,
                    ..
                }
            )
        } else {
            false
        }
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

    #[test]
    fn test_ranges_overlap() {
        // disjoint
        assert!(!ranges_overlap(0, 4, 10, 4));
        assert!(!ranges_overlap(10, 4, 0, 4));

        // adjacent (not overlapping)
        assert!(!ranges_overlap(0, 4, 4, 4));

        // overlapping
        assert!(ranges_overlap(0, 8, 4, 8));
        assert!(ranges_overlap(4, 8, 0, 8));

        // contained
        assert!(ranges_overlap(0, 16, 4, 4));
        assert!(ranges_overlap(4, 4, 0, 16));

        // equal
        assert!(ranges_overlap(0, 8, 0, 8));
    }

    #[test]
    fn test_range_relation() {
        assert_eq!(range_relation(0, 4, 10, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(0, 8, 0, 8), RangeRelation::Equal);
        assert_eq!(range_relation(0, 16, 4, 4), RangeRelation::Contains);
        assert_eq!(range_relation(4, 4, 0, 16), RangeRelation::ContainedBy);
        assert_eq!(range_relation(0, 8, 4, 8), RangeRelation::Overlaps);
    }

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

    #[test]
    fn test_memory_location_constructors() {
        // from_ptr: unknown size, no type
        let loc1 = MemoryLocation::from_ptr(mir::Value::new(0));
        assert_eq!(loc1.ptr, mir::Value::new(0));
        assert!(loc1.size.is_none());
        assert!(loc1.access_type.is_none());

        // with_size: known size, no type
        let loc2 = MemoryLocation::with_size(mir::Value::new(1), 8);
        assert_eq!(loc2.ptr, mir::Value::new(1));
        assert_eq!(loc2.size, Some(8));
        assert!(loc2.access_type.is_none());

        // with_type: no size, has type
        let ty = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let loc3 = MemoryLocation::with_type(mir::Value::new(2), ty.clone());
        assert_eq!(loc3.ptr, mir::Value::new(2));
        assert!(loc3.size.is_none());
        assert_eq!(loc3.access_type, Some(ty.clone()));

        // new: fully specified
        let loc4 = MemoryLocation::new(mir::Value::new(3), Some(4), Some(ty.clone()));
        assert_eq!(loc4.ptr, mir::Value::new(3));
        assert_eq!(loc4.size, Some(4));
        assert_eq!(loc4.access_type, Some(ty));
    }

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

    #[test]
    fn test_pointer_base_is_local_alloc() {
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));
        let managed = PointerBase::ManagedAlloc(mir::LocalNodeId::new(1));
        let raw = PointerBase::RawAlloc(mir::LocalNodeId::new(2));
        let global = PointerBase::Global(mir::LocalNodeId::new(0));
        let param = PointerBase::Parameter {
            index: 0,
            noalias: false,
        };
        let call = PointerBase::CallResult(mir::LocalNodeId::new(3));
        let unknown = PointerBase::Unknown;

        assert!(stack.is_local_alloc());
        assert!(managed.is_local_alloc());
        assert!(raw.is_local_alloc());
        assert!(!global.is_local_alloc());
        assert!(!param.is_local_alloc());
        assert!(!call.is_local_alloc());
        assert!(!unknown.is_local_alloc());
    }

    #[test]
    fn test_pointer_base_all_variants_identified() {
        // stack, managed, raw, and global are identified
        let stack = PointerBase::StackAlloc(mir::LocalNodeId::new(0));
        let managed = PointerBase::ManagedAlloc(mir::LocalNodeId::new(1));
        let raw = PointerBase::RawAlloc(mir::LocalNodeId::new(2));
        let global = PointerBase::Global(mir::LocalNodeId::new(0));

        assert!(stack.is_identified());
        assert!(managed.is_identified());
        assert!(raw.is_identified());
        assert!(global.is_identified());

        // param, call result, and unknown are not identified
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

    #[test]
    fn test_decomposed_pointer_field_path() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::StackAlloc(mir::LocalNodeId::new(0)));
        assert!(ptr.field_path.is_empty());

        ptr.add_field(0);
        assert_eq!(ptr.field_path, vec![0]);

        ptr.add_field(2);
        assert_eq!(ptr.field_path, vec![0, 2]);

        // field access doesn't affect const offset
        assert!(ptr.is_constant_offset());
    }

    #[test]
    fn test_decomposed_pointer_multiple_var_offsets() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::ManagedAlloc(mir::LocalNodeId::new(0)));

        ptr.add_var_offset(mir::Value::new(1), 4); // index * 4 bytes
        ptr.add_var_offset(mir::Value::new(2), 8); // another index * 8 bytes

        assert!(!ptr.is_constant_offset());
        assert_eq!(ptr.var_offsets.len(), 2);
        assert_eq!(ptr.var_offsets[0].index, mir::Value::new(1));
        assert_eq!(ptr.var_offsets[0].scale, 4);
        assert_eq!(ptr.var_offsets[1].index, mir::Value::new(2));
        assert_eq!(ptr.var_offsets[1].scale, 8);
    }

    #[test]
    fn test_decomposed_pointer_const_offset_accumulation() {
        let mut ptr = DecomposedPointer::from_base(PointerBase::Global(mir::LocalNodeId::new(0)));

        ptr.add_const_offset(8);
        ptr.add_const_offset(16);

        assert_eq!(ptr.const_offset, 24);
    }

    #[test]
    fn test_decomposed_pointer_negative_offset() {
        let mut ptr =
            DecomposedPointer::from_base(PointerBase::StackAlloc(mir::LocalNodeId::new(0)));

        ptr.add_const_offset(-8);
        assert_eq!(ptr.const_offset, -8);

        ptr.add_const_offset(4);
        assert_eq!(ptr.const_offset, -4);
    }

    #[test]
    fn test_ranges_equal() {
        assert!(ranges_equal(0, 4, 0, 4));
        assert!(!ranges_equal(0, 4, 0, 8));
        assert!(!ranges_equal(0, 4, 4, 4));
        assert!(!ranges_equal(0, 8, 4, 8));
    }

    #[test]
    fn test_range_relation_adjacent() {
        // adjacent ranges are disjoint, not overlapping
        assert_eq!(range_relation(0, 4, 4, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(4, 4, 0, 4), RangeRelation::Disjoint);
    }

    #[test]
    fn test_range_relation_partial_overlap() {
        // partial overlap from left
        assert_eq!(range_relation(0, 8, 4, 8), RangeRelation::Overlaps);
        // partial overlap from right
        assert_eq!(range_relation(4, 8, 0, 8), RangeRelation::Overlaps);
    }

    #[test]
    fn test_range_relation_zero_size() {
        // two zero-size ranges at same position are disjoint (empty sets don't overlap)
        assert_eq!(range_relation(0, 0, 0, 0), RangeRelation::Disjoint);

        // zero-size range at start of non-zero range: disjoint
        // (end1=0 <= off2=0, so considered disjoint)
        assert_eq!(range_relation(0, 0, 0, 4), RangeRelation::Disjoint);
        assert_eq!(range_relation(0, 4, 0, 0), RangeRelation::Disjoint);

        // zero-size range inside a non-zero range: contained by it
        // (position 2 is within [0, 8), so it's considered contained)
        assert_eq!(range_relation(2, 0, 0, 8), RangeRelation::ContainedBy);

        // zero-size range after the end: disjoint
        assert_eq!(range_relation(10, 0, 0, 8), RangeRelation::Disjoint);
    }
}
