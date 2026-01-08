use std::collections::HashMap;
use std::sync::Arc;

use destack_mir as mir;

use crate::optimize::common::TypeKey;
use crate::optimize::{Analysis, AnalysisCache, AnalysisKind};

/// Result of an alias query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasResult {
    /// The two locations definitely refer to the same memory.
    MustAlias,
    /// The two locations might refer to the same memory (conservative).
    MayAlias,
    /// The two locations definitely do not overlap.
    NoAlias,
}

impl AliasResult {
    /// Returns true if the locations definitely don't alias.
    pub fn is_no_alias(self) -> bool {
        matches!(self, AliasResult::NoAlias)
    }

    /// Returns true if the locations might alias.
    pub fn may_alias(self) -> bool {
        !matches!(self, AliasResult::NoAlias)
    }

    /// Merge two alias results (most conservative wins).
    pub fn merge(self, other: AliasResult) -> AliasResult {
        match (self, other) {
            (AliasResult::NoAlias, AliasResult::NoAlias) => AliasResult::NoAlias,
            (AliasResult::MustAlias, AliasResult::MustAlias) => AliasResult::MustAlias,
            _ => AliasResult::MayAlias,
        }
    }
}

/// A memory location being accessed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// The base pointer value.
    pub base: mir::Value,
    /// Known constant offset from base in bytes.
    pub offset: Option<i64>,
    /// Field path from base for struct accesses.
    pub field_path: Vec<u32>,
    /// The type being accessed.
    pub pointee_type: Option<mir::LocalNodeId<mir::Type>>,
}

impl MemoryLocation {
    /// Create a simple memory location from a pointer value.
    pub fn from_pointer(pointer: mir::Value) -> Self {
        Self {
            base: pointer,
            offset: None,
            field_path: Vec::new(),
            pointee_type: None,
        }
    }

    /// Create a memory location with type information.
    pub fn with_type(pointer: mir::Value, pointee_type: mir::LocalNodeId<mir::Type>) -> Self {
        Self {
            base: pointer,
            offset: None,
            field_path: Vec::new(),
            pointee_type: Some(pointee_type),
        }
    }

    /// Create a memory location for a field access.
    pub fn field(
        base: mir::Value,
        field_index: u32,
        pointee_type: Option<mir::LocalNodeId<mir::Type>>,
    ) -> Self {
        Self {
            base,
            offset: None,
            field_path: vec![field_index],
            pointee_type,
        }
    }
}

/// Source of a pointer value for provenance tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PointerSource {
    /// Allocated via ManagedAlloc.
    ManagedAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Allocated via RawAlloc.
    RawAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Allocated via StackAlloc.
    StackAlloc(mir::LocalNodeId<mir::Instruction>),
    /// Address of a global variable.
    GlobalAddr(mir::LocalNodeId<mir::Global>),
    /// Function parameter with unknown provenance.
    Parameter(u32),
    /// Result of a function call with unknown provenance.
    CallResult,
    /// Derived from another pointer via FieldAddr.
    FieldAddr {
        /// The base pointer.
        base: mir::Value,
        /// The field index.
        field_index: u32,
    },
    /// Derived from another pointer via ElementAddr.
    ElementAddr {
        /// The base pointer.
        base: mir::Value,
        /// The constant index if known.
        index: Option<u64>,
    },
    /// Cast from another pointer.
    Cast(mir::Value),
    /// Unknown source (conservative fallback).
    Unknown,
}

/// Alias analysis for a function.
///
/// Determines whether two memory locations may alias using pointer provenance
/// tracking, field/element offset analysis, and type-based alias analysis.
#[derive(Debug)]
pub struct AliasAnalysis {
    /// Map from value to its pointer source.
    pointer_sources: HashMap<mir::Value, PointerSource>,
    /// Map from value to its structural type (for TBAA).
    value_types: HashMap<mir::Value, TypeKey>,
    /// Map from value to constant integer value (used during construction).
    #[allow(dead_code)]
    constants: HashMap<mir::Value, i64>,
}

impl AliasAnalysis {
    /// Build alias analysis for a function.
    fn build(function: &mir::Function, tree: &mir::NodeTree) -> Self {
        let mut pointer_sources = HashMap::new();
        let mut value_types = HashMap::new();
        let mut constants = HashMap::new();

        // imports have no body
        if function.entry.is_none() {
            return Self {
                pointer_sources,
                value_types,
                constants,
            };
        }

        // parameters have unknown provenance
        for (param_idx, param) in function.parameters.iter().enumerate() {
            pointer_sources.insert(param.value, PointerSource::Parameter(param_idx as u32));
            let type_key = TypeKey::from_type(tree.get(param.ty), tree);
            value_types.insert(param.value, type_key);
        }

        // analyze all instructions
        for &block_id in &function.blocks {
            let block = tree.get(block_id);
            for &inst_id in &block.instructions {
                let inst = tree.get(inst_id);
                Self::analyze_instruction(
                    inst_id,
                    inst,
                    tree,
                    &mut pointer_sources,
                    &mut value_types,
                    &mut constants,
                );
            }
        }

        Self {
            pointer_sources,
            value_types,
            constants,
        }
    }

    /// Analyze a single instruction for pointer sources, types, and constants.
    fn analyze_instruction(
        inst_id: mir::LocalNodeId<mir::Instruction>,
        inst: &mir::Instruction,
        tree: &mir::NodeTree,
        pointer_sources: &mut HashMap<mir::Value, PointerSource>,
        value_types: &mut HashMap<mir::Value, TypeKey>,
        constants: &mut HashMap<mir::Value, i64>,
    ) {
        match inst {
            // constants: track integer values for index lookup
            mir::Instruction::Const {
                destination,
                value: constant,
            } => {
                if let mir::Constant::Int { value, .. } = constant {
                    constants.insert(*destination, *value);
                }
            }

            // allocations create new unique memory
            mir::Instruction::ManagedAlloc {
                destination,
                layout,
            } => {
                pointer_sources.insert(*destination, PointerSource::ManagedAlloc(inst_id));
                let type_key = TypeKey::from_type(tree.get(*layout), tree);
                value_types.insert(*destination, type_key);
            }
            mir::Instruction::ManagedAllocArray {
                destination,
                element,
                ..
            } => {
                pointer_sources.insert(*destination, PointerSource::ManagedAlloc(inst_id));
                let type_key = TypeKey::from_type(tree.get(*element), tree);
                value_types.insert(*destination, type_key);
            }
            mir::Instruction::RawAlloc {
                destination,
                layout,
            } => {
                pointer_sources.insert(*destination, PointerSource::RawAlloc(inst_id));
                let type_key = TypeKey::from_type(tree.get(*layout), tree);
                value_types.insert(*destination, type_key);
            }
            mir::Instruction::StackAlloc {
                destination,
                layout,
            } => {
                pointer_sources.insert(*destination, PointerSource::StackAlloc(inst_id));
                let type_key = TypeKey::from_type(tree.get(*layout), tree);
                value_types.insert(*destination, type_key);
            }

            // global address
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } => {
                pointer_sources.insert(*destination, PointerSource::GlobalAddr(*global));
            }

            // field address derives from base
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                index,
            } => {
                pointer_sources.insert(
                    *destination,
                    PointerSource::FieldAddr {
                        base: *aggregate,
                        field_index: *index,
                    },
                );
            }

            // element address derives from base
            mir::Instruction::ElementAddr {
                destination,
                array,
                index,
            } => {
                pointer_sources.insert(
                    *destination,
                    PointerSource::ElementAddr {
                        base: *array,
                        index: constants.get(index).map(|&v| v as u64),
                    },
                );
            }

            // casts preserve provenance
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            } => {
                pointer_sources.insert(*destination, PointerSource::Cast(*argument));
            }

            // calls return unknown pointers
            mir::Instruction::Call { destination, .. }
            | mir::Instruction::CallIndirect { destination, .. } => {
                if let Some(dest) = destination {
                    pointer_sources.insert(*dest, PointerSource::CallResult);
                }
            }

            // loads have unknown provenance
            mir::Instruction::Load { destination, .. } => {
                pointer_sources.insert(*destination, PointerSource::Unknown);
            }

            _ => {}
        }
    }

    /// Get the source of a pointer value.
    pub fn get_pointer_source(&self, value: mir::Value) -> &PointerSource {
        self.pointer_sources
            .get(&value)
            .unwrap_or(&PointerSource::Unknown)
    }

    /// Get the underlying allocation for a pointer, following derivations.
    pub fn get_underlying_object(&self, value: mir::Value) -> PointerSource {
        let mut current = value;
        let mut visited = std::collections::HashSet::new();

        loop {
            // cycle detection
            if !visited.insert(current) {
                return PointerSource::Unknown;
            }

            match self.pointer_sources.get(&current) {
                // follow derivations
                Some(PointerSource::FieldAddr { base, .. }) => current = *base,
                Some(PointerSource::ElementAddr { base, .. }) => current = *base,
                Some(PointerSource::Cast(base)) => current = *base,
                // found base allocation
                Some(source) => return source.clone(),
                None => return PointerSource::Unknown,
            }
        }
    }

    /// Query whether two memory locations alias.
    pub fn alias(&self, loc1: &MemoryLocation, loc2: &MemoryLocation) -> AliasResult {
        // same base with same field path: must alias
        if loc1.base == loc2.base && loc1.field_path == loc2.field_path {
            return AliasResult::MustAlias;
        }

        // check underlying objects
        let obj1 = self.get_underlying_object(loc1.base);
        let obj2 = self.get_underlying_object(loc2.base);

        // different allocations cannot alias
        if self.different_allocations(&obj1, &obj2) {
            return AliasResult::NoAlias;
        }

        // same allocation but different field paths cannot alias
        if self.same_allocation(&obj1, &obj2) {
            if let Some(result) = self.check_field_aliasing(loc1, loc2) {
                return result;
            }
        }

        // type-based alias analysis using stored TypeKeys
        let ty1 = self.value_types.get(&loc1.base);
        let ty2 = self.value_types.get(&loc2.base);
        if let (Some(t1), Some(t2)) = (ty1, ty2) {
            if self.types_cannot_alias(t1, t2) {
                return AliasResult::NoAlias;
            }
        }

        AliasResult::MayAlias
    }

    /// Check if two pointer sources represent different allocations.
    fn different_allocations(&self, obj1: &PointerSource, obj2: &PointerSource) -> bool {
        match (obj1, obj2) {
            // different allocation instructions
            (PointerSource::ManagedAlloc(a), PointerSource::ManagedAlloc(b)) => a != b,
            (PointerSource::RawAlloc(a), PointerSource::RawAlloc(b)) => a != b,
            (PointerSource::StackAlloc(a), PointerSource::StackAlloc(b)) => a != b,

            // different allocation types
            (PointerSource::ManagedAlloc(_), PointerSource::RawAlloc(_))
            | (PointerSource::RawAlloc(_), PointerSource::ManagedAlloc(_))
            | (PointerSource::ManagedAlloc(_), PointerSource::StackAlloc(_))
            | (PointerSource::StackAlloc(_), PointerSource::ManagedAlloc(_))
            | (PointerSource::RawAlloc(_), PointerSource::StackAlloc(_))
            | (PointerSource::StackAlloc(_), PointerSource::RawAlloc(_)) => true,

            // different globals
            (PointerSource::GlobalAddr(a), PointerSource::GlobalAddr(b)) => a != b,

            // globals vs allocations
            (PointerSource::GlobalAddr(_), PointerSource::ManagedAlloc(_))
            | (PointerSource::ManagedAlloc(_), PointerSource::GlobalAddr(_))
            | (PointerSource::GlobalAddr(_), PointerSource::RawAlloc(_))
            | (PointerSource::RawAlloc(_), PointerSource::GlobalAddr(_))
            | (PointerSource::GlobalAddr(_), PointerSource::StackAlloc(_))
            | (PointerSource::StackAlloc(_), PointerSource::GlobalAddr(_)) => true,

            _ => false,
        }
    }

    /// Check if two pointer sources represent the same allocation.
    fn same_allocation(&self, obj1: &PointerSource, obj2: &PointerSource) -> bool {
        match (obj1, obj2) {
            (PointerSource::ManagedAlloc(a), PointerSource::ManagedAlloc(b)) => a == b,
            (PointerSource::RawAlloc(a), PointerSource::RawAlloc(b)) => a == b,
            (PointerSource::StackAlloc(a), PointerSource::StackAlloc(b)) => a == b,
            (PointerSource::GlobalAddr(a), PointerSource::GlobalAddr(b)) => a == b,
            _ => false,
        }
    }

    /// Check field-based aliasing within the same allocation.
    fn check_field_aliasing(
        &self,
        loc1: &MemoryLocation,
        loc2: &MemoryLocation,
    ) -> Option<AliasResult> {
        // check if field paths are disjoint
        if !loc1.field_path.is_empty() && !loc2.field_path.is_empty() {
            let common_len = loc1
                .field_path
                .iter()
                .zip(&loc2.field_path)
                .take_while(|(a, b)| a == b)
                .count();

            // paths diverge before either ends: different fields
            if common_len < loc1.field_path.len() && common_len < loc2.field_path.len() {
                if loc1.field_path[common_len] != loc2.field_path[common_len] {
                    return Some(AliasResult::NoAlias);
                }
            }
        }

        // check direct field/element addresses from pointer sources
        let src1 = self.pointer_sources.get(&loc1.base);
        let src2 = self.pointer_sources.get(&loc2.base);

        match (src1, src2) {
            // same base, different fields
            (
                Some(PointerSource::FieldAddr {
                    base: b1,
                    field_index: f1,
                }),
                Some(PointerSource::FieldAddr {
                    base: b2,
                    field_index: f2,
                }),
            ) if b1 == b2 && f1 != f2 => Some(AliasResult::NoAlias),

            // same base, different constant indices
            (
                Some(PointerSource::ElementAddr {
                    base: b1,
                    index: Some(i1),
                }),
                Some(PointerSource::ElementAddr {
                    base: b2,
                    index: Some(i2),
                }),
            ) if b1 == b2 && i1 != i2 => Some(AliasResult::NoAlias),

            _ => None,
        }
    }

    /// Check if two types cannot alias based on TBAA.
    ///
    /// Strict aliasing rules:
    /// - Different scalar types (int vs float) cannot alias
    /// - Different struct types cannot alias
    /// - Different reference kinds may indicate non-aliasing
    fn types_cannot_alias(&self, ty1: &TypeKey, ty2: &TypeKey) -> bool {
        match (ty1, ty2) {
            // different scalar types cannot alias (strict aliasing)
            (TypeKey::Int { .. }, TypeKey::Float { .. })
            | (TypeKey::Float { .. }, TypeKey::Int { .. }) => true,

            // different integer widths/signedness may alias (could be views)
            // this is conservative; C allows char* to alias anything
            (TypeKey::Int { .. }, TypeKey::Int { .. }) => false,

            // boolean vs numeric cannot alias
            (TypeKey::Boolean, TypeKey::Int { .. })
            | (TypeKey::Int { .. }, TypeKey::Boolean)
            | (TypeKey::Boolean, TypeKey::Float { .. })
            | (TypeKey::Float { .. }, TypeKey::Boolean) => true,

            // different struct layouts cannot alias
            (TypeKey::Struct { fields: f1 }, TypeKey::Struct { fields: f2 }) => {
                f1.len() != f2.len() || f1 != f2
            }

            // tuple vs struct cannot alias
            (TypeKey::Tuple { .. }, TypeKey::Struct { .. })
            | (TypeKey::Struct { .. }, TypeKey::Tuple { .. }) => true,

            // array vs non-array cannot alias (except element type)
            (TypeKey::Array { .. }, TypeKey::Struct { .. })
            | (TypeKey::Struct { .. }, TypeKey::Array { .. })
            | (TypeKey::Array { .. }, TypeKey::Tuple { .. })
            | (TypeKey::Tuple { .. }, TypeKey::Array { .. }) => true,

            // references with different pointee types
            (
                TypeKey::Reference { pointee: p1, .. },
                TypeKey::Reference { pointee: p2, .. },
            ) => self.types_cannot_alias(p1, p2),

            // conservative: may alias
            _ => false,
        }
    }

    /// Query whether a memory operation at one location could affect
    /// the value loaded from another location.
    pub fn may_conflict(
        &self,
        store_loc: &MemoryLocation,
        load_loc: &MemoryLocation,
    ) -> bool {
        self.alias(store_loc, load_loc).may_alias()
    }

    /// Convenience method: check if two pointer values may alias.
    pub fn pointers_may_alias(&self, ptr1: mir::Value, ptr2: mir::Value) -> bool {
        let loc1 = MemoryLocation::from_pointer(ptr1);
        let loc2 = MemoryLocation::from_pointer(ptr2);
        self.alias(&loc1, &loc2).may_alias()
    }

    /// Convenience method: check if two pointer values definitely don't alias.
    pub fn pointers_no_alias(&self, ptr1: mir::Value, ptr2: mir::Value) -> bool {
        let loc1 = MemoryLocation::from_pointer(ptr1);
        let loc2 = MemoryLocation::from_pointer(ptr2);
        self.alias(&loc1, &loc2).is_no_alias()
    }
}

impl Analysis for AliasAnalysis {
    const KIND: AnalysisKind = AnalysisKind::AliasAnalysis;

    fn compute(function: &mir::Function, tree: &mir::NodeTree, _cache: &AnalysisCache) -> Arc<Self> {
        Arc::new(Self::build(function, tree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Different allocations cannot alias.
    #[test]
    fn test_different_allocations_no_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = managed.alloc @Point
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        // Find v0 and v1.
        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);

        // Different allocations should not alias.
        assert!(analysis.pointers_no_alias(v0, v1));
    }

    /// Same pointer must alias itself.
    #[test]
    fn test_same_pointer_must_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = iconst 1i32
    store v0, v1
    v2 = load v0
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let loc = MemoryLocation::from_pointer(v0);

        assert_eq!(analysis.alias(&loc, &loc), AliasResult::MustAlias);
    }

    /// Different field addresses from same base don't alias.
    #[test]
    fn test_different_fields_no_alias() {
        let program = TestProgram::new(
            r#"type @Point = { i32, i32 }
function @test() -> void {
block0:
    v0 = managed.alloc @Point
    v1 = field.addr v0, 0
    v2 = field.addr v0, 1
    v3 = iconst 1i32
    store v1, v3
    store v2, v3
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v1 = mir::Value::new(1);
        let v2 = mir::Value::new(2);

        // Different fields should not alias.
        assert!(analysis.pointers_no_alias(v1, v2));
    }

    /// Different array elements with constant indices don't alias.
    #[test]
    fn test_different_elements_no_alias() {
        let program = TestProgram::new(
            r#"type @Arr = [i32; 10]
function @test() -> void {
block0:
    v0 = stack.alloc @Arr
    v1 = iconst 0i64
    v2 = iconst 1i64
    v3 = element.addr v0, v1
    v4 = element.addr v0, v2
    v5 = iconst 42i32
    store v3, v5
    store v4, v5
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v3 = mir::Value::new(3);
        let v4 = mir::Value::new(4);

        // Different array elements should not alias.
        assert!(analysis.pointers_no_alias(v3, v4));
    }

    /// Stack and heap allocations don't alias.
    #[test]
    fn test_stack_vs_heap_no_alias() {
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = raw.alloc i32
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    raw.free v1
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);

        // Stack vs heap should not alias.
        assert!(analysis.pointers_no_alias(v0, v1));
    }

    /// Global addresses don't alias with local allocations.
    #[test]
    fn test_global_vs_local_no_alias() {
        let program = TestProgram::new(
            r#"global @g: i32 = 0i32
function @test() -> void {
block0:
    v0 = global.addr @g
    v1 = stack.alloc i32
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);

        // Global vs local allocation should not alias.
        assert!(analysis.pointers_no_alias(v0, v1));
    }

    /// Different globals don't alias.
    #[test]
    fn test_different_globals_no_alias() {
        let program = TestProgram::new(
            r#"global @g1: i32 = 0i32
global @g2: i32 = 0i32
function @test() -> void {
block0:
    v0 = global.addr @g1
    v1 = global.addr @g2
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);

        // Different globals should not alias.
        assert!(analysis.pointers_no_alias(v0, v1));
    }

    /// Casts preserve pointer provenance.
    #[test]
    fn test_cast_preserves_provenance() {
        let program = TestProgram::new(
            r#"function @test() -> void {
block0:
    v0 = stack.alloc i32
    v1 = stack.alloc i64
    v2 = bitcast v0 -> ref<raw i8>
    v3 = iconst 1i8
    store v2, v3
    v4 = iconst 1i64
    store v1, v4
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);
        let v2 = mir::Value::new(2);

        // v2 (cast of v0) should not alias v1 (different allocation).
        assert!(analysis.pointers_no_alias(v2, v1));

        // v2 (cast of v0) may alias v0 (same underlying allocation).
        assert!(analysis.pointers_may_alias(v2, v0));
    }

    /// Unknown pointers (function parameters) are conservative.
    #[test]
    fn test_parameters_may_alias() {
        let program = TestProgram::new(
            r#"function @test(v0: ref<raw i32>, v1: ref<raw i32>) -> void {
block0(v0: ref<raw i32>, v1: ref<raw i32>):
    v2 = iconst 1i32
    store v0, v2
    store v1, v2
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        let v0 = mir::Value::new(0);
        let v1 = mir::Value::new(1);

        // Parameters may alias (we don't know their provenance).
        assert!(analysis.pointers_may_alias(v0, v1));
    }

    /// Function without body returns empty analysis.
    #[test]
    fn test_handle_import_function() {
        let program = TestProgram::new(
            r#"extern function @imported() -> void"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let cache = AnalysisCache::new();
        let _analysis = cache.get::<AliasAnalysis>(function, &program.tree);

        // Just verify it doesn't panic.
    }

    /// TBAA: int vs float types cannot alias.
    #[test]
    fn test_tbaa_int_vs_float() {
        let analysis = AliasAnalysis {
            pointer_sources: HashMap::new(),
            value_types: HashMap::new(),
            constants: HashMap::new(),
        };

        let int_type = TypeKey::Int {
            width: 32,
            signed: true,
        };
        let float_type = TypeKey::Float { width: 64 };

        // int and float cannot alias (strict aliasing)
        assert!(analysis.types_cannot_alias(&int_type, &float_type));
    }

    /// TBAA: struct vs tuple cannot alias.
    #[test]
    fn test_tbaa_struct_vs_tuple() {
        let analysis = AliasAnalysis {
            pointer_sources: HashMap::new(),
            value_types: HashMap::new(),
            constants: HashMap::new(),
        };

        let struct_type = TypeKey::Struct {
            fields: vec![
                (None, TypeKey::Int { width: 32, signed: true }),
                (None, TypeKey::Int { width: 32, signed: true }),
            ],
        };
        let tuple_type = TypeKey::Tuple {
            elements: vec![
                TypeKey::Int { width: 32, signed: true },
                TypeKey::Int { width: 32, signed: true },
            ],
        };

        // struct and tuple cannot alias
        assert!(analysis.types_cannot_alias(&struct_type, &tuple_type));
    }

    /// TBAA: same int types may alias.
    #[test]
    fn test_tbaa_same_int_may_alias() {
        let analysis = AliasAnalysis {
            pointer_sources: HashMap::new(),
            value_types: HashMap::new(),
            constants: HashMap::new(),
        };

        let int32 = TypeKey::Int {
            width: 32,
            signed: true,
        };

        // same types may alias
        assert!(!analysis.types_cannot_alias(&int32, &int32));
    }
}
