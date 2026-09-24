use std::slice;

use destack_dir as dir;
use destack_serde::Reflect;
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

/// A signature help request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A signature help response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureHelpResponse {
    /// Signature help, if available.
    pub help: Option<SignatureHelp>,
}

impl ModuleQueryContext<'_> {
    /// Return signature help for the innermost selected call at one offset.
    pub fn signature_help(
        &self,
        request: SignatureHelpRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<SignatureHelpResponse> {
        // select the innermost authored call
        let position = request.position;
        let file_id = position.file_id;
        let offset = position.offset;
        let cursor = self.cursor(file_id, offset)?;
        let Some(call) = cursor.call()? else {
            return Ok(SignatureHelpResponse { help: None });
        };

        // read the recorded decision for the selected call
        let global_id = call.id.into_global_any(self.module_id());
        let decisions = self.decisions()?;
        let (signatures, bindings) = match decisions.decision(global_id) {
            Some(dir::Decision::Construct(selection)) => {
                let signatures = self.construct_signature_items(program, selection)?;

                (signatures, Some(selection.arguments.as_slice()))
            }
            Some(dir::Decision::Call(selection)) => {
                let signatures =
                    self.call_signature_items(program, call.target, selection.arms())?;

                (signatures, decisions.agreed_call_arguments(global_id))
            }
            Some(dir::Decision::Attempted(selection)) => {
                let signatures = self.call_signature_items(
                    program,
                    call.target,
                    slice::from_ref(selection.as_ref()),
                )?;

                (signatures, Some(selection.arguments.as_slice()))
            }
            _ => return Ok(SignatureHelpResponse { help: None }),
        };

        // select the active parameter from the recorded argument bindings
        let active_parameter = match bindings {
            Some(bindings) => self.active_parameter(call.arguments, bindings, offset)?,
            None => None,
        };

        // return the formatted signatures and active parameter
        let help = SignatureHelp {
            signatures,
            active_signature: 0,
            active_parameter,
        };

        Ok(SignatureHelpResponse { help: Some(help) })
    }

    /// Format every statically selected symbol call target.
    fn call_signature_items(
        &self,
        program: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeIdAny,
        calls: &[dir::Call],
    ) -> QueryResult<Vec<SignatureItem>> {
        let mut signatures = Vec::new();
        for call in calls {
            let signature = match &call.target {
                dir::CallableTarget::Symbol { function, .. } => self.call_signature_item(
                    program,
                    function.key.symbol,
                    &function.key.arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallableTarget::Dynamic {
                    function: dir::DynamicFunction::Symbol(symbol),
                    generic_arguments,
                    ..
                } => self.call_signature_item(
                    program,
                    *symbol,
                    generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallableTarget::Dynamic {
                    function,
                    generic_arguments,
                    ..
                } => self.signature_node_item(
                    program,
                    function,
                    generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallableTarget::Expression { .. } | dir::CallableTarget::Constructor(_) => {
                    self.expression_signature_item(
                        program,
                        callee_id,
                        call.callable_type,
                        call.target.generic_arguments(),
                        &call.arguments,
                        call.return_type,
                    )?
                }
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
        program: &ProgramQueryContext<'_>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let Some(symbol_id) = program.symbol_target(symbol)? else {
            return Err(QueryError::missing(format!(
                "call signature declaration: {symbol:?}"
            )));
        };
        let module = program.module(symbol_id.module_id)?;
        let symbols = module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(name) = symbol.name() else {
            return Err(QueryError::missing(format!(
                "call signature name: {symbol_id:?}"
            )));
        };
        let name = module.strings().get(name).to_string();
        let Some(parameters) = module.callable_parameters(symbol_id.local_id)? else {
            return Err(QueryError::missing(format!(
                "call signature parameters: {symbol_id:?}"
            )));
        };
        let documentation_owner = symbol.declaration.map(|declaration| declaration.local_id);

        self.selected_signature_item(
            program,
            &module,
            documentation_owner,
            &name,
            parameters,
            generic_arguments,
            bindings,
            return_type,
        )
    }

    /// Format one expression-backed callable key.
    fn expression_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeIdAny,
        callable_type: dir::GlobalTypeId,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        // format the authored callee and exact checked parameter types
        let span = self.node_span(self.view()?, callee_id)?;
        let name = self.source_text(span)?;
        let formatter = Formatter::new(self, program);
        let parameter_names = formatter.callable_parameter_labels(callable_type)?;
        let parameter_documentation = vec![None; bindings.len()];

        self.signature_item(
            program,
            None,
            &name,
            generic_arguments,
            &parameter_names,
            &parameter_documentation,
            bindings,
            return_type,
        )
    }

    /// Format one signature-backed dynamic key from its declaring node.
    fn signature_node_item(
        &self,
        program: &ProgramQueryContext<'_>,
        function: &dir::DynamicFunction,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        // select the declaring signature and display name
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
        let module = program.module(node.module_id)?;
        let Some(parameters) = module.node_callable_parameters(node.local_id)? else {
            return Err(QueryError::missing(format!(
                "dynamic signature parameters: {node:?}"
            )));
        };

        self.selected_signature_item(
            program,
            &module,
            Some(node.local_id),
            name,
            parameters,
            generic_arguments,
            bindings,
            return_type,
        )
    }

    /// Format one selected declaration signature with applied call types.
    fn selected_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        declaration_module: &ModuleQueryContext<'_>,
        documentation_owner: Option<dir::LocalNodeIdAny>,
        name: &str,
        declared_parameters: &[dir::LocalNodeId<dir::Parameter>],
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let mut names = Vec::with_capacity(declared_parameters.len());
        let mut parameter_documentation = Vec::with_capacity(declared_parameters.len());
        let formatter = Formatter::new(declaration_module, program);
        let view = declaration_module.view()?;

        // retain authored parameter names and documentation
        for parameter_id in declared_parameters {
            let parameter = view.get(*parameter_id);
            let mut name = declaration_module.parameter_name(parameter)?;
            if parameter.is_optional() {
                name.push('?');
            }
            names.push(Some(name));

            let documentation =
                formatter.parameter_documentation(documentation_owner, *parameter_id)?;
            parameter_documentation.push(documentation);
        }

        // retain callable documentation without repeating its parameter sections
        let documentation = documentation_owner
            .and_then(|owner| view.get_documentation_any(owner))
            .map(|documentation| formatter.callable_documentation(documentation))
            .transpose()?;

        self.signature_item(
            program,
            documentation,
            name,
            generic_arguments,
            &names,
            &parameter_documentation,
            bindings,
            return_type,
        )
    }

    /// Format the exact constructor selected by checking.
    fn construct_signature_items(
        &self,
        program: &ProgramQueryContext<'_>,
        resolution: &dir::ConstructDecision,
    ) -> QueryResult<Vec<SignatureItem>> {
        let item = match &resolution.target {
            dir::ConstructTarget::Class {
                key, constructor, ..
            } => self.class_signature_item(program, key, constructor, resolution)?,
            dir::ConstructTarget::Newtype { key, .. } => {
                let Some(name) = program.symbol_name(key.symbol)? else {
                    return Err(QueryError::missing(format!(
                        "newtype constructor name: {:?}",
                        key.symbol
                    )));
                };
                let parameter_names = vec![None; resolution.arguments.len()];
                let parameter_documentation = vec![None; resolution.arguments.len()];

                self.signature_item(
                    program,
                    program.symbol_callable_documentation(key.symbol)?,
                    &name,
                    &key.arguments,
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
        program: &ProgramQueryContext<'_>,
        key: &dir::InstanceKey,
        constructor: &dir::ClassConstructor,
        resolution: &dir::ConstructDecision,
    ) -> QueryResult<SignatureItem> {
        let Some(name) = program.symbol_name(key.symbol)? else {
            return Err(QueryError::missing(format!(
                "class constructor name: {:?}",
                key.symbol
            )));
        };
        let Some(constructor_symbol) = constructor.call_symbol() else {
            return self.signature_item(
                program,
                program.symbol_callable_documentation(key.symbol)?,
                &name,
                &key.arguments,
                &[],
                &[],
                &resolution.arguments,
                resolution.return_type,
            );
        };

        let Some(constructor_symbol) = program.symbol_target(constructor_symbol)? else {
            return Err(QueryError::missing(format!(
                "class constructor declaration: {constructor_symbol:?}"
            )));
        };
        let module = program.module(constructor_symbol.module_id)?;
        let Some(parameters) = module.callable_parameters(constructor_symbol.local_id)? else {
            return Err(QueryError::missing(format!(
                "class constructor parameters: {constructor_symbol:?}"
            )));
        };
        let symbol = module.bindings()?.get_symbol(constructor_symbol.local_id);
        let documentation_owner = symbol.declaration.map(|declaration| declaration.local_id);

        self.selected_signature_item(
            program,
            &module,
            documentation_owner,
            &name,
            parameters,
            &key.arguments,
            &resolution.arguments,
            resolution.return_type,
        )
    }

    /// Build one signature help item from a selected call signature.
    fn signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        documentation: Option<String>,
        name: &str,
        generic_arguments: &[dir::GenericArgumentBinding],
        parameter_names: &[Option<String>],
        parameter_documentation: &[Option<String>],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<SignatureItem> {
        let formatted = Formatter::new(self, program).applied_signature(
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

        Ok(SignatureItem {
            label,
            documentation,
            parameters,
        })
    }
}
