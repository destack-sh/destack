use std::collections::HashMap;

use destack_mir::{self as mir, Lifetime};

use crate::optimize::{
    Analysis, AnalysisId, ModuleAnalyses, ModuleAnalysis, borrowed_parameter_indices_for_function,
    type_contains_borrowed_refs,
};

/// Resolved lifetime bounds for a function's return value.
///
/// Indicates which parameters the return value may borrow from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedLifetime {
    /// Return does not contain borrowed references (or is void).
    None,
    /// Return borrows from specific parameters (by index).
    /// Typically 1-2 parameters, so Vec is efficient enough.
    Parameters(Vec<u32>),
    /// Return has static lifetime (borrows from global/constant data only).
    Static,
}

impl ResolvedLifetime {
    /// Check if this lifetime indicates no borrowing.
    pub fn is_none(&self) -> bool {
        matches!(self, ResolvedLifetime::None)
    }

    /// Check if this is a static lifetime.
    pub fn is_static(&self) -> bool {
        matches!(self, ResolvedLifetime::Static)
    }

    /// Get the parameter indices if this is a parameter-based lifetime.
    pub fn parameters(&self) -> Option<&[u32]> {
        match self {
            ResolvedLifetime::Parameters(params) => Some(params),
            _ => None,
        }
    }

    /// Check if this lifetime includes a specific parameter.
    pub fn includes_parameter(&self, index: u32) -> bool {
        match self {
            ResolvedLifetime::Parameters(params) => params.contains(&index),
            _ => false,
        }
    }
}

/// Lifetime analysis results for a module.
///
/// Provides resolved lifetime bounds for each function, either from explicit
/// annotations or inferred from function signatures.
#[derive(Debug)]
pub struct LifetimeAnalysis {
    /// Resolved lifetime bounds for each function.
    function_lifetimes: HashMap<mir::LocalNodeId<mir::Function>, ResolvedLifetime>,
}

impl LifetimeAnalysis {
    /// Get the resolved lifetime for a function's return value.
    pub fn get(&self, function_id: mir::LocalNodeId<mir::Function>) -> &ResolvedLifetime {
        self.function_lifetimes
            .get(&function_id)
            .unwrap_or(&ResolvedLifetime::None)
    }

    /// Check if a function's return may borrow from a specific parameter.
    pub fn return_borrows_from(
        &self,
        function_id: mir::LocalNodeId<mir::Function>,
        param_index: u32,
    ) -> bool {
        self.get(function_id).includes_parameter(param_index)
    }

    /// Build lifetime analysis for all functions in the tree.
    fn build(tree: &mir::NodeTree) -> Self {
        let mut function_lifetimes = HashMap::new();

        for (function_id, function) in tree.iter_nodes::<mir::Function>() {
            let resolved = Self::resolve_function_lifetime(function, tree);
            function_lifetimes.insert(function_id, resolved);
        }

        Self { function_lifetimes }
    }

    /// Resolve the effective lifetime for a single function.
    fn resolve_function_lifetime(
        function: &mir::Function,
        tree: &mir::NodeTree,
    ) -> ResolvedLifetime {
        // check if return type contains borrowed references
        let return_ty = tree.get(function.return_type);
        if !type_contains_borrowed_refs(return_ty, tree) {
            return ResolvedLifetime::None;
        }

        // check explicit annotation
        match &function.return_lifetime {
            Lifetime::Static => return ResolvedLifetime::Static,
            Lifetime::Parameters(params) => {
                return ResolvedLifetime::Parameters(params.clone());
            }
            Lifetime::Inferred => {
                // fall through to inference
            }
        }

        // find all borrowed reference parameters for inference
        let borrowed_params = borrowed_parameter_indices_for_function(function, tree);

        // no borrowed parameters: might be a global/static borrow or an error
        // (assume static, stack borrows are checked by stack-check)
        if borrowed_params.is_empty() {
            return ResolvedLifetime::Static;
        }

        // single borrowed param: return borrows from it
        if borrowed_params.len() == 1 {
            return ResolvedLifetime::Parameters(vec![borrowed_params[0]]);
        }

        // multiple borrowed params: conservative, may borrow from all
        ResolvedLifetime::Parameters(borrowed_params)
    }

    // type_contains_borrowed_refs is shared in optimize::common::borrow
}

impl Analysis for LifetimeAnalysis {
    const ID: AnalysisId = AnalysisId("lifetime");
    const DEPENDENCIES: &'static [AnalysisId] = &[];
}

impl ModuleAnalysis for LifetimeAnalysis {
    fn compute(tree: &mir::NodeTree, _analyses: &ModuleAnalyses<'_>) -> Self {
        Self::build(tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// No borrowed parameters yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_no_borrowed_params() {
        let program = TestProgram::new(
            r#"function @add(v0: i32, v1: i32) -> i32 {
block0(v0: i32, v1: i32):
    v2: i32 = iadd v0, v1
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        assert!(analysis.get(function_id).is_none());
    }

    /// Single borrowed param infers return lifetime from it.
    #[test]
    fn test_infer_from_single_borrowed_param() {
        let program = TestProgram::new(
            r#"function @identity(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);
        assert!(lifetime.includes_parameter(0));
        assert!(!lifetime.includes_parameter(1));
    }

    /// Multiple borrowed params infers conservatively from all.
    #[test]
    fn test_infer_conservatively_from_multiple_params() {
        let program = TestProgram::new(
            r#"function @pick(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);
        assert!(lifetime.includes_parameter(0));
        assert!(lifetime.includes_parameter(1));
    }

    /// Void return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_void_return() {
        let program = TestProgram::new(
            r#"function @consume(v0: ref<borrowed i32>) -> void {
block0(v0: ref<borrowed i32>):
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        // void return: no lifetime needed
        assert!(analysis.get(function_id).is_none());
    }

    /// Owned reference return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_owned_return() {
        let program = TestProgram::new(
            r#"function @create() -> ref<raw i32> {
block0:
    v0: ref<raw i32> = raw.alloc i32
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        // raw/owned return: not a borrowed ref, no lifetime
        assert!(analysis.get(function_id).is_none());
    }

    /// Mixed borrowed and non-borrowed params infers only from borrowed ones.
    #[test]
    fn test_infer_only_from_borrowed_params() {
        let program = TestProgram::new(
            r#"function @mixed(v0: i32, v1: ref<borrowed i32>, v2: i32) -> ref<borrowed i32> {
block0(v0: i32, v1: ref<borrowed i32>, v2: i32):
    return v1
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);

        // only v1 (index 1) is borrowed
        assert!(!lifetime.includes_parameter(0)); // i32
        assert!(lifetime.includes_parameter(1)); // ref<borrowed i32>
        assert!(!lifetime.includes_parameter(2)); // i32
    }

    /// Explicit static lifetime annotation overrides inference.
    #[test]
    fn test_resolve_explicit_static_lifetime() {
        let mut program = TestProgram::new(
            r#"function @get_global(v0: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>):
    return v0
}"#,
        );

        // set explicit static lifetime on the function
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        {
            let function = program.tree.get_mut(function_id);
            function.return_lifetime = mir::Lifetime::Static;
        }

        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_static());
        assert!(!lifetime.includes_parameter(0));
    }

    /// Explicit parameter lifetime annotation overrides inference.
    #[test]
    fn test_resolve_explicit_param_lifetime() {
        let mut program = TestProgram::new(
            r#"function @pick_first(v0: ref<borrowed i32>, v1: ref<borrowed i32>) -> ref<borrowed i32> {
block0(v0: ref<borrowed i32>, v1: ref<borrowed i32>):
    return v0
}"#,
        );

        // set explicit lifetime to only borrow from param 0 (not both)
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        {
            let function = program.tree.get_mut(function_id);
            function.return_lifetime = mir::Lifetime::param(0);
        }

        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);

        // explicit annotation: only param 0
        assert!(lifetime.includes_parameter(0));
        assert!(!lifetime.includes_parameter(1));
    }

    /// Borrowed return with no borrowed params infers static lifetime.
    ///
    /// When a function returns a borrowed reference but has no borrowed
    /// parameters, the return must borrow from static/global data.
    #[test]
    fn test_infer_static_for_no_borrowed_params() {
        let program = TestProgram::new(
            r#"function @get_static(v0: i32) -> ref<borrowed i32> {
block0(v0: i32):
    v1: ref<raw addrspace(stack) i32> = stack.alloc i32
    return v1
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);

        // no borrowed params: must be static
        assert!(lifetime.is_static());
    }

    /// lifetimeLifetime helper methods behave correctly.
    #[test]
    fn test_check_lifetime_lifetime_helpers() {
        let none = ResolvedLifetime::None;
        assert!(none.is_none());
        assert!(!none.is_static());
        assert!(none.parameters().is_none());
        assert!(!none.includes_parameter(0));

        let static_lt = ResolvedLifetime::Static;
        assert!(!static_lt.is_none());
        assert!(static_lt.is_static());
        assert!(static_lt.parameters().is_none());
        assert!(!static_lt.includes_parameter(0));

        let param_lt = ResolvedLifetime::Parameters(vec![1, 2]);
        assert!(!param_lt.is_none());
        assert!(!param_lt.is_static());
        assert!(param_lt.parameters().is_some());
        assert!(!param_lt.includes_parameter(0));
        assert!(param_lt.includes_parameter(1));
        assert!(param_lt.includes_parameter(2));
        assert!(!param_lt.includes_parameter(3));
    }
}
