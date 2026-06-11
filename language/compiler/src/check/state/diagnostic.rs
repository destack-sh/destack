use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    CheckState, Constraint, Dependency, DumpContext, Origin, StaticOperand, Task, TypeOperand,
    VariableId,
};
use crate::{CheckError, CompilerError, CompilerResult, DiagnosticAnchor};

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

    /// Report an invalid static guard at one source node.
    pub(in crate::check) fn report_invalid_static_guard(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidCondition { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report a method receiver omitted under implicit receiver restrictions.
    pub(in crate::check) fn report_implicit_receiver(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::ImplicitReceiver { anchor, module };

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

    /// Report an intrinsic marker outside a compiler-recognized language item.
    pub(in crate::check) fn report_invalid_intrinsic_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidIntrinsicType { anchor, module };

        self.module_mut(module).diagnostics.push(diagnostic);
    }

    /// Report a const marker outside an `as const` assertion.
    pub(in crate::check) fn report_invalid_const_type(
        &mut self,
        module: ModuleId,
        source: dir::LocalNodeIdAny,
    ) {
        let anchor = self.diagnostic_anchor(module, source);
        let diagnostic = CheckError::InvalidConstType { anchor, module };

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
        let span = match self.module(module).view().get_span_by_id(source.id) {
            Some(span) => span,
            None => unreachable!("check node {} has no source span", source.id),
        };

        DiagnosticAnchor::from(span)
    }

    /// Return one check origin's diagnostic anchor.
    pub(in crate::check) fn origin_diagnostic_anchor(
        &self,
        origin: Origin,
    ) -> CompilerResult<(ModuleId, DiagnosticAnchor)> {
        let module = origin.module();
        let anchor = match origin {
            Origin::Node(node) => self.diagnostic_anchor(module, node.local_id),
            Origin::Symbol(symbol) => {
                let source = self
                    .module(symbol.module_id)
                    .symbol_declaration_node(symbol.local_id)?;

                self.diagnostic_anchor(module, source)
            }
            Origin::Type(_) => DiagnosticAnchor::from(module),
        };

        Ok((module, anchor))
    }

    /// Return a circular type diagnostic for one origin.
    pub(in crate::check) fn circular_type_error(
        &self,
        origin: Origin,
    ) -> CompilerResult<CheckError> {
        let (module, anchor) = self.origin_diagnostic_anchor(origin)?;

        Ok(CheckError::CircularType { anchor, module })
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

    /// Return an internal error for one node type operand that cannot be committed.
    pub(in crate::check) fn unresolved_node_type_error(
        &self,
        module: ModuleId,
        node: dir::GlobalNodeIdAny,
        operand: TypeOperand,
    ) -> CompilerError {
        self.unresolved_type_operand_error(module, "node_type", "node", Origin::Node(node), operand)
    }

    /// Return an internal error for one symbol type operand that cannot be committed.
    pub(in crate::check) fn unresolved_symbol_type_error(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: TypeOperand,
    ) -> CompilerError {
        self.unresolved_type_operand_error(
            module,
            "symbol_type",
            "symbol",
            Origin::Symbol(symbol),
            operand,
        )
    }

    /// Return an internal error for one symbol static operand that cannot be committed.
    pub(in crate::check) fn unresolved_symbol_static_error(
        &self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        operand: StaticOperand,
    ) -> CompilerError {
        self.unresolved_static_operand_error(
            module,
            "symbol_static",
            "symbol",
            Origin::Symbol(symbol),
            operand,
        )
    }

    /// Return an internal error for one type operand that cannot be committed.
    fn unresolved_type_operand_error(
        &self,
        module: ModuleId,
        kind: &'static str,
        target_key: &'static str,
        target: Origin,
        operand: TypeOperand,
    ) -> CompilerError {
        let mut log = ArtifactEventLog::new();
        let mut event = ArtifactEvent::new("commit.unresolved")
            .error()
            .text("kind", kind)
            .text("module", DumpContext::new(self).module_label(module))
            .text(target_key, self.dump_in_module(module, &target))
            .text("operand", self.dump_in_module(module, &operand));

        // identify the unresolved DIR node shape
        if let Origin::Node(node) = target {
            event = event.text("variant", self.node_variant(node));
            event = event.text("parents", self.node_parent_variants(node));
        }

        // include bounds for open variable operands
        if let TypeOperand::Variable(variable) = operand {
            let lower_bounds = self.inference.lower_type_bounds(variable);
            let upper_bounds = self.inference.upper_type_bounds(variable);
            let lower = lower_bounds
                .iter()
                .map(|bound| self.dump_in_module(module, &bound))
                .collect::<Vec<_>>()
                .join(",");
            let upper = upper_bounds
                .iter()
                .map(|bound| self.dump_in_module(module, &bound))
                .collect::<Vec<_>>()
                .join(",");
            let dependencies = lower_bounds
                .iter()
                .chain(upper_bounds.iter())
                .flat_map(|bound| bound.dependencies(self))
                .filter_map(Dependency::solution_variable)
                .filter(|dependency| *dependency != variable)
                .map(|dependency| {
                    let lower = self
                        .inference
                        .lower_type_bounds(dependency)
                        .into_iter()
                        .map(|bound| self.dump_in_module(module, &bound))
                        .collect::<Vec<_>>()
                        .join(",");
                    let upper = self
                        .inference
                        .upper_type_bounds(dependency)
                        .into_iter()
                        .map(|bound| self.dump_in_module(module, &bound))
                        .collect::<Vec<_>>()
                        .join(",");

                    format!(
                        "{} lower=[{}] upper=[{}]",
                        self.dump_in_module(module, &dependency),
                        lower,
                        upper
                    )
                })
                .collect::<Vec<_>>()
                .join(";");

            event = event.text("lower", format!("[{lower}]"));
            event = event.text("upper", format!("[{upper}]"));
            event = event.text("dependencies", format!("[{dependencies}]"));
            event = event.text("constraints", self.variable_constraints(module, variable));
        }

        log.push(event);

        self.unresolved_commit_error(log)
    }

    /// Return an internal error for one static operand that cannot be committed.
    fn unresolved_static_operand_error(
        &self,
        module: ModuleId,
        kind: &'static str,
        target_key: &'static str,
        target: Origin,
        operand: StaticOperand,
    ) -> CompilerError {
        let mut log = ArtifactEventLog::new();
        let mut event = ArtifactEvent::new("commit.unresolved")
            .error()
            .text("kind", kind)
            .text("module", DumpContext::new(self).module_label(module))
            .text(target_key, self.dump_in_module(module, &target))
            .text("operand", self.dump_in_module(module, &operand));

        // include bounds for open variable operands
        if let StaticOperand::Variable(variable) = operand {
            let lower = self
                .inference
                .lower_static_bounds(variable)
                .into_iter()
                .map(|bound| self.dump_in_module(module, &bound))
                .collect::<Vec<_>>()
                .join(",");
            let upper = self
                .inference
                .upper_static_bounds(variable)
                .into_iter()
                .map(|bound| self.dump_in_module(module, &bound))
                .collect::<Vec<_>>()
                .join(",");

            event = event.text("lower", format!("[{lower}]"));
            event = event.text("upper", format!("[{upper}]"));
        }

        log.push(event);

        self.unresolved_commit_error(log)
    }

    /// Return an internal error for one unresolved commit event.
    fn unresolved_commit_error(&self, log: ArtifactEventLog) -> CompilerError {
        let report = log.render_plain();
        let trace = self.emit_events.then(|| self.event_dump());

        let message = if let Some(trace) = trace {
            format!(
                "check internal error
---------------
{report}
---------------
check trace
---------------
{trace}
---------------"
            )
        } else {
            format!(
                "check internal error
---------------
{report}
---------------"
            )
        };

        CompilerError::Internal { message }
    }

    /// Return a compact DIR variant label for one node.
    fn node_variant(&self, node: dir::GlobalNodeIdAny) -> &'static str {
        let view = self.module(node.module_id).view();

        match node.local_id.ty {
            dir::NodeType::Expression => {
                let id = dir::LocalNodeId::new(node.local_id.id);

                Self::expression_variant(view.get(id))
            }
            dir::NodeType::TypeExpression => {
                let id = dir::LocalNodeId::new(node.local_id.id);

                Self::type_expression_variant(view.get(id))
            }
            dir::NodeType::Pattern => {
                let id = dir::LocalNodeId::new(node.local_id.id);

                Self::pattern_variant(view.get(id))
            }
            dir::NodeType::Block => "Block",
            dir::NodeType::Catch => "Catch",
            dir::NodeType::Declaration => "Declaration",
            dir::NodeType::Declarator => "Declarator",
            dir::NodeType::Property => "Property",
            dir::NodeType::TypeMember => "TypeMember",
            dir::NodeType::TypeMappedParameter => "TypeMappedParameter",
            dir::NodeType::Member => "Member",
            dir::NodeType::EnumField => "EnumField",
            dir::NodeType::WhereClause => "WhereClause",
            dir::NodeType::DependencyItem => "DependencyItem",
            dir::NodeType::GenericParameter => "GenericParameter",
            dir::NodeType::Parameter => "Parameter",
            dir::NodeType::GenericArgument => "GenericArgument",
            dir::NodeType::TupleElement => "TupleElement",
            dir::NodeType::Argument => "Argument",
            dir::NodeType::MatchCase => "MatchCase",
            dir::NodeType::PatternField => "PatternField",
            dir::NodeType::AssignPattern => "AssignPattern",
            dir::NodeType::AssignPatternField => "AssignPatternField",
            dir::NodeType::Decorator => "Decorator",
        }
    }

    /// Return a compact expression variant label.
    fn expression_variant(expression: &dir::Expression) -> &'static str {
        match expression {
            dir::Expression::Declaration(_) => "Expression.Declaration",
            dir::Expression::Block(_) => "Expression.Block",
            dir::Expression::Label { .. } => "Expression.Label",
            dir::Expression::Import { .. } => "Expression.Import",
            dir::Expression::Export { .. } => "Expression.Export",
            dir::Expression::Let { .. } => "Expression.Let",
            dir::Expression::LetElse { .. } => "Expression.LetElse",
            dir::Expression::Using { .. } => "Expression.Using",
            dir::Expression::If { .. } => "Expression.If",
            dir::Expression::While { .. } => "Expression.While",
            dir::Expression::ForEach { .. } => "Expression.ForEach",
            dir::Expression::For { .. } => "Expression.For",
            dir::Expression::Loop { .. } => "Expression.Loop",
            dir::Expression::Try { .. } => "Expression.Try",
            dir::Expression::Match { .. } => "Expression.Match",
            dir::Expression::Break { .. } => "Expression.Break",
            dir::Expression::Continue { .. } => "Expression.Continue",
            dir::Expression::Await { .. } => "Expression.Await",
            dir::Expression::AwaitMaybe { .. } => "Expression.AwaitMaybe",
            dir::Expression::AwaitMust { .. } => "Expression.AwaitMust",
            dir::Expression::Yield { .. } => "Expression.Yield",
            dir::Expression::Throw { .. } => "Expression.Throw",
            dir::Expression::Return { .. } => "Expression.Return",
            dir::Expression::Identifier { .. } => "Expression.Identifier",
            dir::Expression::QualifiedReference { .. } => "Expression.QualifiedReference",
            dir::Expression::PrivateIdentifier { .. } => "Expression.PrivateIdentifier",
            dir::Expression::This => "Expression.This",
            dir::Expression::Super => "Expression.Super",
            dir::Expression::ImportMeta => "Expression.ImportMeta",
            dir::Expression::ScalarLiteral(_) => "Expression.ScalarLiteral",
            dir::Expression::RangeExpression { .. } => "Expression.RangeExpression",
            dir::Expression::TemplateExpression { .. } => "Expression.TemplateExpression",
            dir::Expression::TaggedTemplateExpression { .. } => {
                "Expression.TaggedTemplateExpression"
            }
            dir::Expression::ArrayExpression { .. } => "Expression.ArrayExpression",
            dir::Expression::FixedArrayExpression { .. } => "Expression.FixedArrayExpression",
            dir::Expression::TupleExpression { .. } => "Expression.TupleExpression",
            dir::Expression::SequenceExpression { .. } => "Expression.SequenceExpression",
            dir::Expression::ObjectExpression { .. } => "Expression.ObjectExpression",
            dir::Expression::StructExpression { .. } => "Expression.StructExpression",
            dir::Expression::TreeExpression { .. } => "Expression.TreeExpression",
            dir::Expression::Parenthesized { .. } => "Expression.Parenthesized",
            dir::Expression::Type { .. } => "Expression.Type",
            dir::Expression::Comptime { .. } => "Expression.Comptime",
            dir::Expression::As { .. } => "Expression.As",
            dir::Expression::Satisfies { .. } => "Expression.Satisfies",
            dir::Expression::Is { .. } => "Expression.Is",
            dir::Expression::InstanceOf { .. } => "Expression.InstanceOf",
            dir::Expression::Unary { .. } => "Expression.Unary",
            dir::Expression::MoveOf { .. } => "Expression.MoveOf",
            dir::Expression::BorrowOf { .. } => "Expression.BorrowOf",
            dir::Expression::Member { .. } => "Expression.Member",
            dir::Expression::PrivateMember { .. } => "Expression.PrivateMember",
            dir::Expression::Index { .. } => "Expression.Index",
            dir::Expression::Instantiation { .. } => "Expression.Instantiation",
            dir::Expression::Call { .. } => "Expression.Call",
            dir::Expression::New { .. } => "Expression.New",
            dir::Expression::NewMaybe { .. } => "Expression.NewMaybe",
            dir::Expression::Maybe { .. } => "Expression.Maybe",
            dir::Expression::Must { .. } => "Expression.Must",
            dir::Expression::Binary { .. } => "Expression.Binary",
            dir::Expression::Assign { .. } => "Expression.Assign",
            dir::Expression::Debugger => "Expression.Debugger",
            dir::Expression::Missing => "Expression.Missing",
            dir::Expression::Stub => "Expression.Stub",
            dir::Expression::Error => "Expression.Error",
        }
    }

    /// Return a compact type expression variant label.
    fn type_expression_variant(expression: &dir::TypeExpression) -> &'static str {
        match expression {
            dir::TypeExpression::Parenthesized { .. } => "TypeExpression.Parenthesized",
            dir::TypeExpression::ScalarLiteral { .. } => "TypeExpression.ScalarLiteral",
            dir::TypeExpression::Literal { .. } => "TypeExpression.Literal",
            dir::TypeExpression::Intrinsic => "TypeExpression.Intrinsic",
            dir::TypeExpression::Tuple { .. } => "TypeExpression.Tuple",
            dir::TypeExpression::ArrayTuple { .. } => "TypeExpression.ArrayTuple",
            dir::TypeExpression::Array { .. } => "TypeExpression.Array",
            dir::TypeExpression::Slice { .. } => "TypeExpression.Slice",
            dir::TypeExpression::FixedArray { .. } => "TypeExpression.FixedArray",
            dir::TypeExpression::Object { .. } => "TypeExpression.Object",
            dir::TypeExpression::Function(_) => "TypeExpression.Function",
            dir::TypeExpression::Constructor(_) => "TypeExpression.Constructor",
            dir::TypeExpression::Reference { .. } => "TypeExpression.Reference",
            dir::TypeExpression::Member { .. } => "TypeExpression.Member",
            dir::TypeExpression::Range { .. } => "TypeExpression.Range",
            dir::TypeExpression::Const => "TypeExpression.Const",
            dir::TypeExpression::This => "TypeExpression.This",
            dir::TypeExpression::Readonly { .. } => "TypeExpression.Readonly",
            dir::TypeExpression::Local { .. } => "TypeExpression.Local",
            dir::TypeExpression::Shared { .. } => "TypeExpression.Shared",
            dir::TypeExpression::KeyOf { .. } => "TypeExpression.KeyOf",
            dir::TypeExpression::TypeOfValue { .. } => "TypeExpression.TypeOfValue",
            dir::TypeExpression::Must { .. } => "TypeExpression.Must",
            dir::TypeExpression::Not { .. } => "TypeExpression.Not",
            dir::TypeExpression::OwnedOf { .. } => "TypeExpression.OwnedOf",
            dir::TypeExpression::BorrowedOf { .. } => "TypeExpression.BorrowedOf",
            dir::TypeExpression::PointerOf { .. } => "TypeExpression.PointerOf",
            dir::TypeExpression::Union { .. } => "TypeExpression.Union",
            dir::TypeExpression::Intersection { .. } => "TypeExpression.Intersection",
            dir::TypeExpression::Conditional { .. } => "TypeExpression.Conditional",
            dir::TypeExpression::Extends { .. } => "TypeExpression.Extends",
            dir::TypeExpression::Implements { .. } => "TypeExpression.Implements",
            dir::TypeExpression::Mapped { .. } => "TypeExpression.Mapped",
            dir::TypeExpression::Index { .. } => "TypeExpression.Index",
            dir::TypeExpression::TemplateLiteral { .. } => "TypeExpression.TemplateLiteral",
            dir::TypeExpression::Infer { .. } => "TypeExpression.Infer",
            dir::TypeExpression::Missing => "TypeExpression.Missing",
            dir::TypeExpression::Error => "TypeExpression.Error",
        }
    }

    /// Return a compact pattern variant label.
    fn pattern_variant(pattern: &dir::Pattern) -> &'static str {
        match pattern {
            dir::Pattern::Wildcard => "Pattern.Wildcard",
            dir::Pattern::Must(_) => "Pattern.Must",
            dir::Pattern::Assign { .. } => "Pattern.Assign",
            dir::Pattern::BorrowOf { .. } => "Pattern.BorrowOf",
            dir::Pattern::MoveOf { .. } => "Pattern.MoveOf",
            dir::Pattern::DereferenceOf { .. } => "Pattern.DereferenceOf",
            dir::Pattern::Binding { .. } => "Pattern.Binding",
            dir::Pattern::Expression { .. } => "Pattern.Expression",
            dir::Pattern::Range { .. } => "Pattern.Range",
            dir::Pattern::TypeExpression { .. } => "Pattern.TypeExpression",
            dir::Pattern::Tuple { .. } => "Pattern.Tuple",
            dir::Pattern::Newtype { .. } => "Pattern.Newtype",
            dir::Pattern::Sequence { .. } => "Pattern.Sequence",
            dir::Pattern::Object { .. } => "Pattern.Object",
            dir::Pattern::NominalObject { .. } => "Pattern.NominalObject",
            dir::Pattern::Union { .. } => "Pattern.Union",
        }
    }

    /// Return compact parent labels for one node.
    fn node_parent_variants(&self, node: dir::GlobalNodeIdAny) -> String {
        let view = self.module(node.module_id).view();
        let mut labels = Vec::new();
        let mut current = node.local_id;

        // collect nearest parents first
        while let Some(parent) = view.get_parent_any(current) {
            labels.push(self.node_variant(parent.into_global(node.module_id)));
            current = parent;
        }

        format!("[{}]", labels.join(","))
    }

    /// Return active constraints that mention one variable.
    fn variable_constraints(&self, module: ModuleId, variable: VariableId) -> String {
        let constraints = self
            .inference
            .constraints_with_ids()
            .filter(|(_, constraint)| Self::constraint_references_variable(constraint, variable))
            .map(|(id, constraint)| {
                let task = Task::Constraint(id);
                let dependencies = self
                    .inference
                    .task_dependencies(task)
                    .map(|dependency| self.dump_in_module(module, &dependency))
                    .collect::<Vec<_>>()
                    .join(",");
                let state = if self.inference.is_constraint_complete(id) {
                    "complete"
                } else {
                    "open"
                };

                format!(
                    "<ConstraintRef id={} state={state} blocked_by=[{}] value={}>",
                    id.index,
                    dependencies,
                    self.dump_in_module(module, constraint)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        format!("[{constraints}]")
    }

    /// Return whether one constraint mentions one variable directly.
    fn constraint_references_variable(constraint: &Constraint, variable: VariableId) -> bool {
        match constraint {
            Constraint::Type { left, right, .. } => {
                left.variable() == Some(variable) || right.variable() == Some(variable)
            }
            Constraint::Static { left, right, .. } => {
                left.variable() == Some(variable) || right.variable() == Some(variable)
            }
            Constraint::Pattern { value, .. } => value.variable() == Some(variable),
        }
    }
}
