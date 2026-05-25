use crate::core::QueryPosition;
use destack_dir as dir;
use destack_dir::{Argument, Declaration, Expression, GlobalSymbolId, Member, NodeType};
use serde::{Deserialize, Serialize};

use crate::core::ModuleQueryContext;
use crate::dir::{
    ParameterList, call_target, doc_text_for_node_without_tags, parameter_list_for_symbol,
};
use crate::format::format_call_signature;
use crate::source::span_for_dir_node;

/// A parameter in a signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureParameter {
    /// The parameter label (e.g., "name: string").
    pub label: String,
    /// Documentation for this parameter.
    pub documentation: Option<String>,
}

impl SignatureParameter {
    /// Create a signature parameter.
    pub fn new(label: impl Into<String>) -> Self {
        // build a signature parameter with defaults
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
pub struct SignatureItem {
    /// The full signature label.
    pub label: String,
    /// Documentation for the signature.
    pub documentation: Option<String>,
    /// Parameters in this signature.
    pub parameters: Vec<SignatureParameter>,
}

impl SignatureItem {
    /// Create a signature item.
    pub fn new(label: impl Into<String>) -> Self {
        // build a signature item with defaults
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
    pub fn with_parameter(mut self, param: SignatureParameter) -> Self {
        self.parameters.push(param);
        self
    }
}

/// Signature help result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureHelp {
    /// Available signatures.
    pub signatures: Vec<SignatureItem>,
    /// The active signature (index into signatures).
    pub active_signature: usize,
    /// The active parameter (index into parameters).
    pub active_parameter: usize,
}

impl SignatureHelp {
    /// Create signature help with a single signature.
    pub fn single(signature: SignatureItem, active_parameter: usize) -> Self {
        // build a single signature response
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
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for signature help queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureHelpResponse {
    /// Signature help data, if available.
    pub help: Option<SignatureHelp>,
}

/// Get signature help at the given position (inside a function call).
pub fn signature_help(ctx: &ModuleQueryContext<'_>, offset: u32) -> Option<SignatureHelp> {
    let dir_tree = ctx.dir().view();
    let source_file = ctx
        .repository()
        .file(ctx.revision(), ctx.file_id())
        .ok()
        .flatten()?;
    let source = source_file.text();
    let enclosing =
        ctx.dir()
            .tree()
            .source_index
            .get_enclosing_spans(ctx.file_id(), offset, offset);

    // scan enclosing calls at the cursor
    for enclosing_span in &enclosing {
        let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
            continue;
        };
        if dir_node_id.ty != NodeType::Expression {
            continue;
        }

        let expression_id = dir_node_id.try_into().ok()?;
        let Expression::Call {
            left, arguments, ..
        } = dir_tree.get::<Expression>(expression_id)
        else {
            continue;
        };

        let call_target = call_target(ctx.dir(), *left);
        let function_name = call_target.name.unwrap_or_else(|| "<function>".to_string());
        let Some(symbol_id) = call_target.symbol else {
            continue;
        };
        let Some(signature) = signature_info_for_symbol(ctx, symbol_id, &function_name) else {
            continue;
        };

        let mut active_parameter =
            determine_active_parameter(ctx, dir_tree, expression_id, arguments, offset, source);
        if signature.parameters.is_empty() {
            active_parameter = 0;
        } else if active_parameter >= signature.parameters.len() {
            active_parameter = signature.parameters.len().saturating_sub(1);
        }

        return Some(SignatureHelp::single(signature, active_parameter));
    }

    None
}

/// Format signature item from a resolved symbol.
fn signature_info_for_symbol(
    root_ctx: &ModuleQueryContext<'_>,
    symbol_id: GlobalSymbolId,
    function_name: &str,
) -> Option<SignatureItem> {
    // read the symbol's module and query context
    let ctx = root_ctx.module_context(symbol_id.module_id)?;

    // read the symbol declaration
    let declaration_ref = {
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

    // resolve the function signature from the declaration or member
    let dir_tree = ctx.dir().view();
    let (signature, doc_text) = match declaration_ref.local_id.ty {
        // handle function declarations
        NodeType::Declaration => {
            let declaration_id = declaration_ref.local_id.try_into_typed().ok()?;
            let declaration = dir_tree.get::<Declaration>(declaration_id);
            let Declaration::Function(declaration) = declaration else {
                return None;
            };
            let source_node_id = dir_tree.get_source(declaration_id);
            let doc_text = doc_text_for_node_without_tags(
                ctx.dir(),
                source,
                source_node_id,
                &["@param", "@return", "@returns"],
            );
            (&declaration.signature, doc_text)
        }
        // handle method members
        NodeType::Member => {
            let member_id = declaration_ref.local_id.try_into_typed().ok()?;
            let member = dir_tree.get::<Member>(member_id);
            let signature = member.signature()?;
            let source_node_id = dir_tree.get_source(member_id);
            let doc_text = doc_text_for_node_without_tags(
                ctx.dir(),
                source,
                source_node_id,
                &["@param", "@return", "@returns"],
            );
            (signature, doc_text)
        }
        _ => return None,
    };

    // format the signature label for display
    let types = ctx.dir().types();
    let formatted = format_call_signature(
        function_name,
        signature,
        ctx.module_id(),
        dir_tree,
        types,
        &ctx,
        false,
    );

    // resolve parameter documentation when available
    let params = if let Some(data) = parameter_list_for_symbol(&ctx, symbol_id) {
        signature_parameters_from_list(formatted.parameters.as_slice(), &data)
    } else {
        formatted
            .parameters
            .iter()
            .map(|label| SignatureParameter::new(label.clone()))
            .collect()
    };

    // assemble the signature item
    let mut signature = SignatureItem::new(formatted.label);
    if let Some(doc_text) = doc_text {
        signature = signature.with_documentation(doc_text);
    }
    for param in params {
        signature = signature.with_parameter(param);
    }

    // return the formatted signature
    Some(signature)
}

/// Build signature parameters from shared parameter list.
fn signature_parameters_from_list(
    labels: &[String],
    data: &ParameterList,
) -> Vec<SignatureParameter> {
    // map parameter labels to infos and attach docs by position
    labels
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            // build the signature parameter
            let mut info = SignatureParameter::new(label.clone());
            let name = data.names.get(idx);
            if let Some(name) = name {
                let doc = data.docs.get(name);
                if let Some(doc) = doc {
                    info = info.with_documentation(doc.clone());
                }
            }
            info
        })
        .collect()
}

/// Determine which parameter is active based on cursor position.
///
/// Counts how many arguments come before the cursor position.
fn determine_active_parameter(
    ctx: &ModuleQueryContext<'_>,
    dir_tree: dir::View<'_>,
    call_expression_id: dir::LocalNodeId<Expression>,
    arguments: &[dir::LocalNodeId<Argument>],
    cursor_offset: u32,
    source: &str,
) -> usize {
    // if no arguments, we're on parameter 0
    if arguments.is_empty() {
        return 0;
    }

    // find which argument the cursor is in or after
    let mut active_param = 0;
    let mut lsource_span_end = None;

    for (idx, arg_id) in arguments.iter().enumerate() {
        // get the argument's source span
        let arg_node_id: dir::LocalNodeIdAny = (*arg_id).into();
        let span = span_for_dir_node(ctx.dir(), dir_tree, arg_node_id);
        lsource_span_end = Some(span.end);

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

    // allow an extra parameter when cursor sits after a trailing comma
    if let Some(lsource_span_end) = lsource_span_end {
        // detect trailing comma usage for the call expression
        let call_span = span_for_dir_node(ctx.dir(), dir_tree, call_expression_id.into());
        if cursor_offset > lsource_span_end && cursor_offset <= call_span.end {
            let slice_start = lsource_span_end.min(call_span.end) as usize;
            let slice_end = cursor_offset.min(call_span.end) as usize;
            let slice = source.get(slice_start..slice_end).unwrap_or("");
            if slice.contains(',') {
                return arguments.len();
            }
        }
    }

    // return the determined parameter index
    active_param
}
