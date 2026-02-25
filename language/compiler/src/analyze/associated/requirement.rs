use crate::analyze::common::{CanonicalSymbolMode, TypeTablesContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, Generics, GlobalSymbolId, Heritage, LocalNodeId, Member, NodeTree, Parameter,
    StringId, SymbolTable, WhereClause,
};
use destack_workspace::{Module, ModuleSource, ProfileId};
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
    /// Report missing declared associated requirements for one declaration in one tables context.
    pub(crate) fn report_missing_associated_requirements_in_tables(
        &self,
        tables: &mut TypeTablesContext<'_>,
        declaration_symbol: GlobalSymbolId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_requirements: bool,
    ) -> AnalyzeResult<()> {
        self.check_associated_requirements(
            &mut tables.reborrow(),
            declaration_symbol,
            heritage,
            members,
            allows_deferred_requirements,
            DeclaredAssociatedRequirementKind::Type,
        )?;

        self.check_associated_requirements(
            tables,
            declaration_symbol,
            heritage,
            members,
            allows_deferred_requirements,
            DeclaredAssociatedRequirementKind::Comptime,
        )?;

        Ok(())
    }

    /// Check declared associated requirements for one declaration.
    fn check_associated_requirements(
        &self,
        tables: &mut TypeTablesContext<'_>,
        declaration_symbol: GlobalSymbolId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_requirements: bool,
        requirement_kind: DeclaredAssociatedRequirementKind,
    ) -> AnalyzeResult<()> {
        let declaration_symbol = self
            .declaration_symbol_id(
                tables.module,
                tables.symbols,
                tables.profile,
                declaration_symbol,
            )
            .unwrap_or(declaration_symbol);

        // skip non-user modules
        if !matches!(tables.module.source, ModuleSource::User) {
            return Ok(());
        }

        // collect inherited contract expressions
        let contract_expressions = self.contract_expression_ids_for_heritage(heritage);
        if contract_expressions.is_empty() {
            return Ok(());
        }

        // collect declaration associated names for this requirement category
        let declared_associated_names =
            self.declared_associated_member_names(requirement_kind, members, tables.tree);

        // report or mark missing requirements once per associated name
        let mut reported_missing_names = HashSet::new();
        for expression_id in contract_expressions {
            let Some(target_symbol) = self
                .inherited_contract_symbol_for_expression(&mut tables.reborrow(), expression_id)?
            else {
                continue;
            };

            let requirements = self.collect_associated_requirement_pairs_for_contract(
                requirement_kind,
                tables.module,
                tables.profile,
                target_symbol,
                tables.tree,
                tables.symbols,
            )?;

            for (requirement_name, requires_implementation) in requirements {
                if !requires_implementation
                    || declared_associated_names.contains(&requirement_name)
                    || allows_deferred_requirements
                    || !reported_missing_names.insert(requirement_name)
                {
                    continue;
                }

                tables
                    .types
                    .mark_symbol_with_unimplemented_associated_requirements(declaration_symbol);
                let node = expression_id
                    .into_global_any(tables.module.id)
                    .into_anchored(Some(tables.profile));
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
                (DeclaredAssociatedRequirementKind::Type, Member::Type { name, .. })
                | (
                    DeclaredAssociatedRequirementKind::Comptime,
                    Member::ComptimeConst { name, .. },
                ) => Some(*name),
                _ => None,
            };
            if let Some(name) = name {
                names.insert(name);
            }
        }

        names
    }

    /// Return direct contract expression ids from one declaration heritage.
    fn contract_expression_ids_for_heritage(
        &self,
        heritage: &Heritage,
    ) -> Vec<LocalNodeId<Expression>> {
        let mut contract_expressions = Vec::new();
        if let Some(extends_types) = heritage.extends_types.as_ref() {
            contract_expressions.extend(extends_types.iter().copied());
        }
        if let Some(implements_types) = heritage.implements_types.as_ref() {
            contract_expressions.extend(implements_types.iter().copied());
        }

        contract_expressions
    }

    /// Collect requirement name or required-implementation pairs for one contract.
    fn collect_associated_requirement_pairs_for_contract(
        &self,
        requirement_kind: DeclaredAssociatedRequirementKind,
        module: &Module,
        profile: ProfileId,
        contract_symbol: GlobalSymbolId,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<Vec<(StringId, bool)>> {
        let requirements = match requirement_kind {
            DeclaredAssociatedRequirementKind::Type => self
                .collect_contract_associated_type_requirements(
                    module,
                    profile,
                    contract_symbol,
                    tree,
                    symbols,
                )?
                .into_iter()
                .map(|requirement| (requirement.name, requirement.requires_implementation))
                .collect(),
            DeclaredAssociatedRequirementKind::Comptime => self
                .collect_contract_associated_comptime_requirements(
                    module,
                    profile,
                    contract_symbol,
                    tree,
                    symbols,
                )?
                .into_iter()
                .map(|requirement| (requirement.name, requirement.requires_implementation))
                .collect(),
        };

        Ok(requirements)
    }

    /// Resolve one inherited contract symbol from one heritage expression.
    fn inherited_contract_symbol_for_expression(
        &self,
        tables: &mut TypeTablesContext<'_>,
        expression_id: LocalNodeId<Expression>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve direct symbol links through canonical declaration ownership
        if let Some(target_symbol) = tables.tree.get(expression_id).target_symbol() {
            let mut target_symbol = self.canonical_symbol_id(
                tables.module,
                tables.symbols,
                tables.profile,
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            target_symbol = self.resolve_type_reference_symbol(tables, target_symbol);
            if let Some(target_symbol) = self.declaration_symbol_id(
                tables.module,
                tables.symbols,
                tables.profile,
                target_symbol,
            ) {
                return Ok(Some(target_symbol));
            }
        }

        // otherwise evaluate the heritage expression to resolve the target symbol
        let inherited_contract_type_id = self.resolve_declared_type_expression(
            &mut tables.reborrow(),
            expression_id,
            true,
            true,
        )?;
        let target_symbol = self
            .unwrap_type_value_symbol(tables.types, inherited_contract_type_id)
            .map(|target_symbol| {
                let mut target_symbol = self.canonical_symbol_id(
                    tables.module,
                    tables.symbols,
                    tables.profile,
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                target_symbol = self.resolve_type_reference_symbol(tables, target_symbol);
                self.declaration_symbol_id(
                    tables.module,
                    tables.symbols,
                    tables.profile,
                    target_symbol,
                )
                .unwrap_or(target_symbol)
            });

        Ok(target_symbol)
    }

    /// Declare type-member aliases and their generics for one declaration in one tables context.
    pub(crate) fn collect_associated_type_members_in_tables(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        members: &[LocalNodeId<Member>],
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        for member_id in members {
            let Member::Type {
                static_parameters,
                where_clauses,
                ty,
                value,
                ..
            } = type_tables.tree.get(*member_id)
            else {
                continue;
            };

            self.collect_associated_type_member(
                &mut type_tables.reborrow(),
                *member_id,
                static_parameters.as_deref(),
                where_clauses.as_deref(),
                *ty,
                *value,
                defer_type_evaluation,
            )?;
        }

        Ok(())
    }

    /// Declare one type-member alias and its generic context.
    pub(crate) fn collect_associated_type_member(
        &self,
        type_tables: &mut TypeTablesContext<'_>,
        member_id: LocalNodeId<Member>,
        static_parameters: Option<&[LocalNodeId<Parameter>]>,
        where_clauses: Option<&[LocalNodeId<WhereClause>]>,
        ty: Option<LocalNodeId<Expression>>,
        value: Option<LocalNodeId<Expression>>,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        // declare associated type generics
        let member_generics = Generics {
            static_parameters: static_parameters
                .map(|static_parameters| static_parameters.to_vec()),
            where_clauses: where_clauses.map(|where_clauses| where_clauses.to_vec()),
        };
        self.collect_generics_in_tables(&mut type_tables.reborrow(), &member_generics)?;

        // resolve associated type bound
        if let Some(ty) = ty {
            let bound_ty_id = self.collect_or_defer_type_expression(
                &mut type_tables.reborrow(),
                ty,
                defer_type_evaluation,
            )?;
            type_tables
                .types
                .set_declared_type(ty.into_global_any(type_tables.module.id), bound_ty_id);
        }

        // resolve and register associated type default
        if let Some(value) = value {
            let value_ty_id = self.collect_or_defer_type_expression(
                &mut type_tables.reborrow(),
                value,
                defer_type_evaluation,
            )?;
            type_tables
                .types
                .set_declared_type(value.into_global_any(type_tables.module.id), value_ty_id);

            let member_symbol = type_tables.tree.get(member_id).symbol();
            let member_symbol_entry = type_tables.symbols.get_symbol(member_symbol);
            let member_symbol = GlobalSymbolId::new(
                type_tables.module.id,
                member_symbol.with_type(member_symbol_entry.ty),
            );
            type_tables
                .types
                .set_alias_target_type_id(member_symbol, value_ty_id);
            type_tables
                .types
                .set_instance_type(member_symbol, value_ty_id);
        }

        Ok(())
    }
}
