use destack_core::{StringId, StringPool};
use destack_source::{ProvenanceBuilder, ProvenanceId, ProvenanceJournal, ProvenanceTable};

use crate::build::FunctionHeaderBuilder;
use crate::{
    AccessTable, DispatchTable, DropTable, EffectTable, Layout, LayoutId, LayoutTable, LocalNodeId,
    Node, ProfileTable, TargetLayout, Tree, TreeMut, Type, TypeId,
};

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The tree being built.
    pub(super) tree: Tree,
    /// The provenance table being extended by this module.
    pub(super) provenance: ProvenanceBuilder,
    /// The transform recorded for newly built provenance.
    pub(super) transform: &'static str,
    /// Target ABI layout.
    pub(super) target_layout: TargetLayout,
    /// Canonical MIR layout table.
    pub(super) layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub(super) dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub(super) drops: DropTable,
    /// Explicit MIR memory access table.
    pub(super) accesses: AccessTable,
    /// Function and call effect table.
    pub(super) effects: EffectTable,
    /// Static profile counter table.
    pub(super) profile: ProfileTable,
    /// String pool for names.
    pub(super) strings: StringPool,
}

impl ModuleBuilder {
    /// Create a new module builder.
    pub fn new(provenance: ProvenanceTable, transform: &'static str) -> Self {
        Self {
            tree: Tree::new(),
            provenance: provenance.extend(),
            transform,
            target_layout: TargetLayout::default(),
            layouts: LayoutTable::default(),
            dispatch: DispatchTable::default(),
            drops: DropTable::default(),
            accesses: AccessTable::default(),
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

    /// Borrow the mutable tree and active provenance journal independently.
    pub fn split_mut(&mut self) -> (&mut Tree, ProvenanceJournal<'_>) {
        let provenance = self.provenance.record(self.transform);

        (&mut self.tree, provenance)
    }

    /// Insert one node produced from one or more provenance inputs.
    pub fn insert<T>(&mut self, node: T, inputs: &[ProvenanceId]) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeMut<T>,
    {
        let provenance = self.produce(inputs);

        self.tree.insert(node, provenance)
    }

    /// Produce one provenance from one or more inputs.
    pub(super) fn produce(&mut self, inputs: &[ProvenanceId]) -> ProvenanceId {
        let mut provenance = self.provenance.record(self.transform);

        match inputs {
            [source] => provenance.derive(*source),
            [] => unreachable!("a built MIR node must have a provenance input"),
            inputs => provenance.fuse(inputs),
        }
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

    /// Get the explicit memory accesses.
    pub fn accesses(&self) -> &AccessTable {
        &self.accesses
    }

    /// Get the mutable explicit memory accesses.
    pub fn accesses_mut(&mut self) -> &mut AccessTable {
        &mut self.accesses
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

    /// Set the target ABI layout.
    pub fn set_target_layout(&mut self, target_layout: TargetLayout) {
        self.target_layout = target_layout;
    }

    /// Intern a string and return its id.
    pub fn intern(&mut self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    /// Intern one canonical type.
    pub fn intern_type(&mut self, ty: Type) -> TypeId {
        self.tree.intern_type(ty)
    }

    /// Split this builder into its tree and layout table.
    pub fn tree_and_layouts_mut(&mut self) -> (&mut Tree, &mut LayoutTable) {
        (&mut self.tree, &mut self.layouts)
    }

    /// Record one computed layout for a type.
    pub fn insert_layout(&mut self, ty: TypeId, layout: Layout) -> LayoutId {
        let id = self.layouts.insert(layout);
        self.layouts.set_layout_id(ty, id);

        id
    }

    /// Start a function header.
    pub fn function_header(&mut self, name: &str) -> FunctionHeaderBuilder<'_> {
        let provenance = self.provenance.record(self.transform);

        FunctionHeaderBuilder::new(&mut self.strings, provenance, name)
    }

    /// Finish building the module.
    pub fn finish(
        self,
    ) -> (
        Tree,
        ProvenanceTable,
        TargetLayout,
        LayoutTable,
        DispatchTable,
        DropTable,
        AccessTable,
        EffectTable,
        ProfileTable,
        StringPool,
    ) {
        (
            self.tree,
            self.provenance.finish(),
            self.target_layout,
            self.layouts,
            self.dispatch,
            self.drops,
            self.accesses,
            self.effects,
            self.profile,
            self.strings,
        )
    }

    /// Finish building the module and return only the tree and strings.
    pub fn finish_tree(self) -> (Tree, ProvenanceTable, StringPool) {
        (self.tree, self.provenance.finish(), self.strings)
    }
}
