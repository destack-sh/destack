use std::collections::HashMap;

use crate as mir;

use crate::{Analysis, AnalysisId, ModuleAnalysis, TreeAnalysisCache};

/// Lifetime analysis results for a module.
///
/// Provides the concrete return lifetime contract for each function.
#[derive(Debug)]
pub struct LifetimeAnalysis {
    /// Return lifetime contract for each function.
    function_lifetimes: HashMap<mir::LocalNodeId<mir::Function>, mir::Lifetime>,
}

impl LifetimeAnalysis {
    /// Get the return lifetime for one function.
    pub fn get(&self, function_id: mir::LocalNodeId<mir::Function>) -> &mir::Lifetime {
        self.function_lifetimes
            .get(&function_id)
            .expect("function lifetime analysis is missing a function")
    }

    /// Check if a function's return may borrow from a specific parameter.
    pub fn return_borrows_from(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        param_index: u32,
    ) -> bool {
        self.get(function_id).includes_slot(param_index)
    }

    /// Build lifetime analysis for all functions in the tree.
    fn build(tree: &mir::Tree) -> Self {
        let mut function_lifetimes = HashMap::new();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let resolved = tree
                .type_lifetime(function.return_type)
                .unwrap_or_else(mir::Lifetime::empty);
            function_lifetimes.insert(function_id, resolved);
        }

        Self { function_lifetimes }
    }
}

impl Analysis for LifetimeAnalysis {
    const ID: AnalysisId = AnalysisId("lifetime");
}

impl ModuleAnalysis for LifetimeAnalysis {
    fn compute(tree: &mir::Tree, _analyses: &TreeAnalysisCache) -> Self {
        Self::build(tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// No borrowed parameters yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_no_borrowed_params() {
        let program = TestProgram::new(
            r#"
function add(v0: int32, v1: int32): int32 {
entry(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        assert!(analysis.get(function_id).is_empty());
    }

    /// Borrowed return without declared lifetime yields no contract.
    #[test]
    fn test_resolve_none_for_undeclared_borrowed_return() {
        let program = TestProgram::new(
            r#"
function identity(v0: ref<int32, borrowed, mutable>): ref<int32, borrowed, mutable> {
entry(v0: ref<int32, borrowed, mutable>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Declared return lifetime may include multiple parameter slots.
    #[test]
    fn test_resolve_declared_multiple_parameter_lifetime() {
        let program = TestProgram::new(
            r#"
function pick<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>): ref<int32, borrowed, 'L0 | 'L1, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.includes_slot(0));
        assert!(lifetime.includes_slot(1));
    }

    /// Void return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_void_return() {
        let program = TestProgram::new(
            r#"
function consume(v0: ref<int32, borrowed, mutable>): void {
entry(v0: ref<int32, borrowed, mutable>):
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        // void return: no lifetime needed
        assert!(analysis.get(function_id).is_empty());
    }

    /// Unique reference return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_unique_return() {
        let program = TestProgram::new(
            r#"
function create(): ref<int32, unique, mutable> {
entry:
    v0: ref<int32, unique, mutable> = new.zeroed int32
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        // unique return: not a borrowed ref, no lifetime
        assert!(analysis.get(function_id).is_empty());
    }

    /// Undeclared lifetime does not infer from mixed parameters.
    #[test]
    fn test_resolve_none_for_mixed_undeclared_borrowed_return() {
        let program = TestProgram::new(
            r#"
function mixed(v0: int32, v1: ref<int32, borrowed, mutable>, v2: int32): ref<int32, borrowed, mutable> {
entry(v0: int32, v1: ref<int32, borrowed, mutable>, v2: int32):
    return v1
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Explicit static return lifetime overrides inference.
    #[test]
    fn test_resolve_explicit_static_lifetime() {
        let program = TestProgram::new(
            r#"
function getGlobal(v0: ref<int32, borrowed, mutable>): ref<int32, borrowed, 'static, mutable> {
entry(v0: ref<int32, borrowed, mutable>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_static());
        assert!(!lifetime.includes_slot(0));
    }

    /// Explicit parameter return lifetime overrides inference.
    #[test]
    fn test_resolve_explicit_param_lifetime() {
        let program = TestProgram::new(
            r#"
function pickFirst<'L0, 'L1>(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>): ref<int32, borrowed, 'L0, mutable> {
entry(v0: ref<int32, borrowed, 'L0, mutable>, v1: ref<int32, borrowed, 'L1, mutable>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);

        // explicit annotation: only param 0
        assert!(lifetime.includes_slot(0));
        assert!(!lifetime.includes_slot(1));
    }

    /// Borrowed return with no borrowed params does not infer static lifetime.
    #[test]
    fn test_resolve_none_for_undeclared_static_borrow() {
        let program = TestProgram::new(
            r#"
external function getStatic(int32): ref<int32, borrowed, mutable>
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let tree_analysis_cache = program.tree_analysis_cache();
        let analysis = tree_analysis_cache.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Applied aggregate lifetimes keep independent borrowed paths.
    #[test]
    fn test_resolve_applied_aggregate_borrowed_paths() {
        let program = TestProgram::new(
            r#"
type Pair<'A, 'B> {
    ref<int32, borrowed, 'A, readonly>;
    ref<int32, borrowed, 'B, readonly>;
}

function test<'L0, 'L1>(v0: Pair<'L0, 'L1>): void {
entry(v0: Pair<'L0, 'L1>):
    return
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let pair_type = function.parameters[0].ty;
        let paths = program.tree.type_borrowed_paths(pair_type);

        assert_eq!(paths.len(), 2, "{paths:#?}");
        assert_eq!(
            paths[0].path,
            mir::Path::root().with_projection(mir::Projection::Field { index: 0 })
        );
        assert!(paths[0].lifetime.includes_slot(0));
        assert!(!paths[0].lifetime.includes_slot(1));
        assert_eq!(
            paths[1].path,
            mir::Path::root().with_projection(mir::Projection::Field { index: 1 })
        );
        assert!(!paths[1].lifetime.includes_slot(0));
        assert!(paths[1].lifetime.includes_slot(1));
    }

    /// Resolved lifetime predicates behave correctly.
    #[test]
    fn test_check_resolved_lifetime_predicates() {
        let none = mir::Lifetime::empty();
        assert!(none.is_empty());
        assert!(!none.is_static());
        assert!(none.slot_indices().next().is_none());
        assert!(!none.includes_slot(0));

        let static_lt = mir::Lifetime::static_storage();
        assert!(!static_lt.is_empty());
        assert!(static_lt.is_static());
        assert!(static_lt.slot_indices().next().is_none());
        assert!(!static_lt.includes_slot(0));

        let param_lt = mir::Lifetime::slot_set([1, 2]);
        assert!(!param_lt.is_empty());
        assert!(!param_lt.is_static());
        assert_eq!(param_lt.slot_indices().collect::<Vec<_>>(), vec![1, 2]);
        assert!(!param_lt.includes_slot(0));
        assert!(param_lt.includes_slot(1));
        assert!(param_lt.includes_slot(2));
        assert!(!param_lt.includes_slot(3));
    }
}
