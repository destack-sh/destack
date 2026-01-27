use destack_base::StringId;
use destack_dir::{Argument, Expression, GlobalSymbolId, NodeType};
use destack_source::{FileId, Uri};
use serde::{Deserialize, Serialize};

use crate::query::common::{
    ParameterData, nominal_symbol_for_expression, parameter_data_for_symbol,
    resolve_extension_member_symbol, resolve_member_access_symbol, with_query_context_for_file,
};
use crate::{ModuleAst, Session};

/// A parameter in a signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request signature help at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureHelpRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
}

/// Response payload for signature help queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureHelpResponse {
    /// Signature help data, if available.
    pub help: Option<SignatureHelp>,
}

/// Get signature help at the given position (inside a function call).
pub fn signature_help(session: &Session, file: FileId, offset: u32) -> Option<SignatureHelp> {
    with_query_context_for_file(session, file, |ctx| {
        let dir_tree = ctx.tree();

        // find enclosing AST nodes at the offset
        let enclosing = ctx.ast.tree.source_map.get_enclosing_spans(offset, offset);
        if enclosing.is_empty() {
            return None;
        }

        // look for a call expression among the enclosing nodes
        for enclosing_span in &enclosing {
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
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

                // try to get the function name, target symbol, and member parameters
                let (function_name, target_symbol, member_parameters) = match left_expr {
                    // direct function call: add(...)
                    Expression::GlobalReference { target_symbol, .. }
                    | Expression::LocalReference { target_symbol, .. }
                    | Expression::ModuleReference { target_symbol, .. } => {
                        // copy the target symbol so all match arms use the same type
                        let target_symbol = *target_symbol;

                        // resolve the symbol name from the target module
                        let target_module = session.modules.get(target_symbol.module_id);
                        let target = target_module.read();
                        let name = if let Some(target_ctx) = session.query_context(&target) {
                            let symbols = target_ctx.symbols();
                            let symbol_data = symbols.get_symbol(target_symbol.local_id);
                            symbol_data
                                .name()
                                .map(|id| target_ctx.ast.strings.get(id).to_string())
                        } else {
                            None
                        };
                        (name, Some(target_symbol), None)
                    }
                    // method call: obj.method(...)
                    Expression::Member {
                        left: receiver,
                        name,
                        ..
                    } => {
                        // resolve the target symbol using the member expression and receiver
                        let member_expression_id = *left;
                        let target_symbol = resolve_member_access_symbol(
                            session,
                            &ctx,
                            member_expression_id,
                            *receiver,
                            *name,
                        );

                        // resolve member parameters from extensions when symbol resolution fails
                        let member_parameters = if target_symbol.is_none() {
                            resolve_extension_member_parameters(session, &ctx, *receiver, *name)
                        } else {
                            None
                        };

                        let name = ctx.ast.strings.get(*name).to_string();
                        (Some(name), target_symbol, member_parameters)
                    }
                    _ => (None, None, None),
                };

                let function_name = function_name.unwrap_or_else(|| "<function>".to_string());

                // try to get actual parameter names from the function declaration
                let params = member_parameters.unwrap_or_else(|| {
                    get_function_parameters(session, target_symbol, dynamic_arguments.len())
                });

                let param_labels: Vec<_> = params.iter().map(|p| p.label.clone()).collect();
                let signature_label = format!("{}({})", function_name, param_labels.join(", "));

                let mut signature = SignatureInfo::new(signature_label);
                for param in params {
                    signature = signature.with_parameter(param);
                }

                // determine active parameter based on cursor position
                let active_parameter = determine_active_parameter(
                    ctx.ast,
                    ctx.file_id,
                    &dir_tree,
                    dynamic_arguments,
                    offset,
                );

                return Some(SignatureHelp::single(signature, active_parameter));
            }
        }

        None
    })?
}

/// Get parameter information for a function or method symbol.
fn get_function_parameters(
    session: &Session,
    target_symbol: Option<GlobalSymbolId>,
    argument_count: usize,
) -> Vec<ParameterInfo> {
    // bail out early when there is no target symbol
    let Some(symbol_id) = target_symbol else {
        return fallback_params(argument_count);
    };

    // collect parameter data from the shared parameter helpers
    let Some(data) = parameter_data_for_symbol(session, symbol_id) else {
        return fallback_params(argument_count);
    };

    // build parameter infos with optional documentation
    let params = parameter_infos_from_data(&data);

    // fall back when no parameters were collected
    if params.is_empty() {
        fallback_params(argument_count)
    } else {
        params
    }
}

/// Build parameter infos from shared parameter data.
fn parameter_infos_from_data(data: &ParameterData) -> Vec<ParameterInfo> {
    // map parameter names to infos and attach docs by name
    data.names
        .iter()
        .map(|name| {
            let mut info = ParameterInfo::new(name.clone());
            if let Some(doc) = data.docs.get(name) {
                info = info.with_documentation(doc.clone());
            }
            info
        })
        .collect()
}

/// Resolve member parameters from extensions when symbol resolution is unavailable.
fn resolve_extension_member_parameters(
    session: &Session,
    ctx: &crate::query::common::QueryContext<'_>,
    receiver: destack_dir::LocalNodeId<Expression>,
    member_name_id: StringId,
) -> Option<Vec<ParameterInfo>> {
    // resolve the base nominal symbol for the receiver expression
    let base_symbol = nominal_symbol_for_expression(session, ctx, receiver)?;

    // resolve the member name once for comparisons
    let member_name = session.strings.get(member_name_id).to_string();

    // resolve the extension member symbol through the shared member resolver
    let member_symbol =
        resolve_extension_member_symbol(session, base_symbol, ctx.module_id, &member_name)?;

    // collect parameter data from the resolved member symbol
    let data = parameter_data_for_symbol(session, member_symbol)?;
    let params = parameter_infos_from_data(&data);

    // return none when the extension member has no parameters
    if params.is_empty() {
        None
    } else {
        Some(params)
    }
}

fn fallback_params(argument_count: usize) -> Vec<ParameterInfo> {
    let count = argument_count.max(1);
    (0..count)
        .map(|i| ParameterInfo::new(format!("arg{i}")))
        .collect()
}

/// Determine which parameter is active based on cursor position.
///
/// Counts how many arguments come before the cursor position.
fn determine_active_parameter(
    ast: &ModuleAst,
    file_id: FileId,
    dir_tree: &destack_dir::NodeTree,
    arguments: &[destack_dir::LocalNodeId<Argument>],
    cursor_offset: u32,
) -> usize {
    // if no arguments, we're on parameter 0
    if arguments.is_empty() {
        return 0;
    }

    // find which argument the cursor is in or after
    let mut active_param = 0;

    for (idx, arg_id) in arguments.iter().enumerate() {
        // get the argument's source span
        let arg_node_id: destack_dir::LocalNodeIdAny = (*arg_id).into();
        let span = get_argument_span(ast, file_id, dir_tree, arg_node_id);
        // if cursor is before this argument's start, we're on the previous parameter
        if cursor_offset < span.start {
            break;
        }
        // cursor is in or after this argument
        active_param = idx;

        // if cursor is within this argument, stop here
        if cursor_offset <= span.end {
            break;
        }
    }

    active_param
}

/// Get the span of an argument node.
fn get_argument_span(
    ast: &ModuleAst,
    file_id: FileId,
    dir_tree: &destack_dir::NodeTree,
    node_id: destack_dir::LocalNodeIdAny,
) -> destack_source::Span {
    let source_id = dir_tree.get_source(node_id.id);
    let ast_span = ast.tree.source_map.get(source_id);
    destack_source::Span::new(file_id, ast_span.start, ast_span.end)
}
