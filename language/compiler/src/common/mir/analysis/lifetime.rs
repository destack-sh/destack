use std::collections::HashMap;

use destack_mir as mir;

use crate::common::mir::{
    Analysis, AnalysisId, ModuleAnalyses, ModuleAnalysis, borrowed_parameter_indices_for_signature,
    signature_return_contains_borrowed_refs,
};

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
    pub fn resolve_signature(
        signature: impl Into<mir::TypeReference>,
        tree: &mir::Tree,
    ) -> mir::Lifetime {
        let signature = signature.into();
        if let Some(lifetime) = explicit_signature_return_lifetime(&signature, tree) {
            return lifetime;
        }

        if !signature_return_contains_borrowed_refs(&signature, tree) {
            return mir::Lifetime::empty();
        }

        let Some(indices) = borrowed_parameter_indices_for_signature(&signature, tree) else {
            return mir::Lifetime::static_storage();
        };

        if indices.is_empty() {
            return mir::Lifetime::static_storage();
        }

        let indices = indices
            .into_iter()
            .map(|index| index as u32)
            .collect::<Vec<_>>();
        mir::Lifetime::slot_set(indices)
    }

    /// Build lifetime analysis for all functions in the tree.
    fn build(tree: &mir::Tree) -> Self {
        let mut function_lifetimes = HashMap::new();

        for (function_id, _) in tree.iter_nodes::<mir::Function>() {
            let resolved = tree.infer_function_return_lifetime(function_id);
            function_lifetimes.insert(function_id, resolved);
        }

        Self { function_lifetimes }
    }
}

/// Return the explicit return lifetime carried by one function signature reference.
fn explicit_signature_return_lifetime(
    signature: &mir::TypeReference,
    tree: &mir::Tree,
) -> Option<mir::Lifetime> {
    let lifetime_args = signature.lifetimes();
    let signature = signature.ty()?;
    let mir::Type::FunctionSignature { result, .. } = tree.get(signature) else {
        return None;
    };

    tree.type_reference_lifetime_with_lifetimes(result, lifetime_args)
}

impl Analysis for LifetimeAnalysis {
    const ID: AnalysisId = AnalysisId("lifetime");
    const DEPENDENCIES: &'static [AnalysisId] = &[];
}

impl ModuleAnalysis for LifetimeAnalysis {
    fn compute(tree: &mir::Tree, _analyses: &ModuleAnalyses<'_>) -> Self {
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
            r#"
function add(v0: int32, v1: int32): int32 {
b0(v0: int32, v1: int32):
    v2: int32 = int.add v0, v1
    return v2
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        assert!(analysis.get(function_id).is_empty());
    }

    /// Single borrowed param infers return lifetime from it.
    #[test]
    fn test_infer_from_single_borrowed_param() {
        let program = TestProgram::new(
            r#"
function identity(v0: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);
        assert!(lifetime.includes_slot(0));
        assert!(!lifetime.includes_slot(1));
    }

    /// Multiple borrowed params infers conservatively from all.
    #[test]
    fn test_infer_conservatively_from_multiple_params() {
        let program = TestProgram::new(
            r#"
function pick(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>): ref<int32, borrowed> {
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

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
b0(v0: ref<int32, borrowed>):
    return
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        // void return: no lifetime needed
        assert!(analysis.get(function_id).is_empty());
    }

    /// Owned reference return yields no lifetime bounds.
    #[test]
    fn test_resolve_none_for_owned_return() {
        let program = TestProgram::new(
            r#"
function create(): ref<int32, raw, space(frame)> {
b0:
    v0: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        // raw/owned return: not a borrowed ref, no lifetime
        assert!(analysis.get(function_id).is_empty());
    }

    /// Mixed borrowed and non-borrowed params infers only from borrowed ones.
    #[test]
    fn test_infer_only_from_borrowed_params() {
        let program = TestProgram::new(
            r#"
function mixed(v0: int32, v1: ref<int32, borrowed>, v2: int32): ref<int32, borrowed> {
b0(v0: int32, v1: ref<int32, borrowed>, v2: int32):
    return v1
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);

        // only v1 (index 1) is borrowed
        assert!(!lifetime.includes_slot(0)); // i32
        assert!(lifetime.includes_slot(1)); // ref<int32, borrowed>
        assert!(!lifetime.includes_slot(2)); // i32
    }

    /// Explicit static return lifetime overrides inference.
    #[test]
    fn test_resolve_explicit_static_lifetime() {
        let program = TestProgram::new(
            r#"
function getGlobal(v0: ref<int32, borrowed>): ref<int32, borrowed, lifetime(static)> {
b0(v0: ref<int32, borrowed>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

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
b0(v0: ref<int32, borrowed>, v1: ref<int32, borrowed>):
    return v0
}"#,
        );

        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let _function = program.tree.get(function_id);
        let module_analyses = program.module_analyses();
        let analysis = module_analyses.get::<LifetimeAnalysis>();

        let lifetime = analysis.get(function_id);

        // explicit annotation: only param 0
        assert!(lifetime.includes_slot(0));
        assert!(!lifetime.includes_slot(1));
    }

    /// Borrowed return with no borrowed params infers static lifetime.
    ///
    /// When a function returns a borrowed reference but has no borrowed
    /// parameters, the return must borrow from static/global data.
    #[test]
    fn test_infer_static_for_no_borrowed_params() {
        let program = TestProgram::new(
            r#"
function getStatic(v0: int32): ref<int32, borrowed> {
b0(v0: int32):
    v1: ref<int32, raw, space(frame)> = frame.alloc.zeroed int32
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

    /// Signature lifetime resolves to none when return has no borrowed refs.
    #[test]
    fn test_signature_lifetime_non_borrowed_return() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        let int_ty = program.tree.insert_type(mir::Type::Int {
            width: 32,
            is_signed: true,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into()],
            result: int_ty.into(),
            borrow_obligations: Vec::new(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.is_empty());
    }

    /// Signature lifetime resolves to static when no borrowed parameters exist.
    #[test]
    fn test_signature_lifetime_static_without_borrowed_params() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
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
            pointee: int_ty.into(),
            nullability: mir::Nullability::None,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into()],
            result: borrowed_ref.into(),
            borrow_obligations: Vec::new(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.is_static());
    }

    /// Signature lifetime resolves to parameter indices when borrowed params exist.
    #[test]
    fn test_signature_lifetime_from_borrowed_params() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
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
            pointee: int_ty.into(),
            nullability: mir::Nullability::None,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![int_ty.into(), borrowed_ref.into()],
            result: borrowed_ref.into(),
            borrow_obligations: Vec::new(),
        });

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.includes_slot(1));
        assert!(!lifetime.includes_slot(0));
    }

    /// Applied signature lifetimes resolve through the return type.
    #[test]
    fn test_resolve_applied_signature_lifetime() {
        let mut program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
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
            pointee: int_ty.into(),
            nullability: mir::Nullability::None,
        });
        let signature = program.tree.insert_type(mir::Type::FunctionSignature {
            parameters: vec![borrowed_ref.into()],
            result: borrowed_ref.into(),
            borrow_obligations: Vec::new(),
        });
        let signature = mir::TypeReference::new(signature, vec![mir::Lifetime::slot(2)]);

        let lifetime = LifetimeAnalysis::resolve_signature(signature, &program.tree);
        assert!(lifetime.includes_slot(2));
        assert!(!lifetime.includes_slot(0));
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
