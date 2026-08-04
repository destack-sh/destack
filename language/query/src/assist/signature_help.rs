use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    AppliedSignature, Formatter, ModuleQueryContext, ProgramQueryContext, QueryError,
    QueryPosition, QueryResult,
};

/// One parameter shown by signature help.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureParameter {
    /// The formatted parameter label.
    pub label: String,
    /// The parameter documentation when available.
    pub documentation: Option<String>,
}

/// One callable signature shown by signature help.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureItem {
    /// The full formatted signature.
    pub label: String,
    /// The declaration documentation when available.
    pub documentation: Option<String>,
    /// The formatted parameters in declaration order.
    pub parameters: Vec<SignatureParameter>,
}

/// Signature help for one selected call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelp {
    /// The statically selected callable signatures.
    pub signatures: Vec<SignatureItem>,
    /// The active signature ordinal.
    pub active_signature: usize,
    /// The active parameter ordinal when recorded by checking.
    pub active_parameter: Option<usize>,
}

/// Request signature help at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// Response payload for signature help queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpResponse {
    /// Signature help, if available.
    pub help: Option<SignatureHelp>,
}

impl ModuleQueryContext<'_> {
    /// Return signature help for the innermost selected call at one offset.
    pub fn signature_help(
        &self,
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<SignatureHelp>> {
        let view = self.view();
        let enclosing = self.enclosing_spans_at_cursor(file_id, offset);

        // select the innermost call with one recorded resolution
        for enclosing in enclosing {
            let Some(node_id) = view.get_node_id_by_source_id(enclosing.source_id) else {
                continue;
            };
            if node_id.ty != dir::NodeType::Expression {
                continue;
            }

            let expression_id = dir::LocalNodeId::<dir::Expression>::new(node_id.id);
            let global_id = expression_id.into_global_any(self.module_id());
            let (signatures, arguments, bindings) = match view.get(expression_id) {
                dir::Expression::Call {
                    left, arguments, ..
                } => {
                    let call = self.resolutions().call_resolution(global_id);
                    let construct = self.resolutions().construct_resolution(global_id);
                    if call.is_some() && construct.is_some() {
                        return Err(QueryError::conflict(format!(
                            "signature resolution columns: {global_id:?}"
                        )));
                    }

                    if let Some(resolution) = construct {
                        let signatures = self.construct_signature_items(query, resolution)?;

                        (
                            signatures,
                            arguments.as_slice(),
                            Some(resolution.arguments.as_slice()),
                        )
                    } else {
                        let Some(resolution) = call else {
                            return Ok(None);
                        };
                        let signatures = self.call_signature_items(query, *left, resolution)?;

                        (
                            signatures,
                            arguments.as_slice(),
                            resolution.shared_arguments(),
                        )
                    }
                }
                dir::Expression::New { arguments, .. } => {
                    let Some(resolution) = self.resolutions().construct_resolution(global_id)
                    else {
                        return Ok(None);
                    };
                    let signatures = self.construct_signature_items(query, resolution)?;

                    (
                        signatures,
                        arguments.as_slice(),
                        Some(resolution.arguments.as_slice()),
                    )
                }
                _ => continue,
            };

            let active_parameter = match bindings {
                Some(bindings) => self.active_signature_parameter(arguments, bindings, offset)?,
                None => None,
            };

            return Ok(Some(SignatureHelp {
                signatures,
                active_signature: 0,
                active_parameter,
            }));
        }

        Ok(None)
    }

    /// Format every statically selected symbol call target.
    fn call_signature_items(
        &self,
        query: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
    ) -> QueryResult<Vec<SignatureItem>> {
        let mut signatures = Vec::new();
        for call in resolution.iter() {
            let signature = match &call.target {
                dir::CallTarget::Symbol { function, .. } => self.call_signature_item(
                    query,
                    function.symbol,
                    &function.generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallTarget::Dynamic {
                    function: dir::DynamicFunction::Symbol(symbol),
                    generic_arguments,
                    ..
                } => self.call_signature_item(
                    query,
                    *symbol,
                    generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallTarget::Dynamic {
                    function,
                    generic_arguments,
                    ..
                } => self.signature_node_item(
                    query,
                    function,
                    generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallTarget::Expression { generic_arguments } => self
                    .expression_signature_item(
                        query,
                        callee_id,
                        generic_arguments,
                        &call.arguments,
                        call.return_type,
                    )?,
            };
            signatures.push(signature);
        }

        // retain one signature for repeated arm selections
        signatures.dedup();

        if signatures.is_empty() {
            return Err(QueryError::missing("selected call signatures"));
        }

        Ok(signatures)
    }

    /// Format one selected callable symbol from its recorded declaration.
    fn call_signature_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let Some(symbol_id) = query.canonical_symbol(symbol)? else {
            return Err(QueryError::missing(format!(
                "call signature declaration: {symbol:?}"
            )));
        };
        let module = query.module(symbol_id.module_id)?;
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(name) = symbol.name() else {
            return Err(QueryError::missing(format!(
                "call signature name: {symbol_id:?}"
            )));
        };
        let name = module.strings().get(name).to_string();
        let Some(parameters) = module.callable_parameters(symbol_id.local_id) else {
            return Err(QueryError::missing(format!(
                "call signature parameters: {symbol_id:?}"
            )));
        };

        self.selected_signature_item(
            query,
            &module,
            Some(symbol_id),
            &name,
            parameters,
            generic_arguments,
            bindings,
            return_type,
        )
    }

    /// Format one expression-backed callable selection.
    fn expression_signature_item(
        &self,
        query: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let source = callee_id.into_global_any(self.module_id());
        let Some(symbols) = self.recorded_symbol_targets(source)? else {
            return Err(QueryError::missing(format!(
                "expression call declaration: {source:?}"
            )));
        };
        let [symbol_id] = symbols.as_slice() else {
            return Err(QueryError::conflict(format!(
                "expression call declarations: {source:?}, {symbols:?}"
            )));
        };

        self.call_signature_item(query, *symbol_id, generic_arguments, bindings, return_type)
    }

    /// Format one signature-backed dynamic selection from its declaring node.
    fn signature_node_item(
        &self,
        query: &ProgramQueryContext<'_>,
        function: &dir::DynamicFunction,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        // name the slot after its declaring signature family
        let (node, name) = match function {
            dir::DynamicFunction::CallSignature(node) => (*node, "call"),
            dir::DynamicFunction::ConstructSignature(node) => (*node, "new"),
            dir::DynamicFunction::IndexRead(node) => (*node, "[]"),
            dir::DynamicFunction::IndexWrite(node) => (*node, "[]="),
            dir::DynamicFunction::Symbol(symbol) => {
                return Err(QueryError::invalid(format!(
                    "declaration-backed dynamic signature: {symbol:?}"
                )));
            }
        };
        let module = query.module(node.module_id)?;
        let Some(parameters) = module.node_callable_parameters(node.local_id) else {
            return Err(QueryError::missing(format!(
                "dynamic signature parameters: {node:?}"
            )));
        };
        let parameters = parameters.to_vec();

        self.selected_signature_item(
            query,
            &module,
            None,
            name,
            &parameters,
            generic_arguments,
            bindings,
            return_type,
        )
    }

    /// Format one selected declaration signature with applied call types.
    fn selected_signature_item(
        &self,
        query: &ProgramQueryContext<'_>,
        declaration_module: &ModuleQueryContext<'_>,
        symbol_id: Option<dir::GlobalSymbolId>,
        name: &str,
        declared_parameters: &[dir::LocalNodeId<dir::Parameter>],
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let mut names = Vec::with_capacity(declared_parameters.len());
        let mut documentation = Vec::with_capacity(declared_parameters.len());

        // retain authored parameter names and documentation
        for parameter_id in declared_parameters {
            let parameter = declaration_module.view().get(*parameter_id);
            let mut name = declaration_module.parameter_name(parameter)?;
            if parameter.is_optional() {
                name.push('?');
            }
            names.push(Some(name));

            let parameter_documentation = declaration_module
                .view()
                .get_documentation(*parameter_id)
                .map(|documentation| {
                    declaration_module
                        .strings()
                        .get(documentation.text)
                        .to_string()
                });
            documentation.push(parameter_documentation);
        }

        self.signature_item(
            query,
            symbol_id,
            name,
            generic_arguments,
            &names,
            &documentation,
            bindings,
            return_type,
        )
    }

    /// Format the exact constructor selected by checking.
    fn construct_signature_items(
        &self,
        query: &ProgramQueryContext<'_>,
        resolution: &dir::ConstructResolution,
    ) -> QueryResult<Vec<SignatureItem>> {
        let item = match &resolution.target {
            dir::ConstructTarget::Class(candidate) => {
                self.class_signature_item(query, candidate, resolution)?
            }
            dir::ConstructTarget::Dynamic { function, .. } => self.signature_node_item(
                query,
                function,
                &[],
                &resolution.arguments,
                resolution.return_type,
            )?,
            dir::ConstructTarget::Newtype(candidate) => {
                let Some(name) = query.symbol_name(candidate.symbol)? else {
                    return Err(QueryError::missing(format!(
                        "newtype constructor name: {:?}",
                        candidate.symbol
                    )));
                };
                let parameter_names = vec![None; resolution.arguments.len()];
                let parameter_documentation = vec![None; resolution.arguments.len()];

                self.signature_item(
                    query,
                    Some(candidate.symbol),
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
                    &parameter_documentation,
                    &resolution.arguments,
                    resolution.return_type,
                )?
            }
            dir::ConstructTarget::Variant(candidate) => {
                let symbol_id = candidate.case.variant;
                let module = query.module(symbol_id.module_id)?;
                let (declaring, definition, member) = module
                    .definitions()
                    .member(symbol_id)
                    .ok_or(QueryError::missing(format!(
                        "signature member: {symbol_id:?}"
                    )))?;
                let owner = definition
                    .member_owner(declaring)
                    .ok_or(QueryError::missing(format!(
                        "signature member owner: {symbol_id:?}"
                    )))?;
                let container = query
                    .symbol_name(owner)?
                    .ok_or(QueryError::missing(format!(
                        "signature member owner name: {owner:?}"
                    )))?;
                let member_name =
                    member
                        .name(module.strings())
                        .ok_or(QueryError::missing(format!(
                            "signature member name: {symbol_id:?}"
                        )))?;
                let name = format!("{container}.{member_name}");
                let parameter_names = vec![None; resolution.arguments.len()];
                let parameter_documentation = vec![None; resolution.arguments.len()];

                self.signature_item(
                    query,
                    Some(symbol_id),
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
                    &parameter_documentation,
                    &resolution.arguments,
                    resolution.return_type,
                )?
            }
        };

        Ok(vec![item])
    }

    /// Format one selected class constructor.
    fn class_signature_item(
        &self,
        query: &ProgramQueryContext<'_>,
        candidate: &dir::ClassConstructCandidate,
        resolution: &dir::ConstructResolution,
    ) -> QueryResult<SignatureItem> {
        let Some(name) = query.symbol_name(candidate.symbol)? else {
            return Err(QueryError::missing(format!(
                "class constructor name: {:?}",
                candidate.symbol
            )));
        };
        let Some(constructor_symbol) = candidate.constructor.call_symbol() else {
            return self.signature_item(
                query,
                Some(candidate.symbol),
                &name,
                &candidate.generic_arguments,
                &[],
                &[],
                &resolution.arguments,
                resolution.return_type,
            );
        };

        let Some(constructor_symbol) = query.canonical_symbol(constructor_symbol)? else {
            return Err(QueryError::missing(format!(
                "class constructor declaration: {constructor_symbol:?}"
            )));
        };
        let module = query.module(constructor_symbol.module_id)?;
        let Some(parameters) = module.callable_parameters(constructor_symbol.local_id) else {
            return Err(QueryError::missing(format!(
                "class constructor parameters: {constructor_symbol:?}"
            )));
        };

        self.selected_signature_item(
            query,
            &module,
            Some(constructor_symbol),
            &name,
            parameters,
            &candidate.generic_arguments,
            &resolution.arguments,
            resolution.return_type,
        )
    }

    /// Build one signature help item from a selected call signature.
    fn signature_item(
        &self,
        query: &ProgramQueryContext<'_>,
        symbol_id: Option<dir::GlobalSymbolId>,
        name: &str,
        generic_arguments: &[dir::GenericArgumentBinding],
        parameter_names: &[Option<String>],
        parameter_documentation: &[Option<String>],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let formatted = Formatter::new(self, query).applied_signature(
            name,
            generic_arguments,
            parameter_names,
            bindings,
            return_type,
        )?;
        let AppliedSignature { label, parameters } = formatted;
        if parameters.len() != parameter_documentation.len() {
            return Err(QueryError::invalid(format!(
                "signature parameter documentation: expected {}, found {}",
                parameters.len(),
                parameter_documentation.len()
            )));
        }
        let parameters = parameters
            .into_iter()
            .zip(parameter_documentation.iter().cloned())
            .map(|(label, documentation)| SignatureParameter {
                label,
                documentation,
            })
            .collect();

        let documentation = match symbol_id {
            Some(symbol_id) => query.symbol_documentation(symbol_id)?,
            None => None,
        };

        Ok(SignatureItem {
            label,
            documentation,
            parameters,
        })
    }

    /// Return the selected parameter at one cursor position.
    fn active_signature_parameter(
        &self,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        bindings: &[dir::ArgumentBinding],
        offset: u32,
    ) -> QueryResult<Option<usize>> {
        let view = self.view();

        // use the checker binding for an existing source argument
        for argument_id in arguments.iter().copied() {
            let span = self.node_span(view, argument_id.into_any())?;
            if offset < span.start || offset > span.end {
                continue;
            }

            let global_id = argument_id.into_global_any(self.module_id());
            let parameter = bindings
                .iter()
                .find(|binding| binding.contains_argument(global_id))
                .map(|binding| binding.parameter);
            let Some(parameter) = parameter else {
                return Ok(None);
            };

            return Ok(Some(parameter));
        }

        Ok(None)
    }
}
