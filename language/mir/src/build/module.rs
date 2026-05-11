use destack_core::{StringId, StringPool};

use crate::Tree;

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The tree being built.
    pub(super) tree: Tree,
    /// String pool for names.
    pub(super) strings: StringPool,
}

impl ModuleBuilder {
    /// Create a new module builder.
    pub fn new() -> Self {
        Self {
            tree: Tree::new(),
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

    /// Get a reference to the string pool.
    pub fn strings(&self) -> &StringPool {
        &self.strings
    }

    /// Return module pointer size in bytes.
    pub fn pointer_bytes(&self) -> u8 {
        self.tree.pointer_bytes()
    }

    /// Set module pointer size in bytes.
    pub fn set_pointer_bytes(&mut self, pointer_bytes: u8) {
        self.tree.set_pointer_bytes(pointer_bytes);
    }

    /// Intern a string and return its id.
    pub fn intern(&mut self, s: &str) -> StringId {
        self.strings.intern(s)
    }

    /// Finish building the module.
    pub fn finish(self) -> (Tree, StringPool) {
        (self.tree, self.strings)
    }

    /// Finish building the module with a mutable string pool.
    pub fn finish_mutable(self) -> (Tree, StringPool) {
        (self.tree, self.strings)
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::new()
    }
}
