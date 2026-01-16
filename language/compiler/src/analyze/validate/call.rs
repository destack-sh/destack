use crate::{AnalyzeError, AnalyzeOptions, Compiler};
use destack_dir::{
    Expression, GlobalSymbolId, LocalNodeId, NodeTree, SymbolTable, WellKnownSymbol,
};
use destack_workspace::{Module, ModuleSource, ProfileId, SymbolGroup};

impl Compiler {
    /// Validate call expressions against runtime restriction options.
    pub(crate) fn validate_call_expression(
        &self,
        module: &Module,
        expression_id: LocalNodeId<Expression>,
        callee_id: LocalNodeId<Expression>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        options: &AnalyzeOptions,
        profile: ProfileId,
        is_constructor: bool,
    ) {
        // skip restrictions for non-user modules
        if !matches!(module.source, ModuleSource::User) {
            return;
        }

        // skip work when no relevant restrictions are enabled
        if !options.no_dynamic_evaluation && !options.no_proxy && !options.no_dynamic_shapes {
            return;
        }

        // unwrap nested parentheses before classification
        let callee_id = self.unwrap_parenthesized_expression(callee_id, tree);

        // skip when well-known symbols are not available
        let Some(well_known) = self.get_well_known_symbols(profile) else {
            return;
        };

        // resolve well known globals used in restrictions
        let eval_symbol = well_known.get_symbol(WellKnownSymbol::Eval);
        let function_symbol = well_known.get_symbol(WellKnownSymbol::Function);
        let proxy_symbol = well_known.get_symbol(WellKnownSymbol::Proxy);
        let object_group = well_known.get_group(WellKnownSymbol::Object);
        let reflect_group = well_known.get_group(WellKnownSymbol::Reflect);

        // resolve well known member names for restricted shape mutation
        let define_property_name = self.program.strings.intern("defineProperty");
        let define_properties_name = self.program.strings.intern("defineProperties");
        let set_prototype_of_name = self.program.strings.intern("setPrototypeOf");

        // resolve the canonical symbol for a global reference
        let canonical_symbol =
            |symbol: GlobalSymbolId| self.canonical_symbol_id(module, symbols, profile, symbol);

        // check whether a symbol is contained in a well-known group
        let group_contains = |group: Option<SymbolGroup>, symbol: GlobalSymbolId| {
            group.is_some_and(|group| group.ty == Some(symbol) || group.value == Some(symbol))
        };

        match tree.get(callee_id) {
            Expression::GlobalReference { target_symbol, .. }
            | Expression::LocalReference { target_symbol, .. }
            | Expression::ModuleReference { target_symbol, .. } => {
                // resolve the canonical symbol for the callee
                let callee_symbol = canonical_symbol(*target_symbol);

                // report dynamic evaluation for builtin eval or Function
                if options.no_dynamic_evaluation
                    && (Some(callee_symbol) == function_symbol
                        || (!is_constructor && Some(callee_symbol) == eval_symbol))
                {
                    self.error(AnalyzeError::DynamicEvaluationDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }

                // report Proxy usage when disabled
                if options.no_proxy && Some(callee_symbol) == proxy_symbol {
                    self.error(AnalyzeError::ProxyDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            }
            Expression::Member { left, name, .. } => {
                // short circuit when dynamic shapes are allowed
                if !options.no_dynamic_shapes {
                    return;
                }

                // resolve the owner symbol for shape mutation checks
                let base_symbol = tree.get(*left).target_symbol().map(canonical_symbol);
                let is_shape_mutation = matches!(
                    name,
                    name_id
                        if *name_id == define_property_name
                            || *name_id == define_properties_name
                            || *name_id == set_prototype_of_name
                );
                let is_shape_owner = base_symbol
                    .is_some_and(|symbol| group_contains(object_group, symbol))
                    || base_symbol.is_some_and(|symbol| group_contains(reflect_group, symbol));

                if is_shape_mutation && is_shape_owner {
                    self.error(AnalyzeError::DynamicShapesDisabled {
                        node: expression_id
                            .into_global_any(module.id)
                            .into_anchored(Some(profile)),
                    });
                }
            }
            _ => {}
        }
    }
}
