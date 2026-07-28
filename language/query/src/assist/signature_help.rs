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
        program: &ProgramQueryContext<'_>,
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
                    if let Some(resolution) = self.resolutions().construct_resolution(global_id) {
                        let signatures =
                            self.construct_signature_items(program, resolution)?.ok_or(
                                QueryError::missing(format!("signature help: {global_id:?}")),
                            )?;

                        (
                            signatures,
                            arguments.as_slice(),
                            resolution.arguments.as_slice(),
                        )
                    } else {
                        let Some(resolution) = self.resolutions().call_resolution(global_id) else {
                            continue;
                        };
                        let signatures = self
                            .call_signature_items(program, *left, resolution)?
                            .ok_or(QueryError::missing(format!(
                                "signature help: {global_id:?}"
                            )))?;

                        (
                            signatures,
                            arguments.as_slice(),
                            resolution.first().arguments.as_slice(),
                        )
                    }
                }
                dir::Expression::New { arguments, .. }
                | dir::Expression::NewMaybe { arguments, .. } => {
                    let Some(resolution) = self.resolutions().construct_resolution(global_id)
                    else {
                        continue;
                    };
                    let signatures = self.construct_signature_items(program, resolution)?.ok_or(
                        QueryError::missing(format!("signature help: {global_id:?}")),
                    )?;

                    (
                        signatures,
                        arguments.as_slice(),
                        resolution.arguments.as_slice(),
                    )
                }
                _ => continue,
            };

            let active_parameter = self.active_signature_parameter(arguments, bindings, offset)?;

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
        program: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        resolution: &dir::CallResolution,
    ) -> QueryResult<Option<Vec<SignatureItem>>> {
        let mut signatures = Vec::new();
        for call in resolution.iter() {
            let signature = match &call.target {
                dir::CallTarget::Symbol { function, .. } => self.call_signature_item(
                    program,
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
                    program,
                    *symbol,
                    generic_arguments,
                    &call.arguments,
                    call.return_type,
                )?,
                dir::CallTarget::Dynamic { .. } => None,
                dir::CallTarget::Expression { generic_arguments } => self
                    .expression_signature_item(
                        program,
                        callee_id,
                        generic_arguments,
                        &call.arguments,
                        call.return_type,
                    )?,
            };
            let Some(signature) = signature else {
                return Ok(None);
            };
            signatures.push(signature);
        }

        // retain one signature for repeated arm selections
        signatures.dedup();

        Ok((!signatures.is_empty()).then_some(signatures))
    }

    /// Format one selected callable symbol from its recorded declaration.
    fn call_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol: dir::GlobalSymbolId,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let Some(symbol_id) = program.canonical_symbol(symbol)? else {
            return Ok(None);
        };
        let module = program.module(symbol_id.module_id)?;
        let symbols = module.symbols();
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let Some(name) = symbol.name() else {
            return Ok(None);
        };
        let name = module.strings().get(name).to_string();
        let Some(parameters) = module.callable_parameters(symbol_id) else {
            return Ok(None);
        };

        self.selected_signature_item(
            program,
            module,
            symbol_id,
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
        program: &ProgramQueryContext<'_>,
        callee_id: dir::LocalNodeId<dir::Expression>,
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let source = callee_id.into_global_any(self.module_id());
        let Some(symbols) = self.recorded_symbol_targets(source) else {
            return Ok(None);
        };
        let [symbol_id] = symbols.as_slice() else {
            return Ok(None);
        };
        let symbol = self.symbols().get_symbol(symbol_id.local_id);
        let Some(name) = symbol.name() else {
            return Ok(None);
        };
        let name = self.strings().get(name).to_string();
        let Some(parameters) = self.variable_callable_parameters(*symbol_id) else {
            return Ok(None);
        };

        self.selected_signature_item(
            program,
            self,
            *symbol_id,
            &name,
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
        symbol_id: dir::GlobalSymbolId,
        name: &str,
        declared_parameters: &[dir::LocalNodeId<dir::Parameter>],
        generic_arguments: &[dir::GenericArgumentBinding],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let mut names = Vec::with_capacity(declared_parameters.len());
        let mut documentation = Vec::with_capacity(declared_parameters.len());

        // retain authored parameter names and documentation
        for parameter_id in declared_parameters {
            let parameter = declaration_module.view().get(*parameter_id);
            let Some(mut name) = declaration_module.parameter_name(parameter)? else {
                return Ok(None);
            };
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
            program,
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
        program: &ProgramQueryContext<'_>,
        resolution: &dir::ConstructResolution,
    ) -> QueryResult<Option<Vec<SignatureItem>>> {
        let item = match &resolution.target {
            dir::ConstructTarget::Class(candidate) => {
                let Some(item) = self.class_signature_item(program, candidate, resolution)? else {
                    return Ok(None);
                };

                item
            }
            dir::ConstructTarget::Newtype(candidate) => {
                let Some(name) = program.symbol_name(candidate.symbol)? else {
                    return Ok(None);
                };
                let parameter_names = vec![None; resolution.arguments.len()];
                let parameter_documentation = vec![None; resolution.arguments.len()];

                let Some(item) = self.signature_item(
                    program,
                    candidate.symbol,
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
                    &parameter_documentation,
                    &resolution.arguments,
                    resolution.return_type,
                )?
                else {
                    return Ok(None);
                };

                item
            }
            dir::ConstructTarget::Variant(candidate) => {
                let symbol_id = candidate.case.variant;
                let Some(member) = program
                    .module_index(symbol_id.module_id)?
                    .members
                    .symbol_entry(symbol_id)
                else {
                    return Ok(None);
                };
                let Some(owner) = member.owner else {
                    return Ok(None);
                };
                let Some(container) = program.symbol_name(owner)? else {
                    return Ok(None);
                };
                let name = format!("{container}.{}", member.name);
                let parameter_names = vec![None; resolution.arguments.len()];
                let parameter_documentation = vec![None; resolution.arguments.len()];

                let Some(item) = self.signature_item(
                    program,
                    symbol_id,
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
                    &parameter_documentation,
                    &resolution.arguments,
                    resolution.return_type,
                )?
                else {
                    return Ok(None);
                };

                item
            }
        };

        Ok(Some(vec![item]))
    }

    /// Format one selected class constructor.
    fn class_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        candidate: &dir::ClassConstructCandidate,
        resolution: &dir::ConstructResolution,
    ) -> QueryResult<Option<SignatureItem>> {
        let Some(name) = program.symbol_name(candidate.symbol)? else {
            return Ok(None);
        };
        let Some(constructor_symbol) = candidate.constructor.call_symbol() else {
            return self.signature_item(
                program,
                candidate.symbol,
                &name,
                &candidate.generic_arguments,
                &[],
                &[],
                &resolution.arguments,
                resolution.return_type,
            );
        };

        let Some(constructor_symbol) = program.canonical_symbol(constructor_symbol)? else {
            return Ok(None);
        };
        let module = program.module(constructor_symbol.module_id)?;
        let Some(parameters) = module.callable_parameters(constructor_symbol) else {
            return Ok(None);
        };

        self.selected_signature_item(
            program,
            module,
            constructor_symbol,
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
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        name: &str,
        generic_arguments: &[dir::GenericArgumentBinding],
        parameter_names: &[Option<String>],
        parameter_documentation: &[Option<String>],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let Some(formatted) = Formatter::new(self, program).applied_signature(
            name,
            generic_arguments,
            parameter_names,
            bindings,
            return_type,
        )?
        else {
            return Ok(None);
        };
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

        Ok(Some(SignatureItem {
            label,
            documentation: program.symbol_documentation(symbol_id)?,
            parameters,
        }))
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
