use std::collections::HashMap;

use destack_dir as dir;
use destack_source::DiagnosticSeverity;
use destack_workspace::{DiagnosticPolicy, LintSeverity};

use crate::{
    AnalyzeError, CompileError, CompileWarning, Compiler, DiagnosticAnchor, ImportError,
    ResolveError,
};

/// Severity override derived from a diagnostic directive decorator.
#[derive(Debug, Clone, Copy)]
struct DiagnosticDirectiveOverride {
    /// The override severity.
    severity: LintSeverity,
    /// Whether this override forbids inner changes.
    is_forbidden: bool,
}

impl Compiler {
    /// Resolve the effective error severity after diagnostic overrides.
    pub(crate) fn error_effective_severity(
        &self,
        error: &CompileError,
    ) -> Option<DiagnosticSeverity> {
        if let CompileError::Analyze(error) = error
            && let Some(severity) = self.analyze_policy_severity(error)
        {
            return Some(severity);
        }

        if let CompileError::Import(error) = error
            && let Some(severity) = self.import_policy_severity(error)
        {
            return Some(severity);
        }

        if let CompileError::Resolve(error) = error
            && let Some(severity) = self.resolve_policy_severity(error)
        {
            return Some(severity);
        }

        let CompileError::Optimize(error) = error else {
            return Some(DiagnosticSeverity::Error);
        };

        if !error.is_directive() {
            return Some(DiagnosticSeverity::Error);
        }

        self.diagnostic_effective_severity(error.anchor(), error.code(), LintSeverity::Error)
    }

    /// Resolve the effective warning severity after diagnostic overrides.
    pub(crate) fn warning_effective_severity(
        &self,
        warning: &CompileWarning,
    ) -> Option<DiagnosticSeverity> {
        self.diagnostic_effective_severity(
            warning.anchor(),
            &warning.full_code(),
            LintSeverity::Warning,
        )
    }

    /// Resolve the effective diagnostic severity after directive overrides.
    fn diagnostic_effective_severity(
        &self,
        anchor: DiagnosticAnchor,
        diagnostic_code: &str,
        base: LintSeverity,
    ) -> Option<DiagnosticSeverity> {
        // collect decorator overrides for the anchor
        let overrides = self.diagnostic_overrides_for_anchor(anchor, diagnostic_code);

        // apply overrides from outermost to innermost
        let mut effective = base;
        let mut is_forbidden = false;
        for override_info in overrides.into_iter().rev() {
            // honor forbid boundaries
            if is_forbidden {
                continue;
            }
            effective = override_info.severity;
            is_forbidden = override_info.is_forbidden;
        }

        // map to diagnostic severity
        match effective {
            LintSeverity::Off => None,
            LintSeverity::Note => Some(DiagnosticSeverity::Note),
            LintSeverity::Warning => Some(DiagnosticSeverity::Warning),
            LintSeverity::Error => Some(DiagnosticSeverity::Error),
        }
    }

    /// Resolve analyze error severity from policy settings.
    fn analyze_policy_severity(&self, error: &AnalyzeError) -> Option<DiagnosticSeverity> {
        let context = self.current_context_maybe()?;
        let module_id = error.anchor().module_id()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let policy = match error {
            AnalyzeError::AnyTypeDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_any),
            AnalyzeError::ImplicitAny { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_any),
            AnalyzeError::UnknownTypeDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unknown),
            AnalyzeError::ImprecisePrimitiveDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_imprecise_primitives),
            AnalyzeError::UnsafeTypeAssertionDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unsafe_type_assertions),
            AnalyzeError::MustAssertionDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_must_assertions),
            AnalyzeError::DefiniteAssignmentAssertionDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_definite_assignment_assertions),
            AnalyzeError::CustomTypeGuardDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_custom_type_guards),
            AnalyzeError::UntrustedDeclarationDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_untrusted_declarations),
            AnalyzeError::UnsoundVarianceDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unsound_variance),
            AnalyzeError::UnsoundNarrowingDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unsound_narrowing),
            AnalyzeError::UnreachableCode { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.allow_unreachable_code),
            AnalyzeError::UnusedLabel { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.allow_unused_labels),
            AnalyzeError::ImplicitThis { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_this),
            AnalyzeError::DynamicImportDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_dynamic_import),
            AnalyzeError::DynamicEvaluationDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_dynamic_evaluation),
            AnalyzeError::ProxyDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_proxy),
            AnalyzeError::DynamicShapesDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_dynamic_shapes),
            AnalyzeError::ComputedPropertyAccessDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_computed_property_access),
            AnalyzeError::ReferentialEqualityDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_referential_equality),
            AnalyzeError::GlobalThisDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_global_this),
            AnalyzeError::ImplicitDynamicDispatchDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_dynamic_dispatch),
            AnalyzeError::MissingOverride { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_override),
            AnalyzeError::MissingReturn { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_returns),
            AnalyzeError::SwitchFallthrough { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_fallthrough_cases_in_switch),
            AnalyzeError::PropertyAccessFromIndexSignature { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_property_access_from_index_signature),
            AnalyzeError::UnusedLocal { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unused_locals),
            AnalyzeError::UnusedParameter { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_unused_parameters),
            AnalyzeError::ImplicitManagedTypeDisabled { .. }
            | AnalyzeError::ImplicitManagedValueDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_implicit_managed),
            AnalyzeError::ManagedMemoryDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_managed),
            AnalyzeError::RuntimeDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_runtime),
            AnalyzeError::ExceptionsDisabled { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_exceptions),
            _ => return None,
        };

        match policy.unwrap_or(DiagnosticPolicy::Allow) {
            DiagnosticPolicy::Allow => None,
            DiagnosticPolicy::Warn => Some(DiagnosticSeverity::Warning),
            DiagnosticPolicy::Deny => Some(DiagnosticSeverity::Error),
        }
    }

    /// Resolve import error severity from policy settings.
    fn import_policy_severity(&self, error: &ImportError) -> Option<DiagnosticSeverity> {
        let context = self.current_context_maybe()?;
        let module_id = error.anchor().module_id()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let policy = match error {
            ImportError::ConflictingBinding { is_local, .. } => {
                if *is_local {
                    context
                        .compiler_options_for_module(module)
                        .map(|options| options.no_redeclared_locals)
                } else {
                    None
                }
            }
            _ => None,
        };

        match policy.unwrap_or(DiagnosticPolicy::Allow) {
            DiagnosticPolicy::Allow => None,
            DiagnosticPolicy::Warn => Some(DiagnosticSeverity::Warning),
            DiagnosticPolicy::Deny => Some(DiagnosticSeverity::Error),
        }
    }

    /// Resolve resolve-phase error severity from policy settings.
    fn resolve_policy_severity(&self, error: &ResolveError) -> Option<DiagnosticSeverity> {
        let context = self.current_context_maybe()?;
        let module_id = error.anchor().module_id()?;
        let module = context.module(module_id);
        let module = module.as_ref();
        let policy = match error {
            ResolveError::UnsupportedInternalModule { .. } => context
                .compiler_options_for_module(module)
                .map(|options| options.no_internal_import),
            _ => None,
        };

        match policy.unwrap_or(DiagnosticPolicy::Allow) {
            DiagnosticPolicy::Allow => None,
            DiagnosticPolicy::Warn => Some(DiagnosticSeverity::Warning),
            DiagnosticPolicy::Deny => Some(DiagnosticSeverity::Error),
        }
    }

    /// Collect diagnostic directive overrides for an anchor.
    fn diagnostic_overrides_for_anchor(
        &self,
        anchor: DiagnosticAnchor,
        diagnostic_code: &str,
    ) -> Vec<DiagnosticDirectiveOverride> {
        // resolve a dir node from the anchor
        match anchor {
            DiagnosticAnchor::DirNode(anchored) => {
                // resolve profile for well known decorators
                let profile_id = anchored.profile_id.or_else(|| {
                    self.current_context_maybe()
                        .map(|context| context.default_profile_id_for_module(anchored.module_id()))
                });
                let Some(profile_id) = profile_id else {
                    return Vec::new();
                };
                let decorator_map = self.collect_well_known_decorators(profile_id);

                // load the base dir tree
                let snapshot = self.artifact_dir_base(anchored.module_id());
                let Some(snapshot) = snapshot else {
                    return Vec::new();
                };
                let tree = &snapshot.tree;

                // validate the node id
                let node_id = anchored.local_id();
                if !tree.has_node_id(node_id.id) {
                    return Vec::new();
                }

                // collect overrides from the dir tree
                self.diagnostic_overrides_for_dir_node(
                    tree,
                    node_id,
                    diagnostic_code,
                    &decorator_map,
                )
            }
            DiagnosticAnchor::MirNode(anchored) => {
                // resolve profile for well known decorators
                let Some(profile_id) = self
                    .current_context_maybe()
                    .map(|context| context.default_profile_id_for_module(anchored.module_id()))
                else {
                    return Vec::new();
                };
                let decorator_map = self.collect_well_known_decorators(profile_id);

                // load the module and mir tree
                let source_id = if let Some(mir) =
                    self.mir_optimized(anchored.module_id(), profile_id, &anchored.target_id)
                {
                    mir.tree.get_source(anchored.local_id().id)
                } else if let Some(mir) =
                    self.mir_base(anchored.module_id(), profile_id, &anchored.target_id)
                {
                    mir.tree.get_source(anchored.local_id().id)
                } else {
                    None
                };
                let Some(source_id) = source_id else {
                    return Vec::new();
                };

                // load the base dir tree
                let Some(dir) = self.artifact_dir_base(anchored.module_id()) else {
                    return Vec::new();
                };
                let tree = &dir.tree;
                if !tree.has_node_id(source_id) {
                    return Vec::new();
                }

                // collect overrides from the dir tree
                let node_id = dir::LocalNodeIdAny::new(source_id, tree.get_node_type(source_id));
                self.diagnostic_overrides_for_dir_node(
                    tree,
                    node_id,
                    diagnostic_code,
                    &decorator_map,
                )
            }
            _ => Vec::new(),
        }
    }

    /// Collect decorator overrides up the parent chain from a dir node.
    fn diagnostic_overrides_for_dir_node(
        &self,
        tree: &dir::NodeTree,
        node_id: dir::LocalNodeIdAny,
        diagnostic_code: &str,
        decorator_map: &HashMap<dir::GlobalSymbolId, dir::WellKnownDecorator>,
    ) -> Vec<DiagnosticDirectiveOverride> {
        // walk up the parent chain collecting overrides
        let mut overrides = Vec::new();
        let mut current = Some(node_id);

        while let Some(current_id) = current {
            // collect overrides for the current node
            for annotation_id in tree.get_decorators(current_id.id) {
                if let Some(override_info) = self.parse_diagnostic_directive(
                    tree,
                    annotation_id,
                    diagnostic_code,
                    decorator_map,
                ) {
                    overrides.push(override_info);
                }
            }

            // move to the parent node
            current = tree.get_parent(current_id.id);
        }

        overrides
    }

    /// Parse a diagnostic directive decorator for a specific diagnostic code.
    fn parse_diagnostic_directive(
        &self,
        tree: &dir::NodeTree,
        annotation_id: dir::LocalNodeId<dir::Decorator>,
        diagnostic_code: &str,
        decorator_map: &HashMap<dir::GlobalSymbolId, dir::WellKnownDecorator>,
    ) -> Option<DiagnosticDirectiveOverride> {
        let annotation = tree.get(annotation_id);

        // resolve decorator marker symbol
        let call = self.decorator_call(tree, annotation.expression);
        let callee_expr = tree.get(call.callee);
        let target_symbol = match callee_expr {
            dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. }
            | dir::Expression::GlobalReference { target_symbol, .. } => Some(*target_symbol),
            _ => None,
        };
        let target_symbol = target_symbol?;

        // resolve the well known decorator marker
        let marker = decorator_map.get(&target_symbol)?;

        // resolve the directive severity
        let (severity, is_forbidden) = match marker {
            dir::WellKnownDecorator::Allow => (LintSeverity::Off, false),
            dir::WellKnownDecorator::Warn => (LintSeverity::Warning, false),
            dir::WellKnownDecorator::Deny => (LintSeverity::Error, false),
            dir::WellKnownDecorator::Forbid => (LintSeverity::Error, true),
            dir::WellKnownDecorator::Expect => (LintSeverity::Warning, false),
            _ => return None,
        };

        // read the first argument as a warning specifier
        let arguments = call.arguments?;
        let first_argument_id = arguments.first()?;
        let argument = tree.get(*first_argument_id);
        let value_id = match argument {
            dir::Argument::Positional { value, .. }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }
            | dir::Argument::Error { value } => *value,
            dir::Argument::Spread { .. } => return None,
        };

        // resolve the specifier string
        let value = tree.get(value_id);
        let specifier_id = match value {
            dir::Expression::ScalarLiteral {
                value: dir::ScalarLiteral::String(string_id),
            } => Some(*string_id),
            dir::Expression::UnresolvedPath { path, .. }
            | dir::Expression::LocalReference { path, .. }
            | dir::Expression::ModuleReference { path, .. }
            | dir::Expression::GlobalReference { path, .. } => {
                if path.segments.len() == 1 {
                    Some(path.segments[0])
                } else {
                    None
                }
            }
            _ => None,
        }?;

        // check the diagnostic code match
        let specifier = self.repository.strings.get(specifier_id);
        if !self.diagnostic_code_matches(specifier.as_ref(), diagnostic_code) {
            return None;
        }

        Some(DiagnosticDirectiveOverride {
            severity,
            is_forbidden,
        })
    }

    /// Check if a diagnostic code matches a decorator specifier.
    fn diagnostic_code_matches(&self, specifier: &str, diagnostic_code: &str) -> bool {
        if specifier == diagnostic_code {
            return true;
        }

        // allow the short form without the phase prefix
        let stripped = diagnostic_code
            .strip_prefix('W')
            .or_else(|| diagnostic_code.strip_prefix('E'));
        let Some(stripped) = stripped else {
            return false;
        };

        specifier == stripped
    }
}
