use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use destack_artifact::DirDeclared;
use destack_dir::{GlobalSymbolId, StaticKey, SymbolSpace};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::analyze::module::GlobalMergeCategory;

/// Shared task-local analyze index state.
#[derive(Debug, Default)]
struct AnalyzeIndexState {
    /// Cached declared directories for remote merge normalization.
    declared_directories: HashMap<(ModuleId, ProfileId), Arc<DirDeclared>>,
    /// Cached raw global and library merge source groups.
    global_merge_sources: HashMap<GlobalMergeSourcesKey, Vec<GlobalSymbolId>>,
}

/// A task-local analyze index for stable derived lookup facts.
#[derive(Debug, Clone, Default)]
pub(crate) struct AnalyzeIndex {
    /// The shared mutable index state.
    state: Rc<RefCell<AnalyzeIndexState>>,
}

/// One cache key for raw global and library merge source groups.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) struct GlobalMergeSourcesKey {
    /// The active module.
    pub module: ModuleId,
    /// The active profile.
    pub profile: ProfileId,
    /// The canonical merge key.
    pub key: StaticKey,
    /// The anchor declaration space.
    pub anchor_space: SymbolSpace,
    /// The merge source category.
    pub category: GlobalMergeCategory,
}

impl AnalyzeIndex {
    /// Return one cached declared directory when available.
    pub(crate) fn declared_directory(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Option<Arc<DirDeclared>> {
        self.state
            .borrow()
            .declared_directories
            .get(&(module, profile))
            .cloned()
    }

    /// Cache one declared directory for later reuse.
    pub(crate) fn set_declared_directory(
        &self,
        module: ModuleId,
        profile: ProfileId,
        directory: Arc<DirDeclared>,
    ) {
        self.state
            .borrow_mut()
            .declared_directories
            .insert((module, profile), directory);
    }

    /// Return one cached raw merge source group when available.
    pub(crate) fn global_merge_sources(
        &self,
        key: GlobalMergeSourcesKey,
    ) -> Option<Vec<GlobalSymbolId>> {
        self.state.borrow().global_merge_sources.get(&key).cloned()
    }

    /// Cache one raw merge source group for later reuse.
    pub(crate) fn set_global_merge_sources(
        &self,
        key: GlobalMergeSourcesKey,
        symbols: Vec<GlobalSymbolId>,
    ) {
        self.state
            .borrow_mut()
            .global_merge_sources
            .insert(key, symbols);
    }
}
