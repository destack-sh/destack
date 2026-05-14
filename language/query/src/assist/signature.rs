use destack_dir as dir;
use destack_dir::{Argument, Declaration, Expression, GlobalSymbolId, Member, NodeType};
use destack_source::{FileId, Uri};
use destack_workspace::{Repository, Revision};
use serde::{Deserialize, Serialize};

use crate::core::{QueryContext, query_context, with_query_context_for_file};
use crate::dir::{
    ParameterData, call_target, doc_text_for_node_without_tags, parameter_data_for_symbol,
};
use crate::format::format_call_signature;
use crate::source::span_for_dir_node;

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
        // build a parameter info with defaults
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
        // build a signature info with defaults
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
pub fn signature_help(
    repository: &Repository,
    revision: Revision,
    file: FileId,
    offset: u32,
) -> Option<SignatureHelp> {
    // resolve signature help within the query context
    with_query_context_for_file(repository, revision, file, |ctx| {
        // resolve the dir tree for traversal
        let dir_tree = ctx.dir().view();

        // resolve the source text for cursor checks
        let source_file = repository.file(revision, ctx.file_id()).ok().flatten()?;
        let source = source_file.text();

        // find enclosing source nodes at the offset
        let enclosing = ctx
            .source()
            .tree()
            .source_map
            .get_enclosing_spans(offset, offset);

        // bail out when no enclosing spans exist
        if enclosing.is_empty() {
            return None;
        }

        // look for a call expression among the enclosing nodes
        for enclosing_span in &enclosing {
            // resolve the dir node for the enclosing source span
            let Some(dir_node_id) = dir_tree.get_node_id_by_source_id(enclosing_span.idx) else {
                continue;
            };

            // skip non expression nodes
            if dir_node_id.ty != NodeType::Expression {
                continue;
            }

            // read the expression node
            let expression_id = dir_node_id.try_into().ok()?;
            let expression = dir_tree.get::<Expression>(expression_id);

            // check if this is a call expression
            if let Expression::Call {
                left, arguments, ..
            } = expression
            {
                // read the call target symbol and name
                let call_target = call_target(repository, ctx.dir(), *left);
                let function_name = call_target.name.unwrap_or_else(|| "<function>".to_string());
                let Some(symbol_id) = call_target.symbol else {
                    continue;
                };

                // build signature info from the target symbol
                let Some(signature) =
                    build_signature_info(repository, revision, &function_name, symbol_id)
                else {
                    continue;
                };

                // determine active parameter based on cursor position
                let mut active_parameter = determine_active_parameter(
                    &ctx,
                    dir_tree,
                    expression_id,
                    arguments,
                    offset,
                    source,
                );

                // default to zero when no parameters are available
                if signature.parameters.is_empty() {
                    active_parameter = 0;
                }
                // clamp to the last parameter when the cursor is past the end
                else if active_parameter >= signature.parameters.len() {
                    active_parameter = signature.parameters.len().saturating_sub(1);
                }

                // return the resolved signature help
                return Some(SignatureHelp::single(signature, active_parameter));
            }
        }

        // return none when no call expression is found
        None
    })?
}

/// Get parameter information for a function or method symbol.
fn build_signature_info(
    repository: &Repository,
    revision: Revision,
    function_name: &str,
    target_symbol: GlobalSymbolId,
) -> Option<SignatureInfo> {
    // format the symbol's signature when available
    signature_info_for_symbol(repository, revision, target_symbol, function_name)
}

/// Format signature info from a resolved symbol.
fn signature_info_for_symbol(
    repository: &Repository,
    revision: Revision,
    symbol_id: GlobalSymbolId,
    function_name: &str,
) -> Option<SignatureInfo> {
    // read the symbol's module and query context
    let ctx = query_context(repository, revision, symbol_id.module_id)?;

    // read the symbol declaration
    let declaration_ref = {
        let symbols = ctx.dir().symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        symbol.declaration?
    };

    // resolve the source text for doc parsing
    let source_file = repository.file(revision, ctx.file_id()).ok().flatten()?;
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
                ctx.source(),
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
                ctx.source(),
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
        repository,
        revision,
        ctx.dir().strings(),
        false,
    );

    // resolve parameter documentation when available
    let params = if let Some(data) = parameter_data_for_symbol(repository, revision, symbol_id) {
        parameter_infos_from_data(formatted.parameters.as_slice(), &data)
    } else {
        formatted
            .parameters
            .iter()
            .map(|label| ParameterInfo::new(label.clone()))
            .collect()
    };

    // assemble the signature info
    let mut signature = SignatureInfo::new(formatted.label);
    if let Some(doc_text) = doc_text {
        signature = signature.with_documentation(doc_text);
    }
    for param in params {
        signature = signature.with_parameter(param);
    }

    // return the formatted signature
    Some(signature)
}

/// Build parameter infos from shared parameter data.
fn parameter_infos_from_data(labels: &[String], data: &ParameterData) -> Vec<ParameterInfo> {
    // map parameter labels to infos and attach docs by position
    labels
        .iter()
        .enumerate()
        .map(|(idx, label)| {
            // build the parameter info
            let mut info = ParameterInfo::new(label.clone());
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
    ctx: &QueryContext,
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
        let span = span_for_dir_node(ctx.source(), dir_tree, arg_node_id);
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
        let call_span = span_for_dir_node(ctx.source(), dir_tree, call_expression_id.into());
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
