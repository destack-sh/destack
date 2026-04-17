use crate::analyze::common::{CanonicalSymbolMode, TreeSymbolView, TypeContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    GenericParameter, GlobalSymbolId, LocalNodeId, LocalTypeId, Member, NodeTree, StringId,
    TypeExpression, TypeMember, WhereClause,
};
use destack_workspace::ModuleSource;
use std::collections::HashSet;

/// The associated requirement category for declaration checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeclaredAssociatedRequirementKind {
    /// Associated type requirements.
    Type,
    /// Associated comptime requirements.
    Comptime,
}

impl DeclaredAssociatedRequirementKind {
    /// Return the missing-requirement diagnostic message for this category.
    fn missing_requirement_message(self) -> &'static str {
        match self {
            Self::Type => "missing associated type implementation",
            Self::Comptime => "missing associated comptime implementation",
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect one associated member type expression, deferring only when eager resolution yields.
    fn collect_associated_member_type_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<LocalTypeId> {
        if defer_type_evaluation {
            return self.resolve_declared_type_expression(
                &mut ctx.reborrow(),
                expression_id,
                true,
                true,
            );
        }

        let type_id =
            self.query_declared_type_expression(&mut ctx.reborrow(), expression_id, true, true)?;
        if let Some(type_id) = type_id {
            return Ok(type_id);
        }

        self.resolve_declared_type_expression(&mut ctx.reborrow(), expression_id, true, true)
    }

    /// Report missing declared associated requirements for one declaration in one ctx context.
    pub(crate) fn report_missing_associated_requirements(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_symbol: GlobalSymbolId,
        extends_types: &[LocalNodeId<TypeExpression>],
        implements_types: &[LocalNodeId<TypeExpression>],
        members: &[LocalNodeId<Member>],
        allows_deferred_requirements: bool,
    ) -> AnalyzeResult<()> {
        self.check_associated_requirements(
            &mut ctx.reborrow(),
            declaration_symbol,
            extends_types,
            implements_types,
            members,
            allows_deferred_requirements,
            DeclaredAssociatedRequirementKind::Type,
        )?;

        self.check_associated_requirements(
            ctx,
            declaration_symbol,
            extends_types,
            implements_types,
            members,
            allows_deferred_requirements,
            DeclaredAssociatedRequirementKind::Comptime,
        )?;

        Ok(())
    }

    /// Check declared associated requirements for one declaration.
    fn check_associated_requirements(
        &self,
        ctx: &mut TypeContext<'_>,
        declaration_symbol: GlobalSymbolId,
        extends_types: &[LocalNodeId<TypeExpression>],
        implements_types: &[LocalNodeId<TypeExpression>],
        members: &[LocalNodeId<Member>],
        allows_deferred_requirements: bool,
        requirement_kind: DeclaredAssociatedRequirementKind,
    ) -> AnalyzeResult<()> {
        let declaration_symbol = self
            .declaration_symbol_id(ctx.module_symbol_view(), declaration_symbol)
            .unwrap_or(declaration_symbol);

        // skip non-user modules
        if !matches!(ctx.module.source, ModuleSource::User) {
            return Ok(());
        }

        // collect inherited contract expressions
        let contract_expressions =
            self.contract_expression_ids_for_lineage(extends_types, implements_types);
        if contract_expressions.is_empty() {
            return Ok(());
        }

        // collect declaration associated names for this requirement category
        let declared_associated_names =
            self.declared_associated_member_names(requirement_kind, members, ctx.tree);

        // report or mark missing requirements once per associated name
        let mut reported_missing_names = HashSet::new();
        for expression_id in contract_expressions {
            let Some(target_symbol) =
                self.inherited_contract_symbol_for_expression(&mut ctx.reborrow(), expression_id)?
            else {
                continue;
            };

            let requirements = self.collect_associated_requirement_pairs_for_contract(
                requirement_kind,
                ctx.tree_symbol_view(),
                target_symbol,
            )?;

            for (requirement_name, requires_implementation) in requirements {
                if !requires_implementation
                    || declared_associated_names.contains(&requirement_name)
                    || allows_deferred_requirements
                    || !reported_missing_names.insert(requirement_name)
                {
                    continue;
                }

                ctx.types
                    .mark_symbol_with_unimplemented_associated_requirements(declaration_symbol);
                let node = expression_id
                    .into_global_any(ctx.module.id)
                    .into_anchored(Some(ctx.profile));
                self.error(AnalyzeError::InvalidStaticArgument {
                    node,
                    message: requirement_kind.missing_requirement_message().to_string(),
                });
            }
        }

        Ok(())
    }

    /// Return declaration associated names for one requirement category.
    fn declared_associated_member_names(
        &self,
        requirement_kind: DeclaredAssociatedRequirementKind,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
    ) -> HashSet<StringId> {
        let mut names = HashSet::new();
        for member_id in members {
            let name = match (requirement_kind, tree.get(*member_id)) {
                (DeclaredAssociatedRequirementKind::Type, Member::AssociatedType { name, .. })
                | (
                    DeclaredAssociatedRequirementKind::Comptime,
                    Member::AssociatedConst { name, .. },
                ) => Some(*name),
                _ => None,
            };
            if let Some(name) = name {
                names.insert(name);
            }
        }

        names
    }

    /// Return direct contract expression ids from declaration lineage.
    fn contract_expression_ids_for_lineage(
        &self,
        extends_types: &[LocalNodeId<TypeExpression>],
        implements_types: &[LocalNodeId<TypeExpression>],
    ) -> Vec<LocalNodeId<TypeExpression>> {
        let mut contract_expressions = Vec::new();
        contract_expressions.extend(extends_types.iter().copied());
        contract_expressions.extend(implements_types.iter().copied());

        contract_expressions
    }

    /// Collect requirement name or required-implementation pairs for one contract.
    fn collect_associated_requirement_pairs_for_contract(
        &self,
        requirement_kind: DeclaredAssociatedRequirementKind,
        ctx: TreeSymbolView<'_>,
        contract_symbol: GlobalSymbolId,
    ) -> AnalyzeResult<Vec<(StringId, bool)>> {
        let requirements = match requirement_kind {
            DeclaredAssociatedRequirementKind::Type => self
                .collect_contract_associated_type_requirements(ctx, contract_symbol)?
                .into_iter()
                .map(|requirement| (requirement.name, requirement.requires_implementation))
                .collect(),
            DeclaredAssociatedRequirementKind::Comptime => self
                .collect_contract_associated_comptime_requirements(ctx, contract_symbol)?
                .into_iter()
                .map(|requirement| (requirement.name, requirement.requires_implementation))
                .collect(),
        };

        Ok(requirements)
    }

    /// Resolve one inherited contract symbol from one heritage expression.
    fn inherited_contract_symbol_for_expression(
        &self,
        ctx: &mut TypeContext<'_>,
        expression_id: LocalNodeId<TypeExpression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        let expression = ctx.tree.get(expression_id);

        // resolve direct symbol links through canonical declaration ownership
        let target_symbol = expression.target_symbol();
        if let Some(target_symbol) = target_symbol {
            let mut target_symbol = self.canonical_symbol_id(
                ctx.module_symbol_view(),
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            target_symbol = self.resolve_type_reference_symbol(ctx, target_symbol);
            if let Some(target_symbol) =
                self.declaration_symbol_id(ctx.module_symbol_view(), target_symbol)
            {
                return Ok(Some(target_symbol));
            }
        }

        Ok(None)
    }

    /// Declare type-member aliases and their generics for one declaration in one ctx context.
    pub(crate) fn collect_associated_type_members(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<Member>],
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        for member_id in members {
            let Member::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } = ctx.tree.get(*member_id)
            else {
                continue;
            };

            self.collect_associated_type_member(
                &mut ctx.reborrow(),
                *member_id,
                generic_parameters,
                where_clauses,
                *constraint,
                *value,
                defer_type_evaluation,
            )?;
        }

        Ok(())
    }

    /// Declare type-surface aliases and their generics for one declaration in one ctx context.
    pub(crate) fn collect_type_member_associated_types(
        &self,
        ctx: &mut TypeContext<'_>,
        members: &[LocalNodeId<TypeMember>],
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        for member_id in members {
            let TypeMember::AssociatedType {
                generic_parameters,
                where_clauses,
                constraint,
                value,
                ..
            } = ctx.tree.get(*member_id)
            else {
                continue;
            };

            self.collect_type_member_associated_type(
                &mut ctx.reborrow(),
                *member_id,
                generic_parameters,
                where_clauses,
                *constraint,
                *value,
                defer_type_evaluation,
            )?;
        }

        Ok(())
    }

    /// Declare one type-member alias and its generic context.
    pub(crate) fn collect_associated_type_member(
        &self,
        ctx: &mut TypeContext<'_>,
        member_id: LocalNodeId<Member>,
        generic_parameters: &[LocalNodeId<GenericParameter>],
        _where_clauses: &[LocalNodeId<WhereClause>],
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        // declare associated type generic parameters
        self.collect_generics(&mut ctx.reborrow(), generic_parameters)?;

        // resolve associated type bound
        if let Some(declared_type) = declared_type {
            let bound_ty_id = self.collect_associated_member_type_expression(
                &mut ctx.reborrow(),
                declared_type,
                defer_type_evaluation,
            )?;
            ctx.types
                .set_declared_type(declared_type.into_global_any(ctx.module.id), bound_ty_id);
        }

        // resolve and register associated type default
        if let Some(value) = value {
            let value_ty_id = self.collect_associated_member_type_expression(
                &mut ctx.reborrow(),
                value,
                defer_type_evaluation,
            )?;
            ctx.types
                .set_declared_type(value.into_global_any(ctx.module.id), value_ty_id);

            let member_symbol = ctx.tree.get(member_id).symbol().into_global(ctx.module.id);
            let member_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), member_symbol)
                .unwrap_or(member_symbol);
            ctx.types
                .set_alias_target_type_id(member_symbol, value_ty_id);
            ctx.types.set_instance_type(member_symbol, value_ty_id);
        }

        Ok(())
    }

    /// Declare one type-surface alias and its generic context.
    pub(crate) fn collect_type_member_associated_type(
        &self,
        ctx: &mut TypeContext<'_>,
        member_id: LocalNodeId<TypeMember>,
        generic_parameters: &[LocalNodeId<GenericParameter>],
        _where_clauses: &[LocalNodeId<WhereClause>],
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        // declare associated type generic parameters
        self.collect_generics(&mut ctx.reborrow(), generic_parameters)?;

        // resolve associated type bound
        if let Some(declared_type) = declared_type {
            let bound_ty_id = self.collect_associated_member_type_expression(
                &mut ctx.reborrow(),
                declared_type,
                defer_type_evaluation,
            )?;
            ctx.types
                .set_declared_type(declared_type.into_global_any(ctx.module.id), bound_ty_id);
        }

        // resolve and register associated type default
        if let Some(value) = value {
            let value_ty_id = self.collect_associated_member_type_expression(
                &mut ctx.reborrow(),
                value,
                defer_type_evaluation,
            )?;
            ctx.types
                .set_declared_type(value.into_global_any(ctx.module.id), value_ty_id);

            let member_symbol = ctx.tree.get(member_id).symbol().into_global(ctx.module.id);
            let member_symbol = self
                .declaration_symbol_id(ctx.module_symbol_view(), member_symbol)
                .unwrap_or(member_symbol);
            ctx.types
                .set_alias_target_type_id(member_symbol, value_ty_id);
        }

        Ok(())
    }
}
