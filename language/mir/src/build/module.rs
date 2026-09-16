use destack_core::{StringId, StringPool};
use destack_source::ModuleId;

use crate::build::FunctionHeaderBuilder;
use crate::{
    Copy,
    DispatchTable, DropTable, EffectTable, Layout, LayoutId, LayoutTable, ProfileTable,
    TargetLayout, Tree, Type, TypeId, WitnessTable,
};

/// The builder for one MIR module.
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The module being built.
    pub(super) module: ModuleId,
    /// The tree being built.
    pub(super) tree: Tree,
    /// The target ABI layout.
    pub(super) target_layout: TargetLayout,
    /// The canonical MIR layout table.
    pub(super) layouts: LayoutTable,
    /// The canonical MIR dispatch table.
    pub(super) dispatch: DispatchTable,
    /// The canonical MIR drop table.
    pub(super) drops: DropTable,
    /// The canonical MIR witness table.
    pub(super) witnesses: WitnessTable,

    /// The function and call effect table.
    pub(super) effects: EffectTable,
    /// The static profile counter table.
    pub(super) profile: ProfileTable,
    /// The string pool holding the names.
    pub(super) strings: StringPool,
}

impl ModuleBuilder {
    /// Create one module builder.
    pub fn new(module: ModuleId) -> Self {
        Self {
            module,
            tree: Tree::new(),
            target_layout: TargetLayout::default(),
            layouts: LayoutTable::default(),
            dispatch: DispatchTable::default(),
            drops: DropTable::default(),
            witnesses: WitnessTable::default(),
            effects: EffectTable::default(),
            profile: ProfileTable::default(),
            strings: StringPool::new(),
        }
    }

    /// Return the tree.
    pub fn tree(&self) -> &Tree {
        &self.tree
    }

    /// Return the mutable tree.
    pub fn tree_mut(&mut self) -> &mut Tree {
        &mut self.tree
    }

    /// Return the layout table.
    pub fn layouts(&self) -> &LayoutTable {
        &self.layouts
    }

    /// Return the mutable layout table.
    pub fn layouts_mut(&mut self) -> &mut LayoutTable {
        &mut self.layouts
    }

    /// Return the dispatch table.
    pub fn dispatch(&self) -> &DispatchTable {
        &self.dispatch
    }

    /// Return the mutable dispatch table.
    pub fn dispatch_mut(&mut self) -> &mut DispatchTable {
        &mut self.dispatch
    }

    /// Return the drop table.
    pub fn drops(&self) -> &DropTable {
        &self.drops
    }

    /// Return the mutable drop table.
    pub fn drops_mut(&mut self) -> &mut DropTable {
        &mut self.drops
    }

    /// Return the mutable witness table.
    pub fn witnesses_mut(&mut self) -> &mut WitnessTable {
        &mut self.witnesses
    }

    /// Return the effect table.
    pub fn effects(&self) -> &EffectTable {
        &self.effects
    }

    /// Return the mutable effect table.
    pub fn effects_mut(&mut self) -> &mut EffectTable {
        &mut self.effects
    }

    /// Return the profile table.
    pub fn profile(&self) -> &ProfileTable {
        &self.profile
    }

    /// Return the mutable profile table.
    pub fn profile_mut(&mut self) -> &mut ProfileTable {
        &mut self.profile
    }

    /// Return the string pool.
    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    /// Return the target pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.target_layout.pointer_bytes()
    }

    /// Return the target ABI layout.
    pub const fn target_layout(&self) -> TargetLayout {
        self.target_layout
    }

    /// Set the target ABI layout.
    pub fn set_target_layout(&mut self, target_layout: TargetLayout) {
        self.target_layout = target_layout;
    }

    /// Intern one string and return its id.
    pub fn intern(&mut self, text: &str) -> StringId {
        self.strings.intern(text)
    }

    /// Insert one type node directly.
    pub fn intern_type(&mut self, ty: Type, copy: Copy) -> TypeId {
        self.tree.intern_type(ty, copy)
    }

    /// Return the mutable tree and effect table together.
    pub fn tree_and_effects_mut(&mut self) -> (&mut Tree, &mut EffectTable) {
        (&mut self.tree, &mut self.effects)
    }

    /// Return the mutable tree and layout table together.
    pub fn tree_and_layouts_mut(&mut self) -> (&mut Tree, &mut LayoutTable) {
        (&mut self.tree, &mut self.layouts)
    }

    /// Record one computed layout for a type.
    pub fn insert_layout(&mut self, ty: TypeId, layout: Layout) -> LayoutId {
        let id = self.layouts.insert(layout);
        self.layouts.set_layout_id(ty, id);

        id
    }

    /// Start one function header.
    pub fn function_header(&mut self, name: &str) -> FunctionHeaderBuilder<'_> {
        FunctionHeaderBuilder::new(&self.strings, self.module, name)
    }

    /// Finish building the module.
    pub fn finish(
        self,
    ) -> (
        Tree,
        TargetLayout,
        LayoutTable,
        DispatchTable,
        DropTable,
        WitnessTable,
        EffectTable,
        ProfileTable,
        StringPool,
    ) {
        (
            self.tree,
            self.target_layout,
            self.layouts,
            self.dispatch,
            self.drops,
            self.witnesses,
            self.effects,
            self.profile,
            self.strings,
        )
    }

    /// Finish building the module and return only the tree and strings.
    pub fn finish_tree(self) -> (Tree, StringPool) {
        (self.tree, self.strings)
    }
}
