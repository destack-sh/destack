use destack_core::{StringId, StringPool};

use crate::build::FunctionHeaderBuilder;
use crate::{
    DispatchTable, DropTable, EffectTable, Layout, LayoutId, LayoutTable, LocalNodeId, MemoryTable,
    ProfileTable, TargetLayout, Tree, Type, TypeTable,
};

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The tree being built.
    pub(super) tree: Tree,
    /// Target ABI layout.
    pub(super) target_layout: TargetLayout,
    /// Canonical MIR type table.
    pub(super) types: TypeTable,
    /// Canonical MIR layout table.
    pub(super) layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub(super) dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub(super) drops: DropTable,
    /// Explicit MIR memory access table.
    pub(super) memory: MemoryTable,
    /// Function and call effect table.
    pub(super) effects: EffectTable,
    /// Static profile counter table.
    pub(super) profile: ProfileTable,
    /// String pool for names.
    pub(super) strings: StringPool,
}

impl ModuleBuilder {
    /// Create a new module builder.
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
            target_layout: TargetLayout::default(),
            types: TypeTable::default(),
            layouts: LayoutTable::default(),
            dispatch: DispatchTable::default(),
            drops: DropTable::default(),
            memory: MemoryTable::default(),
            effects: EffectTable::default(),
            profile: ProfileTable::default(),
            strings: StringPool::new(),
        }
    }

    /// Get a reference to the tree.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Get a mutable reference to the tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }

    /// Get a reference to the type table.
    pub fn types(&self) -> &TypeTable {
        &self.types
    }

    /// Get a mutable reference to the type table.
    pub fn types_mut(&mut self) -> &mut TypeTable {
        &mut self.types
    }

    /// Get a reference to the layout table.
    pub fn layouts(&self) -> &LayoutTable {
        &self.layouts
    }

    /// Get a mutable reference to the layout table.
    pub fn layouts_mut(&mut self) -> &mut LayoutTable {
        &mut self.layouts
    }

    /// Get a reference to the dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.dispatch
    }

    /// Get a mutable reference to the dispatch table.
    pub fn dispatch_mut(&mut self) -> &mut DispatchTable {
        &mut self.dispatch
    }

    /// Get a reference to the drop table.
    pub fn drops(&self) -> &DropTable {
        &self.drops
    }

    /// Get a mutable reference to the drop table.
    pub fn drops_mut(&mut self) -> &mut DropTable {
        &mut self.drops
    }

    /// Get a reference to the memory table.
    pub fn memory(&self) -> &MemoryTable {
        &self.memory
    }

    /// Get a mutable reference to the memory table.
    pub fn memory_mut(&mut self) -> &mut MemoryTable {
        &mut self.memory
    }

    /// Get a reference to the effect table.
    pub fn effects(&self) -> &EffectTable {
        &self.effects
    }

    /// Get a mutable reference to the effect table.
    pub fn effects_mut(&mut self) -> &mut EffectTable {
        &mut self.effects
    }

    /// Get a reference to the profile table.
    pub fn profile(&self) -> &ProfileTable {
        &self.profile
    }

    /// Get a mutable reference to the profile table.
    pub fn profile_mut(&mut self) -> &mut ProfileTable {
        &mut self.profile
    }

    /// Get a reference to the string pool.
    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    /// Return module pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.target_layout.pointer_bytes()
    }

    /// Return the target ABI layout.
    pub const fn target_layout(&self) -> TargetLayout {
        self.target_layout
    }

    /// Set module pointer size in bytes.
    pub fn set_pointer_bytes(&mut self, pointer_bytes: u8) {
        self.target_layout = TargetLayout::for_pointer_bytes(pointer_bytes);
    }

    /// Intern a string and return its id.
    pub fn intern(&mut self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    /// Insert one type node directly.
    pub fn intern_type(&mut self, ty: Type) -> LocalNodeId<Type> {
        self.tree.intern_type(ty)
    }

    /// Split this builder into its tree and layout table.
    pub fn tree_and_layouts_mut(&mut self) -> (&mut Tree, &mut LayoutTable) {
        (&mut self.tree, &mut self.layouts)
    }

    /// Record one computed layout for a type.
    pub fn insert_layout(&mut self, ty: LocalNodeId<Type>, layout: Layout) -> LayoutId {
        let id = self.layouts.insert(layout);
        self.layouts.types.insert(ty, id);

        id
    }

    /// Start a function header.
    pub fn function_header(&mut self, name: &str) -> FunctionHeaderBuilder<'_> {
        FunctionHeaderBuilder::new(&mut self.strings, name)
    }

    /// Finish building the module.
    pub fn finish(
        mut self,
    ) -> (
        Tree,
        TargetLayout,
        TypeTable,
        LayoutTable,
        DispatchTable,
        DropTable,
        MemoryTable,
        EffectTable,
        ProfileTable,
        StringPool,
    ) {
        self.types.rebuild_primitive_types(&self.tree);

        (
            self.tree,
            self.target_layout,
            self.types,
            self.layouts,
            self.dispatch,
            self.drops,
            self.memory,
            self.effects,
            self.profile,
            self.strings,
        )
    }

    /// Finish building the module and return only the tree and strings.
    pub fn finish_tree(mut self) -> (Tree, StringPool) {
        self.types.rebuild_primitive_types(&self.tree);

        (self.tree, self.strings)
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}
