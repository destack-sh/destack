use std::collections::HashMap;

use destack_dir as dir;
use destack_source::DiagnosticSeverity;
use destack_workspace::LintSeverity;

use crate::{Compiler, DiagnosticAnchor, TaskError, TaskWarning};

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
    pub(crate) fn error_effective_severity(&self, error: &TaskError) -> Option<DiagnosticSeverity> {
        let TaskError::Optimize(error) = error else {
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
        warning: &TaskWarning,
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
                let profile_id = anchored.profile_id.unwrap_or_else(|| {
                    self.program
                        .default_profile_id_for_module(anchored.module_id())
                });
                let decorator_map = self.collect_well_known_decorators(profile_id);

                // load the module and dir tree
                let module = self.program.modules.get(anchored.module_id());
                let module = module.read();

                // select the correct dir tree
                let tree = if let Some(profile_id) = anchored.profile_id {
                    let Some(dir) = module.dir_maybe(profile_id) else {
                        return Vec::new();
                    };
                    dir.tree.read()
                } else {
                    let Some(dir) = module.dir_base_maybe() else {
                        return Vec::new();
                    };
                    dir.tree.read()
                };

                // validate the node id
                let node_id = anchored.local_id();
                if !tree.has_node_id(node_id.id) {
                    return Vec::new();
                }

                // collect overrides from the dir tree
                self.diagnostic_overrides_for_dir_node(
                    &tree,
                    node_id,
                    diagnostic_code,
                    &decorator_map,
                )
            }
            DiagnosticAnchor::MirNode(anchored) => {
                // resolve profile for well known decorators
                let profile_id = self
                    .program
                    .default_profile_id_for_module(anchored.module_id());
                let decorator_map = self.collect_well_known_decorators(profile_id);

                // load the module and mir tree
                let module = self.program.modules.get(anchored.module_id());
                let module = module.read();

                // resolve the source dir node
                let mir = module.mir(&anchored.target_id);
                let mir_tree = mir.tree.read();
                let Some(source_id) = mir_tree.get_source(anchored.local_id().id) else {
                    return Vec::new();
                };

                // load the base dir tree
                let Some(dir) = module.dir_base_maybe() else {
                    return Vec::new();
                };
                let tree = dir.tree.read();
                if !tree.has_node_id(source_id) {
                    return Vec::new();
                }

                // collect overrides from the dir tree
                let node_id = dir::LocalNodeIdAny::new(source_id, tree.get_node_type(source_id));
                self.diagnostic_overrides_for_dir_node(
                    &tree,
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
            for annotation_id in tree.get_annotations(current_id.id) {
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
        annotation_id: dir::LocalNodeId<dir::Annotation>,
        diagnostic_code: &str,
        decorator_map: &HashMap<dir::GlobalSymbolId, dir::WellKnownDecorator>,
    ) -> Option<DiagnosticDirectiveOverride> {
        let annotation = tree.get(annotation_id);
        let dir::Annotation::Decorator {
            left, arguments, ..
        } = annotation
        else {
            return None;
        };

        // resolve decorator marker symbol
        let left_expr = tree.get(*left);
        let target_symbol = match left_expr {
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
        let Some(arguments) = arguments else {
            return None;
        };
        let first_argument_id = arguments.first()?;
        let argument = tree.get(*first_argument_id);
        let value_id = match argument {
            dir::Argument::Positional { value, .. }
            | dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. } => *value,
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
        let specifier = self.program.strings.get(specifier_id);
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
