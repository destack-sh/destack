use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir as dir;

use crate::core::{DirQueryContext, ModuleQueryContext};

use super::parse_param_docs;

/// Parameter names and documentation collected from a declaration.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParameterList {
    /// The parameter display names in declared order.
    pub names: Vec<String>,
    /// Documentation keyed by parameter name.
    pub docs: HashMap<String, String>,
}

/// One expected-parameter hint for argument completion ranking.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ExpectedParameterHint {
    /// The parameter display name when available.
    pub name: Option<String>,
    /// The direct nominal type symbol when available.
    pub type_symbol: Option<dir::GlobalSymbolId>,
    /// Related nominal type symbols reachable through the declared type.
    pub type_symbols: Vec<dir::GlobalSymbolId>,
    /// Whether callable values are preferred.
    pub prefers_callable: bool,
    /// Whether constructable values are preferred.
    pub prefers_constructable: bool,
}

/// Format a parameter into a display name.
pub(crate) fn parameter_display_name(strings: &StringPool, parameter: &dir::Parameter) -> String {
    // choose the display name based on the parameter shape
    match parameter {
        dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
        dir::Parameter::VariadicNamed { name, .. } => {
            let name_str = strings.get(*name).to_string();
            format!("...{name_str}")
        }
        dir::Parameter::Pattern { .. } | dir::Parameter::VariadicPattern { .. } => {
            "<pattern>".to_string()
        }
        dir::Parameter::Error => "<error>".to_string(),
    }
}

/// Collect parameter display names from parameter nodes.
pub(crate) fn parameter_display_names(
    strings: &StringPool,
    tree: dir::View<'_>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
) -> Vec<String> {
    // collect parameter display names in declared order
    parameters
        .iter()
        .map(|param_id| {
            let param = tree.get::<dir::Parameter>(*param_id);
            parameter_display_name(strings, param)
        })
        .collect()
}

/// Resolve the parameter node that should guide one argument index.
fn resolve_expected_parameter_id(
    dir_tree: dir::View<'_>,
    parameters: &[dir::LocalNodeId<dir::Parameter>],
    parameter_index: usize,
) -> Option<dir::LocalNodeId<dir::Parameter>> {
    if let Some(parameter_id) = parameters.get(parameter_index) {
        return Some(*parameter_id);
    }

    let last_parameter_id = *parameters.last()?;
    let last_parameter = dir_tree.get::<dir::Parameter>(last_parameter_id);
    match last_parameter {
        dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
            Some(last_parameter_id)
        }
        _ => None,
    }
}

impl ModuleQueryContext<'_> {
    /// Get parameter names for a function symbol.
    pub(crate) fn parameter_names_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<Vec<String>> {
        // collect parameter list for the symbol declaration
        let data = self.parameter_list_for_symbol(symbol_id)?;

        if data.names.is_empty() {
            return None;
        }

        Some(data.names)
    }

    /// Collect parameter names and docs for a function or method symbol.
    pub(crate) fn parameter_list_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<ParameterList> {
        let ctx = self.module_context(symbol_id.module_id)?;
        ctx.parameter_list_for_symbol_with_context(symbol_id)
    }

    /// Classify the expected value shape for one parameter type.
    fn expected_value_shape(&self, type_id: dir::GlobalTypeId) -> (bool, bool) {
        self.with_global_type(type_id, |ty, _| match ty {
            dir::Type::Function(_) | dir::Type::Closure(_) => (true, false),
            dir::Type::Shape(object) => (
                !object.call_signatures.is_empty(),
                !object.construct_signatures.is_empty(),
            ),
            _ => (false, false),
        })
        .unwrap_or((false, false))
    }
}

impl DirQueryContext<'_> {
    /// Collect @param documentation from a declaration's doc comments.
    pub(crate) fn parameter_doc_map(
        self,
        source: &str,
        source_node_id: u32,
    ) -> HashMap<String, String> {
        // initialize the parameter doc map
        let mut param_docs = HashMap::new();

        // gather docs on the node or enclosing nodes
        let doc_strings = self.doc_strings_for_node_or_enclosing(source, source_node_id);

        // parse @param tags from collected docs
        for doc_text in doc_strings {
            param_docs.extend(parse_param_docs(&doc_text));
        }

        param_docs
    }
}

impl ModuleQueryContext<'_> {
    /// Collect parameter names and docs from one ready query context.
    fn parameter_list_for_symbol_with_context(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<ParameterList> {
        let ctx = self;
        // read the symbol declaration
        let global_node_id = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        // resolve the source text for doc parsing
        let source_file = ctx
            .repository()
            .file(ctx.revision(), ctx.file_id())
            .ok()
            .flatten()?;
        let source = source_file.text();

        // resolve parameter list based on the declaration node type
        let dir_tree = ctx.dir().view();
        match global_node_id.local_id.ty {
            // collect parameters from function declarations
            dir::NodeType::Declaration => {
                let declaration_id = global_node_id.local_id.try_into_typed().ok()?;
                let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };

                let source_node_id = dir_tree.get_source(declaration_id);
                let docs = ctx.dir().parameter_doc_map(source, source_node_id);
                let names = parameter_display_names(
                    ctx.dir().strings(),
                    dir_tree,
                    &declaration.signature.parameters,
                );

                Some(ParameterList { names, docs })
            }

            // collect parameters from method members
            dir::NodeType::Member => {
                let member_id = global_node_id.local_id.try_into_typed().ok()?;
                let member = dir_tree.get::<dir::Member>(member_id);
                let signature = member.signature()?;

                let source_node_id = dir_tree.get_source(member_id);
                let docs = ctx.dir().parameter_doc_map(source, source_node_id);
                let names =
                    parameter_display_names(ctx.dir().strings(), dir_tree, &signature.parameters);

                Some(ParameterList { names, docs })
            }

            _ => None,
        }
    }

    /// Resolve the expected-parameter hint for one active argument.
    pub(crate) fn expected_parameter_hint_for_symbol(
        &self,
        symbol_id: dir::GlobalSymbolId,
        parameter_index: usize,
    ) -> Option<ExpectedParameterHint> {
        let ctx = self;
        // read the target module and build a query context
        let ctx = ctx.module_context(symbol_id.module_id)?;

        // read the symbol declaration
        let global_node_id = {
            let symbols = ctx.dir().symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        // resolve the parameters for the declaration
        let dir_tree = ctx.dir().view();
        let parameters = match global_node_id.local_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = global_node_id.local_id.try_into_typed().ok()?;
                let declaration = dir_tree.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };

                declaration.signature.parameters.as_slice()
            }
            dir::NodeType::Member => {
                let member_id = global_node_id.local_id.try_into_typed().ok()?;
                let member = dir_tree.get::<dir::Member>(member_id);
                let signature = member.signature()?;

                signature.parameters.as_slice()
            }
            _ => return None,
        };

        // resolve the active parameter node
        let parameter_id = resolve_expected_parameter_id(dir_tree, parameters, parameter_index)?;
        let parameter = dir_tree.get::<dir::Parameter>(parameter_id);
        let name = Some(parameter_display_name(ctx.dir().strings(), parameter));

        // resolve the declared parameter type and classify its shape
        let global_parameter_id = parameter_id.into_global_any(ctx.module_id());
        let type_id = ctx.dir().types().get_node_type_id(global_parameter_id)?;
        let type_symbol = ctx
            .with_global_type(type_id, |ty, _| ty.symbol())
            .flatten()
            .map(|symbol_id| ctx.canonical_symbol(symbol_id));
        let type_symbols = ctx.collect_expected_type_symbols(type_id);
        let (prefers_callable, prefers_constructable) = ctx.expected_value_shape(type_id);

        Some(ExpectedParameterHint {
            name,
            type_symbol,
            type_symbols,
            prefers_callable,
            prefers_constructable,
        })
    }

    /// Collect nominal type symbols that should contribute to expected-type ranking.
    fn collect_expected_type_symbols(
        &self,
        type_id: dir::GlobalTypeId,
    ) -> Vec<dir::GlobalSymbolId> {
        let ctx = self;
        let mut symbols = Vec::new();
        let mut seen_types = HashSet::new();
        let mut seen_symbols = HashSet::new();

        ctx.collect_expected_type_symbols_inner(
            type_id,
            &mut seen_types,
            &mut seen_symbols,
            &mut symbols,
        );

        symbols
    }

    /// Collect nominal symbols from one declared parameter type.
    fn collect_expected_type_symbols_inner(
        &self,
        type_id: dir::GlobalTypeId,
        seen_types: &mut HashSet<dir::GlobalTypeId>,
        seen_symbols: &mut HashSet<dir::GlobalSymbolId>,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) {
        let ctx = self;
        let Some(type_id) = ctx.unwrap_global_form_payload_type_id(type_id) else {
            return;
        };
        if !seen_types.insert(type_id) {
            return;
        }

        ctx.with_global_type(type_id, |ty, type_ctx| {
            // direct nominal references
            if let dir::Type::Reference(reference) = ty {
                let symbol = reference.symbol;
                let canonical_symbol = ctx.canonical_symbol(symbol);
                if seen_symbols.insert(canonical_symbol) {
                    symbols.push(canonical_symbol);
                }

                let types = type_ctx.dir().types();
                if let Some(target_type_id) = types.get_symbol_type_id(symbol) {
                    ctx.collect_expected_type_symbols_inner(
                        target_type_id,
                        seen_types,
                        seen_symbols,
                        symbols,
                    );
                }

                return;
            }

            // nominal combinations
            match ty {
                dir::Type::Union(union) => {
                    for &element_id in &union.elements {
                        ctx.collect_expected_type_symbols_inner(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Intersection(intersection) => {
                    for &element_id in &intersection.elements {
                        ctx.collect_expected_type_symbols_inner(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Form(value) => {
                    ctx.collect_expected_type_symbols_inner(
                        value.value,
                        seen_types,
                        seen_symbols,
                        symbols,
                    );
                }
                _ => {}
            }
        });
    }
}
