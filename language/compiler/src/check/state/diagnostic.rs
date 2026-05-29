use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::Origin;
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

        self.module_mut(module).diagnostics.push(diagnostic);
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

        self.module_mut(module).diagnostics.push(diagnostic);
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

        self.module_mut(module).diagnostics.push(diagnostic);
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

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report an invalid static condition at one source node.
    pub(in crate::check) fn report_invalid_static_condition(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidCondition { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic);
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

        self.module_mut(module).diagnostics.push(diagnostic);
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

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report a parsed type form that is not supported by the language model.
    pub(in crate::check) fn report_unsupported_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
        name: impl Into<String>,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UnsupportedType {
            anchor,
            module,
            name: name.into(),
        };

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report an invalid writable place at one source node.
    pub(in crate::check) fn report_not_writable(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::NotWritable { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report a read from a binding that is not definitely assigned.
    pub(in crate::check) fn report_use_before_assigned(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::UseBeforeAssigned { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Return the diagnostic anchor for one source node.
    pub(in crate::check) fn diagnostic_anchor(
        &self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) -> DiagnosticAnchor {
        let span = match self.module(module).parsed.tree.get_span_by_id(source.id) {
            Some(span) => span,
            None => panic!("check node {} has no source span", source.id),
        };

        DiagnosticAnchor::from(span)
    }

    /// Return the diagnostic anchor for one check origin.
    pub(in crate::check) fn diagnostic_anchor_for_origin(
        &self,
        origin: Origin,
    ) -> (ModuleId, DiagnosticAnchor) {
        let module = origin.module();
        let source = match origin {
            Origin::Node(node) => node.local_id,
            Origin::Symbol(symbol) => self.symbol_source_node(symbol),
        };
        let anchor = self.diagnostic_anchor(module, source);

        (module, anchor)
    }

    /// Return a circular type diagnostic for one origin.
    pub(in crate::check) fn circular_type_error(&self, origin: Origin) -> CheckError {
        let (module, anchor) = self.diagnostic_anchor_for_origin(origin);

        CheckError::CircularType { anchor, module }
    }

    /// Return a type complexity diagnostic for one origin.
    pub(in crate::check) fn type_too_complex_error(&self, origin: Origin) -> CheckError {
        let (module, anchor) = self.diagnostic_anchor_for_origin(origin);

        CheckError::TypeTooComplex { anchor, module }
    }

    /// Return a human readable path label.
    fn path_label(&self, module: ModuleId, path: &dir::Path) -> String {
        let mut label = String::new();

        // join path segments with dot notation
        for (index, segment) in path.segments.iter().enumerate() {
            if index > 0 {
                label.push('.');
            }

            label.push_str(self.module(module).strings.get(*segment));
        }

        label
    }
}
