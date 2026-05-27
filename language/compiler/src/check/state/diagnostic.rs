use destack_dir as dir;
use destack_source::ModuleId;

use crate::{CheckError, DiagnosticAnchor};

use super::CheckState;

impl CheckState<'_> {
    /// Report a missing annotation at one source node.
    pub(in crate::check) fn report_missing_type_annotation(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::MissingTypeAnnotation { anchor, module };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report invalid control flow at one source node.
    pub(in crate::check) fn report_invalid_control_flow(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        message: &'static str,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidControlFlow {
            anchor,
            module,
            message: message.to_owned(),
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an invalid yield expression at one source node.
    pub(in crate::check) fn report_invalid_yield(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        message: &'static str,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidYield {
            anchor,
            module,
            message: message.to_owned(),
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an invalid await expression at one source node.
    pub(in crate::check) fn report_invalid_await(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        message: &'static str,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidAwait {
            anchor,
            module,
            message: message.to_owned(),
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an invalid static condition at one source node.
    pub(in crate::check) fn report_invalid_static_condition(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidStaticCondition { anchor, module };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an unresolved reference at one source node.
    pub(in crate::check) fn report_unresolved_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnresolvedReference {
            anchor,
            module,
            name: self.path_label(module, path),
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an ambiguous reference at one source node.
    pub(in crate::check) fn report_ambiguous_reference(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        path: &dir::Path,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::AmbiguousReference {
            anchor,
            module,
            name: self.path_label(module, path),
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an internal check failure at one source node.
    pub(in crate::check) fn report_internal(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        message: String,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::Internal {
            anchor,
            module,
            message,
        };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Report an invalid writable place at one source node.
    pub(in crate::check) fn report_not_writable(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::NotWritable { anchor, module };

        self.diagnostics_mut(module).push(diagnostic);
    }

    /// Return a human readable path label.
    fn path_label(&self, module: ModuleId, path: &dir::Path) -> String {
        let mut label = String::new();

        // join path segments with dot notation
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                label.push('.');
            }

            label.push_str(self.input(module).strings.get(*segment));
        }

        label
    }

    /// Return the diagnostic anchor for one source node.
    fn diagnostic_anchor(&self, module: ModuleId, source: dir::LocalNodeIdAny) -> DiagnosticAnchor {
        let span = match self.input(module).parsed.tree.get_span_by_id(source.id) {
            Some(span) => span,
            None => panic!("check node {} has no source span", source.id),
        };

        DiagnosticAnchor::from(span)
    }
}
