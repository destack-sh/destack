use destack_dir::{Declaration, Expression, GlobalSymbolId, NodeType, Parameter};
use destack_source::FileId;

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// A parameter in a signature.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// The parameter label (e.g., "name: string").
    pub label: String,
    /// Documentation for this parameter.
    pub documentation: Option<String>,
}

impl ParameterInfo {
    /// Create a parameter info.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            documentation: None,
        }
    }

    /// Add documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }
}

/// A single signature (for overloaded functions, there may be multiple).
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// The full signature label.
    pub label: String,
    /// Documentation for the signature.
    pub documentation: Option<String>,
    /// Parameters in this signature.
    pub parameters: Vec<ParameterInfo>,
}

impl SignatureInfo {
    /// Create a signature info.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            documentation: None,
            parameters: Vec::new(),
        }
    }

    /// Add documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Add a parameter.
    pub fn with_parameter(mut self, param: ParameterInfo) -> Self {
        self.parameters.push(param);
        self
    }
}

/// Signature help result.
#[derive(Debug, Clone)]
pub struct SignatureHelp {
    /// Available signatures.
    pub signatures: Vec<SignatureInfo>,
    /// The active signature (index into signatures).
    pub active_signature: usize,
    /// The active parameter (index into parameters).
    pub active_parameter: usize,
}

impl SignatureHelp {
    /// Create signature help with a single signature.
    pub fn single(signature: SignatureInfo, active_parameter: usize) -> Self {
        Self {
            signatures: vec![signature],
            active_signature: 0,
            active_parameter,
        }
    }
}

/// Get signature help at the given position (inside a function call).
pub fn signature_help(session: &Session, file: FileId, offset: u32) -> Option<SignatureHelp> {
    // 1. get the module for this file
    let module = get_module_by_file_id(session, file)?;
    let module_guard = module.read();

    // 2. find enclosing AST nodes at the offset
    let enclosing = module_guard
        .ast
        .tree
        .source_map
        .get_enclosing_spans(offset, offset);
    if enclosing.is_empty() {
        return None;
    }

    let dir_tree = module_guard.dir.tree.read();

    // 3. look for a call expression among the enclosing nodes
    for enc in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enc.idx) else {
            continue;
        };

        if dir_node_id.ty != NodeType::Expression {
            continue;
        }

        let expression_id = dir_node_id.try_into().ok()?;
        let expression = dir_tree.get::<Expression>(expression_id);

        // check if this is a call expression
        if let Expression::Call {
            left,
            dynamic_arguments,
            ..
        } = expression
        {
            // get the function being called
            let left_expr = dir_tree.get::<Expression>(*left);

            // try to get the function name and target symbol
            let (function_name, target_symbol) = match left_expr {
                // direct function call: add(...)
                Expression::GlobalReference { target_symbol, .. }
                | Expression::LocalReference { target_symbol, .. }
                | Expression::ModuleReference { target_symbol, .. } => {
                    let target_module = session.modules.get(target_symbol.module_id);
                    let target_guard = target_module.read();
                    let symbols = target_guard.dir.symbols.read();
                    let symbol_data = symbols.get_symbol(target_symbol.local_id);
                    let name = symbol_data
                        .name()
                        .map(|id| target_guard.ast.strings.get(id).to_string());
                    (name, Some(*target_symbol))
                }
                // method call: obj.method(...)
                Expression::Member { name, .. } => {
                    let name = module_guard.ast.strings.get(*name).to_string();
                    (Some(name), None) // TODO #Incomplete: resolve method to get actual signature
                }
                _ => (None, None),
            };

            let function_name = function_name.unwrap_or_else(|| "<function>".to_string());

            // try to get actual parameter names from the function declaration
            let params = get_function_parameters(session, target_symbol, dynamic_arguments.len());

            let param_labels: Vec<_> = params.iter().map(|p| p.label.clone()).collect();
            let signature_label = format!("{}({})", function_name, param_labels.join(", "));

            let mut signature = SignatureInfo::new(signature_label);
            for param in params {
                signature = signature.with_parameter(param);
            }

            // determine active parameter (basic: assume first for now)
            // A more complete implementation would count commas before the cursor
            let active_parameter = 0;

            return Some(SignatureHelp::single(signature, active_parameter));
        }
    }

    None
}

/// Get parameter information for a function.
///
/// If the target symbol points to a function declaration, extracts actual parameter names.
/// Falls back to generic arg0, arg1, etc. if not available.
fn get_function_parameters(
    session: &Session,
    target_symbol: Option<GlobalSymbolId>,
    argument_count: usize,
) -> Vec<ParameterInfo> {
    // try to get actual parameter names from the function declaration
    if let Some(symbol_id) = target_symbol {
        let target_module = session.modules.get(symbol_id.module_id);
        let target_guard = target_module.read();
        let symbols = target_guard.dir.symbols.read();
        let symbol_data = symbols.get_symbol(symbol_id.local_id);

        // check if this symbol has a primary declaration
        if let Some(global_node_id) = symbol_data.primary_declaration {
            // must be a declaration node
            if global_node_id.local_id.ty == NodeType::Declaration {
                let declaration_id = global_node_id.local_id.try_into_typed().ok();
                if let Some(declaration_id) = declaration_id {
                    let dir_tree = target_guard.dir.tree.read();
                    let declaration = dir_tree.get::<Declaration>(declaration_id);

                    // if it's a function, get its parameters
                    if let Declaration::Function { signature, .. } = declaration {
                        let params: Vec<_> = signature
                            .dynamic_parameters
                            .iter()
                            .map(|param_id| {
                                let param = dir_tree.get::<Parameter>(*param_id);
                                let param_name = match param {
                                    Parameter::Named { name, .. } => {
                                        target_guard.ast.strings.get(*name).to_string()
                                    }
                                    Parameter::Variadic { name, .. } => {
                                        let name_str =
                                            target_guard.ast.strings.get(*name).to_string();
                                        format!("...{name_str}")
                                    }
                                    Parameter::Pattern { .. } => "<pattern>".to_string(),
                                };
                                ParameterInfo::new(param_name)
                            })
                            .collect();

                        if !params.is_empty() {
                            return params;
                        }
                    }
                }
            }
        }
    }

    // fallback to generic parameter names
    let count = argument_count.max(1);
    (0..count)
        .map(|i| ParameterInfo::new(format!("arg{i}")))
        .collect()
}
