use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::format::format_call_signature;
use crate::{ModuleQueryContext, ParameterList, Position};

/// A parameter in a signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureParameter {
    /// The parameter label.
    pub label: String,
    /// Documentation for this parameter.
    pub documentation: Option<String>,
}

impl SignatureParameter {
    /// Create a signature parameter.
    pub fn new(label: impl Into<String>, documentation: Option<String>) -> Self {
        Self {
            label: label.into(),
            documentation,
        }
    }

    /// Build signature parameters from formatted labels.
    fn list(labels: &[String], data: Option<&ParameterList>) -> Vec<Self> {
        labels
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let documentation = data
                    .and_then(|data| data.names.get(index).and_then(|name| data.docs.get(name)))
                    .cloned();

                Self::new(label.clone(), documentation)
            })
            .collect()
    }
}

/// A single callable signature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    pub fn new(
        label: impl Into<String>,
        documentation: Option<String>,
        parameters: Vec<SignatureParameter>,
    ) -> Self {
        Self {
            label: label.into(),
            documentation,
            parameters,
        }
    }
}

/// Signature help result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelp {
    /// Available signatures.
    pub signatures: Vec<SignatureItem>,
    /// The active signature.
    pub active_signature: usize,
    /// The active parameter.
    pub active_parameter: usize,
}

impl SignatureHelp {
    /// Create signature help with a single signature.
    pub fn single(signature: SignatureItem, active_parameter: usize) -> Self {
        Self {
            signatures: vec![signature],
            active_signature: 0,
            active_parameter,
        }
    }
}

/// Request signature help at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for signature help queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpResponse {
    /// Signature help data, if available.
    pub help: Option<SignatureHelp>,
}

impl ModuleQueryContext<'_> {
    /// Return signature help at the given position.
    pub fn signature_help(&self, offset: u32) -> Option<SignatureHelp> {
        let view = self.view();
        let source_file = self.source_file();
        let source = source_file.text();
        let enclosing =
            self.tree()
                .source_index
                .get_enclosing_spans(self.file_id(), offset, offset);

        // scan enclosing calls at the cursor
        for enclosing_span in &enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing_span.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression_id = node_id.try_into().unwrap_or_else(|_| {
                panic!("signature enclosing node is not an expression: {node_id:?}")
            });
            let dir::Expression::Call {
                left, arguments, ..
            } = view.get::<dir::Expression>(expression_id)
            else {
                continue;
            };

            let call_target = self.signature_target(*left);
            let Some(function_name) = call_target.name else {
                continue;
            };
            let Some(symbol_id) = call_target.symbol_id else {
                continue;
            };
            let Some(signature) = self.symbol_signature(symbol_id, &function_name) else {
                continue;
            };

            let active_parameter =
                self.active_signature_parameter(view, expression_id, arguments, offset, source);
            let active_parameter = if signature.parameters.is_empty() {
                0
            } else {
                active_parameter.min(signature.parameters.len().saturating_sub(1))
            };

            return Some(SignatureHelp::single(signature, active_parameter));
        }

        None
    }

    /// Format a signature item from a resolved symbol.
    fn symbol_signature(
        &self,
        symbol_id: dir::GlobalSymbolId,
        function_name: &str,
    ) -> Option<SignatureItem> {
        let symbol_module = self.module_context(symbol_id.module_id);
        let source_file = symbol_module.source_file();
        let source = source_file.text();
        let view = symbol_module.view();

        // read the symbol declaration from checked symbol data
        let declaration = {
            let symbols = symbol_module.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        // resolve the callable signature and documentation
        let (signature, documentation) = match declaration.local_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = declaration.local_id.try_into_typed().unwrap_or_else(|_| {
                    panic!(
                        "signature target is not a declaration: {:?}",
                        declaration.local_id
                    )
                });
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };

                let source_node_id = view.get_source(declaration_id);
                let documentation = symbol_module.node_doc_text_without_tags(
                    source,
                    source_node_id,
                    &["@param", "@return", "@returns"],
                );

                (&declaration.signature, documentation)
            }
            dir::NodeType::Member => {
                let member_id = declaration.local_id.try_into_typed().unwrap_or_else(|_| {
                    panic!(
                        "signature target is not a member: {:?}",
                        declaration.local_id
                    )
                });
                let member = view.get::<dir::Member>(member_id);
                let signature = member.signature()?;
                let source_node_id = view.get_source(member_id);
                let documentation = symbol_module.node_doc_text_without_tags(
                    source,
                    source_node_id,
                    &["@param", "@return", "@returns"],
                );

                (signature, documentation)
            }
            _ => return None,
        };

        // format the signature label and parameter labels
        let formatted = format_call_signature(function_name, signature, &symbol_module, false);
        let parameter_data = symbol_module.symbol_parameters(symbol_id);
        let parameters =
            SignatureParameter::list(formatted.parameters.as_slice(), parameter_data.as_ref());

        Some(SignatureItem::new(
            formatted.label,
            documentation,
            parameters,
        ))
    }

    /// Return the active parameter at a cursor position.
    fn active_signature_parameter(
        &self,
        view: dir::View<'_>,
        call_expression_id: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        cursor_offset: u32,
        source: &str,
    ) -> usize {
        if arguments.is_empty() {
            return 0;
        }

        let mut active_parameter = 0;
        let mut last_argument_end = None;

        // find the argument containing or preceding the cursor
        for (index, argument_id) in arguments.iter().enumerate() {
            let argument_node_id: dir::LocalNodeIdAny = (*argument_id).into();
            let span = self.get_span(view, argument_node_id);
            last_argument_end = Some(span.end);

            if cursor_offset < span.start {
                break;
            }

            active_parameter = index;
            if cursor_offset <= span.end {
                break;
            }
        }

        // allow an extra parameter after a trailing comma
        if let Some(last_argument_end) = last_argument_end {
            let call_span = self.get_span(view, call_expression_id.into());
            if self.cursor_is_after_trailing_argument_comma(
                source,
                call_span,
                last_argument_end,
                cursor_offset,
            ) {
                return arguments.len();
            }
        }

        active_parameter
    }

    /// Return whether the cursor follows a trailing comma in a call expression.
    fn cursor_is_after_trailing_argument_comma(
        &self,
        source: &str,
        call_span: Span,
        last_argument_end: u32,
        cursor_offset: u32,
    ) -> bool {
        // require the cursor to sit after the final argument and inside the call
        if cursor_offset <= last_argument_end || cursor_offset > call_span.end {
            return false;
        }

        // inspect the source slice between the argument and cursor
        let slice_start = last_argument_end.min(call_span.end) as usize;
        let slice_end = cursor_offset.min(call_span.end) as usize;
        let slice = source.get(slice_start..slice_end).unwrap_or_else(|| {
            panic!("signature source slice is out of bounds: {slice_start}..{slice_end}")
        });

        slice.contains(',')
    }
}
