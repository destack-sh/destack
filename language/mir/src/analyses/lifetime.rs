use std::collections::HashMap;

use crate as mir;

use crate::{Analysis, AnalysisId, ModuleAnalyses, ModuleAnalysis};

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

    /// Resolve a return lifetime from a function signature type.
    pub fn resolve_signature(signature: impl Into<mir::TypeId>, tree: &mir::Tree) -> mir::Lifetime {
        let signature = signature.into();
        if let Some(lifetime) = explicit_signature_return_lifetime(&signature, tree) {
            return lifetime;
        }

        mir::Lifetime::empty()
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

/// Return the explicit return lifetime carried by one function signature reference.
fn explicit_signature_return_lifetime(
    signature: &mir::TypeId,
    tree: &mir::Tree,
) -> Option<mir::Lifetime> {
    let signature = *signature;
    let mir::Type::FunctionSignature { result, .. } = tree.get(signature) else {
        return None;
    };

    tree.type_lifetime(*result)
}

impl Analysis for LifetimeAnalysis {
    const ID: AnalysisId = AnalysisId("lifetime");
}

impl ModuleAnalysis for LifetimeAnalysis {
    fn compute(tree: &mir::Tree, _analyses: &ModuleAnalyses) -> Self {
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
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        assert!(analysis.get(function_id).is_empty());
    }

    /// Borrowed return without declared lifetime yields no contract.
    #[test]
    fn test_resolve_none_for_undeclared_borrowed_return() {
        let program = TestProgram::new(
            r#"
function identity(v0: ref<int32, borrowed>): ref<int32, borrowed> {
entry(v0: ref<int32, borrowed>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Declared return lifetime may include multiple parameter slots.
    #[test]
    fn test_resolve_declared_multiple_parameter_lifetime() {
        let program = TestProgram::new(
            r#"
function pick(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0, 1)> {
entry(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.includes_slot(0));
        assert!(lifetime.includes_slot(1));
    }

    /// Void return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_void_return() {
        let program = TestProgram::new(
            r#"
function consume(v0: ref<int32, borrowed>): void {
entry(v0: ref<int32, borrowed>):
    return
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        // void return: no lifetime needed
        assert!(analysis.get(function_id).is_empty());
    }

    /// Owned reference return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_owned_return() {
        let program = TestProgram::new(
            r#"
function create(): ref<int32, raw, space(frame)> {
entry:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        // raw/owned return: not a borrowed ref, no lifetime
        assert!(analysis.get(function_id).is_empty());
    }

    /// Undeclared lifetime does not infer from mixed parameters.
    #[test]
    fn test_resolve_none_for_mixed_undeclared_borrowed_return() {
        let program = TestProgram::new(
            r#"
function mixed(v0: int32, v1: ref<int32, borrowed>, v2: int32): ref<int32, borrowed> {
entry(v0: int32, v1: ref<int32, borrowed>, v2: int32):
    return v1
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Explicit static return lifetime overrides inference.
    #[test]
    fn test_resolve_explicit_static_lifetime() {
        let program = TestProgram::new(
            r#"
function getGlobal(v0: ref<int32, borrowed>): ref<int32, borrowed, lifetime(static)> {
entry(v0: ref<int32, borrowed>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_static());
        assert!(!lifetime.includes_slot(0));
    }

    /// Explicit parameter return lifetime overrides inference.
    #[test]
    fn test_resolve_explicit_param_lifetime() {
        let program = TestProgram::new(
            r#"
function pickFirst(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed, lifetime(0)> {
entry(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

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
function getStatic(v0: int32): ref<int32, borrowed> {
entry(v0: int32):
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    return v1
}
"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>(&program.tree);

        let lifetime = analysis.get(function_id);
        assert!(lifetime.is_empty());
    }

    /// Signature lifetime resolves to none when return has no borrowed refs.
    #[test]
    fn test_signature_lifetime_non_borrowed_return() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let int_ty = program.tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into()],
            result: int_ty.into(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.is_empty());
    }

    /// Signature lifetime resolves to none without declared return lifetime.
    #[test]
    fn test_signature_lifetime_none_without_declared_lifetime() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let int_ty = program.tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = program.tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
            access: mir::Access::Readonly,
            pointee: int_ty,
            nullability: mir::Nullability::None,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into()],
            result: borrowed_ref.into(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.is_empty());
    }

    /// Signature lifetime resolves to none instead of inferring from borrowed params.
    #[test]
    fn test_signature_lifetime_none_for_undeclared_borrowed_params() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let int_ty = program.tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = program.tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            space: mir::Space::Local,
            access: mir::Access::Readonly,
            pointee: int_ty,
            nullability: mir::Nullability::None,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into(), borrowed_ref.into()],
            result: borrowed_ref.into(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.is_empty());
    }

    /// Applied signature lifetimes resolve through the return type.
    #[test]
    fn test_resolve_applied_signature_lifetime() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
entry:
    return
}
"#,
        );

        let int_ty = program.tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let borrowed_ref = program.tree.insert_type(mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::slot(0),
            space: mir::Space::Local,
            access: mir::Access::Readonly,
            pointee: int_ty,
            nullability: mir::Nullability::None,
        });
        let applied_ref = program.tree.insert_type(mir::Type::WithLifetimes {
            base: borrowed_ref,
            lifetimes: vec![mir::Lifetime::slot(2)],
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![applied_ref.into()],
            result: applied_ref.into(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.includes_slot(2));
        assert!(!lifetime.includes_slot(0));
    }

    /// Applied aggregate lifetimes keep independent borrowed paths.
    #[test]
    fn test_resolve_applied_aggregate_borrowed_paths() {
        let program = TestProgram::new(
            r#"
type Pair<A: lifetime, B: lifetime> {
    ref<int32, borrowed, lifetime(A), readonly>;
    ref<int32, borrowed, lifetime(B), readonly>;
}

function test(v0: Pair<lifetime(0), lifetime(1)>): void {
entry(v0: Pair<lifetime(0), lifetime(1)>):
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
