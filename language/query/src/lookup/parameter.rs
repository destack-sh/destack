use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_dir as dir;

use crate::ModuleQueryContext;
use crate::source::parse_parameter_docs;

/// One declaration or member with callable parameters.
#[derive(Debug, Clone, Copy)]
struct ParameterTarget<'dir> {
    /// The source node that owns the callable parameter docs.
    source_node_id: u32,
    /// The callable parameters in declared order.
    parameters: &'dir [dir::LocalNodeId<dir::Parameter>],
}

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
    /// The direct nominal symbol when available.
    pub nominal_symbol: Option<dir::GlobalSymbolId>,
    /// Related nominal symbols reachable through the declared type.
    pub related_nominals: Vec<dir::GlobalSymbolId>,
    /// Whether callable values are preferred.
    pub prefers_callable: bool,
    /// Whether constructible values are preferred.
    pub prefers_constructible: bool,
}

/// Query display methods for DIR parameters.
trait ParameterQuery {
    /// Format the parameter into a display name.
    fn display_name(&self, strings: &StringPool) -> String;
}

impl ParameterQuery for dir::Parameter {
    fn display_name(&self, strings: &StringPool) -> String {
        // choose the display name based on the parameter shape
        match self {
            dir::Parameter::Named { name, .. } => strings.get(*name).to_string(),
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = strings.get(*name).to_string();
                format!("...{name}")
            }
            dir::Parameter::Pattern { .. } => "_".to_string(),
            dir::Parameter::VariadicPattern { .. } => "..._".to_string(),
            dir::Parameter::Error => panic!("error parameter reached query formatting"),
        }
    }
}

impl ParameterTarget<'_> {
    /// Resolve the parameter node that should guide one argument index.
    fn parameter_id(
        &self,
        view: dir::View<'_>,
        parameter_index: usize,
    ) -> Option<dir::LocalNodeId<dir::Parameter>> {
        if let Some(parameter_id) = self.parameters.get(parameter_index) {
            return Some(*parameter_id);
        }

        let last_parameter_id = *self.parameters.last()?;
        let last_parameter = view.get::<dir::Parameter>(last_parameter_id);
        match last_parameter {
            dir::Parameter::VariadicNamed { .. } | dir::Parameter::VariadicPattern { .. } => {
                Some(last_parameter_id)
            }
            _ => None,
        }
    }
}

impl ParameterList {
    /// Collect parameter names and docs from one target.
    fn from_target(module: &ModuleQueryContext<'_>, target: ParameterTarget<'_>) -> Self {
        // resolve source text for doc parsing
        let source_file = module.source_file();
        let source = source_file.text();

        // collect docs and names from the target
        let docs = module.parameter_doc_map(source, target.source_node_id);
        let names = Self::names(module.strings(), module.view(), target.parameters);

        Self { names, docs }
    }

    /// Collect parameter display names from parameter nodes.
    fn names(
        strings: &StringPool,
        view: dir::View<'_>,
        parameters: &[dir::LocalNodeId<dir::Parameter>],
    ) -> Vec<String> {
        // collect parameter display names in declared order
        parameters
            .iter()
            .map(|parameter_id| {
                let parameter = view.get::<dir::Parameter>(*parameter_id);
                parameter.display_name(strings)
            })
            .collect()
    }
}

impl ModuleQueryContext<'_> {
    /// Return parameter names for a function symbol.
    pub(crate) fn symbol_parameter_names(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<Vec<String>> {
        // collect parameter list for the symbol declaration
        let data = self.symbol_parameters(symbol_id)?;

        if data.names.is_empty() {
            return None;
        }

        Some(data.names)
    }

    /// Collect parameter names and docs for a function or method symbol.
    pub(crate) fn symbol_parameters(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<ParameterList> {
        let symbol_module = self.module_context(symbol_id.module_id);
        let target = symbol_module.symbol_parameter_target(symbol_id)?;

        Some(ParameterList::from_target(&symbol_module, target))
    }

    /// Classify the expected value shape for one parameter type.
    fn expected_value_shape(&self, type_id: dir::GlobalTypeId) -> (bool, bool) {
        self.read_global_type(type_id, |ty, _| match ty {
            dir::Type::FunctionSignature(_)
            | dir::Type::Function(_)
            | dir::Type::FunctionPointer(_) => (true, false),
            dir::Type::Shape(object) => (
                !object.call_signatures.is_empty(),
                !object.construct_signatures.is_empty(),
            ),
            _ => (false, false),
        })
    }
}

impl ModuleQueryContext<'_> {
    /// Collect @param documentation from a declaration's doc comments.
    pub(crate) fn parameter_doc_map(
        &self,
        source: &str,
        source_node_id: u32,
    ) -> HashMap<String, String> {
        // initialize the parameter doc map
        let mut param_docs = HashMap::new();

        // gather docs on the node or enclosing nodes
        let doc_strings = self.node_or_enclosing_doc_strings(source, source_node_id);

        // parse @param tags from collected docs
        for doc_text in doc_strings {
            param_docs.extend(parse_parameter_docs(&doc_text));
        }

        param_docs
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the callable parameter target for one symbol.
    fn symbol_parameter_target(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<ParameterTarget<'_>> {
        // read the symbol declaration
        let global_node_id = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        // resolve the parameter target based on the declaration node type
        let view = self.view();
        match global_node_id.local_id.ty {
            // collect parameters from function declarations
            dir::NodeType::Declaration => {
                let declaration_id =
                    global_node_id
                        .local_id
                        .try_into_typed()
                        .unwrap_or_else(|_| {
                            panic!(
                                "parameter source is not a declaration: {:?}",
                                global_node_id.local_id
                            )
                        });
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };

                let source_node_id = view.get_source(declaration_id);

                Some(ParameterTarget {
                    source_node_id,
                    parameters: &declaration.signature.parameters,
                })
            }

            // collect parameters from method members
            dir::NodeType::Member => {
                let member_id = global_node_id
                    .local_id
                    .try_into_typed()
                    .unwrap_or_else(|_| {
                        panic!(
                            "parameter source is not a member: {:?}",
                            global_node_id.local_id
                        )
                    });
                let member = view.get::<dir::Member>(member_id);
                let signature = member.signature()?;

                let source_node_id = view.get_source(member_id);

                Some(ParameterTarget {
                    source_node_id,
                    parameters: &signature.parameters,
                })
            }

            _ => None,
        }
    }

    /// Resolve the expected-parameter hint for one active argument.
    pub(crate) fn symbol_expected_parameter(
        &self,
        symbol_id: dir::GlobalSymbolId,
        parameter_index: usize,
    ) -> Option<ExpectedParameterHint> {
        // read the target module and parameter target
        let symbol_module = self.module_context(symbol_id.module_id);
        let target = symbol_module.symbol_parameter_target(symbol_id)?;

        // resolve the active parameter node
        let view = symbol_module.view();
        let parameter_id = target.parameter_id(view, parameter_index)?;
        let parameter = view.get::<dir::Parameter>(parameter_id);
        let name = Some(parameter.display_name(symbol_module.strings()));

        // resolve the declared parameter type and classify its shape
        let global_parameter_id = parameter_id.into_global_any(symbol_module.module_id());
        let type_id = symbol_module
            .types()
            .get_node_type_id(global_parameter_id)?;
        let nominal_symbol = symbol_module
            .read_global_type(type_id, |ty, _| ty.symbol())
            .map(|symbol_id| symbol_module.canonical_symbol(symbol_id));
        let related_nominals = symbol_module.collect_expected_nominals(type_id);
        let (prefers_callable, prefers_constructible) = symbol_module.expected_value_shape(type_id);

        Some(ExpectedParameterHint {
            name,
            nominal_symbol,
            related_nominals,
            prefers_callable,
            prefers_constructible,
        })
    }

    /// Collect nominal symbols that should contribute to expected-type ranking.
    fn collect_expected_nominals(&self, type_id: dir::GlobalTypeId) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();
        let mut seen_types = HashSet::new();
        let mut seen_symbols = HashSet::new();

        self.collect_expected_nominals_inner(
            type_id,
            &mut seen_types,
            &mut seen_symbols,
            &mut symbols,
        );

        symbols
    }

    /// Collect nominal symbols from one declared parameter type.
    fn collect_expected_nominals_inner(
        &self,
        type_id: dir::GlobalTypeId,
        seen_types: &mut HashSet<dir::GlobalTypeId>,
        seen_symbols: &mut HashSet<dir::GlobalSymbolId>,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) {
        let type_id = self.unwrap_global_form_payload_type_id(type_id);
        if !seen_types.insert(type_id) {
            return;
        }

        self.read_global_type(type_id, |ty, type_context| {
            // direct nominal references
            if let dir::Type::Application(reference) = ty {
                let symbol = reference.symbol;
                let canonical_symbol = self.canonical_symbol(symbol);
                if seen_symbols.insert(canonical_symbol) {
                    symbols.push(canonical_symbol);
                }

                let types = type_context.types();
                if let Some(target_type_id) = types.get_symbol_type_id(symbol) {
                    self.collect_expected_nominals_inner(
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
                    for &element_id in type_context.types().type_ids(union.elements) {
                        self.collect_expected_nominals_inner(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Intersection(intersection) => {
                    for &element_id in type_context.types().type_ids(intersection.elements) {
                        self.collect_expected_nominals_inner(
                            element_id,
                            seen_types,
                            seen_symbols,
                            symbols,
                        );
                    }
                }
                dir::Type::Form(value) => {
                    self.collect_expected_nominals_inner(
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
