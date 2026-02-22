use crate::analyze::common::CanonicalSymbolMode;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, Generics, GlobalSymbolId, Heritage, LocalNodeId, Member, NodeTree, Parameter,
    StringId, SymbolTable, TypeTable,
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
    /// Report missing declared associated type requirements for one declaration.
    pub(crate) fn report_missing_declared_associated_type_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_symbol: GlobalSymbolId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_associated_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        self.check_declared_associated_requirements(
            module,
            profile,
            declaration_symbol,
            heritage,
            members,
            allows_deferred_associated_types,
            DeclaredAssociatedRequirementKind::Type,
            tree,
            symbols,
            types,
        )
    }

    /// Report missing declared associated comptime requirements for one declaration.
    pub(crate) fn report_missing_declared_associated_comptime_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_symbol: GlobalSymbolId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_associated_comptime: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        self.check_declared_associated_requirements(
            module,
            profile,
            declaration_symbol,
            heritage,
            members,
            allows_deferred_associated_comptime,
            DeclaredAssociatedRequirementKind::Comptime,
            tree,
            symbols,
            types,
        )
    }

    /// Check declared associated requirements for one declaration.
    #[allow(clippy::too_many_arguments)]
    fn check_declared_associated_requirements(
        &self,
        module: &Module,
        profile: ProfileId,
        declaration_symbol: GlobalSymbolId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_requirements: bool,
        requirement_kind: DeclaredAssociatedRequirementKind,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<()> {
        let declaration_symbol = self
            .declaration_symbol_id(module, symbols, profile, declaration_symbol)
            .unwrap_or(declaration_symbol);

        // skip non-user modules
        if !matches!(module.source, ModuleSource::User) {
            return Ok(());
        }

        // collect inherited contract expressions
        let contract_expressions = self.contract_expression_ids_for_heritage(heritage);
        if contract_expressions.is_empty() {
            return Ok(());
        }

        // collect declaration associated names for this requirement category
        let declared_associated_names =
            self.declared_associated_member_names(requirement_kind, members, tree);

        // report or mark missing requirements once per associated name
        let mut reported_missing_names = HashSet::new();
        for expression_id in contract_expressions {
            let Some(target_symbol) = self.inherited_contract_symbol_for_expression(
                module,
                profile,
                expression_id,
                tree,
                symbols,
                types,
            )?
            else {
                continue;
            };

            let requirements = self.collect_declared_associated_requirement_pairs_for_contract(
                requirement_kind,
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            )?;

            for (requirement_name, requires_implementation) in requirements {
                if !requires_implementation
                    || declared_associated_names.contains(&requirement_name)
                    || allows_deferred_requirements
                    || !reported_missing_names.insert(requirement_name)
                {
                    continue;
                }

                types.mark_symbol_with_unimplemented_associated_requirements(declaration_symbol);
                let node = expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
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
    fn collect_declared_associated_requirement_pairs_for_contract(
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
        module: &Module,
        profile: ProfileId,
        expression_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // resolve direct symbol links through canonical declaration ownership
        if let Some(target_symbol) = tree.get(expression_id).target_symbol() {
            let mut target_symbol = self.canonical_symbol_id(
                module,
                symbols,
                profile,
                target_symbol,
                CanonicalSymbolMode::FollowAliases,
            );
            target_symbol =
                self.resolve_type_reference_symbol(module, profile, target_symbol, tree, symbols);
            if let Some(target_symbol) =
                self.declaration_symbol_id(module, symbols, profile, target_symbol)
            {
                return Ok(Some(target_symbol));
            }
        }

        // otherwise evaluate the heritage expression to resolve the target symbol
        let inherited_contract_type_id = self.resolve_declared_type_expression(
            module,
            profile,
            expression_id,
            tree,
            symbols,
            types,
            true,
            true,
        )?;
        let target_symbol = self
            .unwrap_type_value_symbol(types, inherited_contract_type_id)
            .map(|target_symbol| {
                let mut target_symbol = self.canonical_symbol_id(
                    module,
                    symbols,
                    profile,
                    target_symbol,
                    CanonicalSymbolMode::FollowAliases,
                );
                target_symbol = self.resolve_type_reference_symbol(
                    module,
                    profile,
                    target_symbol,
                    tree,
                    symbols,
                );
                self.declaration_symbol_id(module, symbols, profile, target_symbol)
                    .unwrap_or(target_symbol)
            });

        Ok(target_symbol)
    }

    /// Declare type-member aliases and their generics for one declaration.
    pub(crate) fn collect_associated_type_members(
        &self,
        module: &Module,
        profile: ProfileId,
        members: &[LocalNodeId<Member>],
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        for member_id in members {
            let Member::Type {
                static_parameters,
                where_clauses,
                ty,
                value,
                ..
            } = tree.get(*member_id)
            else {
                continue;
            };

            self.collect_associated_type_member(
                module,
                profile,
                *member_id,
                static_parameters.as_deref(),
                where_clauses.as_deref(),
                *ty,
                *value,
                tree,
                symbols,
                types,
                defer_type_evaluation,
            )?;
        }

        Ok(())
    }

    /// Declare one type-member alias and its generic context.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn collect_associated_type_member(
        &self,
        module: &Module,
        profile: ProfileId,
        member_id: LocalNodeId<Member>,
        static_parameters: Option<&[LocalNodeId<Parameter>]>,
        where_clauses: Option<&[LocalNodeId<destack_dir::WhereClause>]>,
        ty: Option<LocalNodeId<Expression>>,
        value: Option<LocalNodeId<Expression>>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        defer_type_evaluation: bool,
    ) -> AnalyzeResult<()> {
        // declare associated type generics
        let member_generics = Generics {
            static_parameters: static_parameters
                .map(|static_parameters| static_parameters.to_vec()),
            where_clauses: where_clauses.map(|where_clauses| where_clauses.to_vec()),
        };
        self.collect_generics(module, profile, &member_generics, tree, symbols, types)?;

        // resolve associated type bound
        if let Some(ty) = ty {
            let bound_ty_id = self.collect_or_defer_type_expression(
                module,
                profile,
                ty,
                tree,
                symbols,
                types,
                defer_type_evaluation,
            )?;
            types.set_declared_type(ty.into_global_any(module.id), bound_ty_id);
        }

        // resolve and register associated type default
        if let Some(value) = value {
            let value_ty_id = self.collect_or_defer_type_expression(
                module,
                profile,
                value,
                tree,
                symbols,
                types,
                defer_type_evaluation,
            )?;
            types.set_declared_type(value.into_global_any(module.id), value_ty_id);

            let member_symbol = tree.get(member_id).symbol();
            let member_symbol_entry = symbols.get_symbol(member_symbol);
            let member_symbol =
                GlobalSymbolId::new(module.id, member_symbol.with_type(member_symbol_entry.ty));
            types.set_alias_target_type_id(member_symbol, value_ty_id);
            types.set_instance_type(member_symbol, value_ty_id);
        }

        Ok(())
    }
}
