use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    Expression, Generics, GlobalSymbolId, Heritage, LocalNodeId, Member, NodeTree, Parameter,
    SymbolTable, TypeTable,
};
use destack_workspace::{Module, ModuleSource, ProfileId};
use std::collections::HashSet;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Validate associated type contract presence for one declaration.
    pub(crate) fn validate_associated_type_contract_presence(
        &self,
        module: &Module,
        profile: ProfileId,
        heritage: &Heritage,
        members: &[LocalNodeId<Member>],
        allows_deferred_associated_types: bool,
        tree: &NodeTree,
        symbols: &SymbolTable,
    ) -> AnalyzeResult<()> {
        // skip non-user modules
        if !matches!(module.source, ModuleSource::User) {
            return Ok(());
        }

        // collect declaration associated type names
        let mut declared_associated_names = HashSet::new();
        for member_id in members {
            if let Member::Type { name, .. } = tree.get(*member_id) {
                declared_associated_names.insert(*name);
            }
        }

        // collect inherited contract expressions
        let mut contract_expressions = Vec::new();
        if let Some(extends_types) = heritage.extends_types.as_ref() {
            contract_expressions.extend(extends_types.iter().copied());
        }
        if let Some(implements_types) = heritage.implements_types.as_ref() {
            contract_expressions.extend(implements_types.iter().copied());
        }
        if contract_expressions.is_empty() {
            return Ok(());
        }

        // report missing requirements once per associated name
        let mut reported_missing_names = HashSet::new();
        for expression_id in contract_expressions {
            let Some(target_symbol) = tree.get(expression_id).target_symbol() else {
                continue;
            };
            let requirements = self.collect_contract_associated_type_requirements(
                module,
                profile,
                target_symbol,
                tree,
                symbols,
            )?;

            for requirement in requirements {
                if !requirement.requires_implementation
                    || declared_associated_names.contains(&requirement.name)
                    || allows_deferred_associated_types
                    || !reported_missing_names.insert(requirement.name)
                {
                    continue;
                }

                let node = expression_id
                    .into_global_any(module.id)
                    .into_anchored(Some(profile));
                self.error(AnalyzeError::InvalidStaticArgument {
                    node,
                    message: "missing associated type implementation".to_string(),
                });
            }
        }

        Ok(())
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
