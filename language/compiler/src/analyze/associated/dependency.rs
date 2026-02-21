use crate::analyze::common::AnalyzeDependencyStage;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, Member, NodeTree, NodeVisitor, NodeVisitorOptions,
    SymbolTable, TypeTable, walk_expression,
};
use destack_workspace::{Module, ProfileId};

/// Walk one expression subtree and record whether projection dependency forms appear.
#[derive(Debug)]
struct ProjectionDependencyExpressionVisitor {
    /// Whether this subtree contains projection dependency forms.
    has_projection_dependency: bool,
    /// The visitor options.
    options: NodeVisitorOptions,
}

impl ProjectionDependencyExpressionVisitor {
    /// Create a projection dependency visitor.
    fn new() -> Self {
        Self {
            has_projection_dependency: false,
            options: NodeVisitorOptions::default(),
        }
    }
}

impl NodeVisitor for ProjectionDependencyExpressionVisitor {
    fn options(&self) -> &NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
        expression: &Expression,
    ) {
        // projection forms can trigger deferred static cycle diagnostics
        if matches!(
            expression,
            Expression::Member { .. }
                | Expression::PrivateMember { .. }
                | Expression::TypeIndex { .. }
        ) {
            self.has_projection_dependency = true;
            return;
        }

        // stop once the flag is set
        if self.has_projection_dependency {
            return;
        }

        // keep traversing nested expressions
        walk_expression(self, tree, expression_id, expression);
    }
}

impl Compiler {
    /// Collect projection-dependency facts for associated comptime members.
    pub(crate) fn collect_associated_comptime_member_projection_dependencies(
        &self,
        module: &Module,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) {
        for member_id in members {
            let Member::ComptimeConst {
                value: Some(value_id),
                ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            if !self.expression_has_projection_dependency(tree, *value_id) {
                continue;
            }

            let member_symbol = tree.get(*member_id).symbol();
            let member_symbol_entry = symbols.get_symbol(member_symbol);
            let member_symbol =
                GlobalSymbolId::new(module.id, member_symbol.with_type(member_symbol_entry.ty));
            types.mark_symbol_with_associated_comptime_projection_dependencies(member_symbol);
        }
    }

    /// Return true when one expression contains projection dependency forms.
    fn expression_has_projection_dependency(
        &self,
        tree: &NodeTree,
        expression_id: LocalNodeId<Expression>,
    ) -> bool {
        let mut visitor = ProjectionDependencyExpressionVisitor::new();
        visitor.visit_expression(tree, expression_id, tree.get(expression_id));
        visitor.has_projection_dependency
    }

    /// Return true when one associated comptime member symbol depends on projection forms.
    pub(crate) fn query_symbol_has_associated_comptime_projection_dependencies(
        &self,
        module: &Module,
        profile: ProfileId,
        symbol: GlobalSymbolId,
        _symbols: &SymbolTable,
        types: &TypeTable,
    ) -> AnalyzeResult<bool> {
        self.with_module_types_or_local_at_stage(
            module,
            profile,
            symbol.module_id,
            types,
            AnalyzeDependencyStage::Declare,
            |_, owner_types| {
                owner_types.symbol_has_associated_comptime_projection_dependencies(symbol)
            },
        )
        .map_err(AnalyzeError::from)
    }
}
