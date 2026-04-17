use crate::analyze::common::{ModuleTypeView, TypeContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, Member, NodeTree, NodeVisitor, NodeVisitorOptions,
    TypeMember, walk_expression,
};
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
            Expression::Member { .. } | Expression::PrivateMember { .. }
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
    /// Collect projection dependencies for associated comptime members.
    pub(crate) fn collect_member_projection_dependencies(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<Member>],
    ) {
        for member_id in members {
            let Member::AssociatedConst {
                value: Some(value_id),
                ..
            } = ctx.tree.get(*member_id)
            else {
                continue;
            };

            if !self.expression_has_projection_dependency(ctx.tree, *value_id) {
                continue;
            }

            let member_symbol = ctx.tree.get(*member_id).symbol().into_global(ctx.module.id);
            let member_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), member_symbol)
                .unwrap_or(member_symbol);
            ctx.types
                .mark_symbol_with_associated_comptime_projection_dependencies(member_symbol);
        }
    }

    /// Collect projection dependencies for type-surface associated comptime members.
    pub(crate) fn collect_type_member_projection_dependencies(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<TypeMember>],
    ) {
        for member_id in members {
            let TypeMember::AssociatedConst {
                value: Some(value_id),
                ..
            } = ctx.tree.get(*member_id)
            else {
                continue;
            };

            if !self.expression_has_projection_dependency(ctx.tree, *value_id) {
                continue;
            }

            let member_symbol = ctx.tree.get(*member_id).symbol().into_global(ctx.module.id);
            let member_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), member_symbol)
                .unwrap_or(member_symbol);
            ctx.types
                .mark_symbol_with_associated_comptime_projection_dependencies(member_symbol);
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
    pub(crate) fn symbol_has_projection_dependencies(
        &self,
        ctx: ModuleTypeView<'_>,
        symbol: GlobalSymbolId,
    ) -> AnalyzeResult<bool> {
        self.with_module_types_or_local_for_artifact(
            ctx.compiler_context,
            ctx.module,
            ctx.profile,
            symbol.module_id,
            ctx.types,
            destack_artifact::ArtifactKey::dir_declared,
            |_, owner_types| {
                owner_types.symbol_has_associated_comptime_projection_dependencies(symbol)
            },
        )
        .map_err(AnalyzeError::from)
    }
}
