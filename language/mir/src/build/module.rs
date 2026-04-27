use destack_core::{ImmutableStringPool, StringId, StringPool};

use crate::Tree;
use crate::validate::Validator;

/// Builder for constructing a MIR module (collection of functions and types).
#[derive(Debug)]
pub struct ModuleBuilder {
    /// The tree being built.
    pub(super) tree: Tree,
    /// String pool for names.
    pub(super) strings: StringPool,
    /// Whether to verify functions as they are built.
    pub(super) verify: bool,
}

impl ModuleBuilder {
    /// Create a new unverified module builder.
    pub fn unchecked() -> Self {
        Self::new_with_verify(false)
    }

    /// Create a new module builder with function verification enabled.
    pub fn checked() -> Self {
        Self::new_with_verify(true)
    }

    /// Create a new module builder with the given verify flag.
    pub fn new_with_verify(verify: bool) -> Self {
        Self {
            tree: Tree::new(),
            strings: StringPool::new(),
            verify,
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
    pub fn finish_immutable(self) -> (Tree, ImmutableStringPool) {
        self.validate_tree();
        (self.tree, self.strings.into_immutable())
    }

    /// Finish building the module with a mutable string pool.
    pub fn finish_mutable(self) -> (Tree, StringPool) {
        self.validate_tree();
        (self.tree, self.strings)
    }

    /// Validate the completed tree when builder verification is enabled.
    fn validate_tree(&self) {
        if !self.verify {
            return;
        }

        let validator = Validator::new(&self.tree);
        if let Err(error) = validator.validate() {
            panic!("mir validation failed: {error}");
        }
    }
}

impl Default for ModuleBuilder {
    fn default() -> Self {
        Self::unchecked()
    }
}
