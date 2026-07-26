use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::FileId;
use serde::{Deserialize, Serialize};

use crate::{
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
    format_global_type,
};

/// One parameter shown by signature help.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureParameter {
    /// The formatted parameter label.
    pub label: String,
    /// The checked parameter documentation when available.
    pub documentation: Option<String>,
}

/// One callable signature shown by signature help.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SignatureItem {
    /// The full formatted signature.
    pub label: String,
    /// The checked declaration documentation when available.
    pub documentation: Option<String>,
    /// The formatted parameters in declaration order.
    pub parameters: Vec<SignatureParameter>,
}

/// Signature help for one checked call.
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
    /// Return signature help for the innermost checked call at one offset.
    pub fn signature_help(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<SignatureHelp>> {
        let view = self.view();
        let enclosing = self.sorted_enclosing_spans(file_id, offset, offset);

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
                            resolution.arguments.as_slice(),
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
        let mut signatures = match &resolution.target {
            dir::CallTarget::Symbol(candidate) => {
                let Some(signature) = self.call_signature_item(
                    program,
                    candidate,
                    &resolution.arguments,
                    resolution.return_type,
                )?
                else {
                    return Ok(None);
                };

                vec![signature]
            }
            dir::CallTarget::Universal(candidates) => {
                let mut signatures = Vec::new();
                for candidate in candidates {
                    let Some(signature) = self.call_signature_item(
                        program,
                        candidate,
                        &resolution.arguments,
                        resolution.return_type,
                    )?
                    else {
                        return Ok(None);
                    };
                    signatures.push(signature);
                }

                signatures
            }
            dir::CallTarget::Expression { generic_arguments } => {
                let Some(signature) = self.expression_signature_item(
                    program,
                    callee_id,
                    generic_arguments,
                    &resolution.arguments,
                    resolution.return_type,
                )?
                else {
                    return Ok(None);
                };

                vec![signature]
            }
        };

        // retain one signature for repeated universal candidates
        signatures.dedup();

        Ok((!signatures.is_empty()).then_some(signatures))
    }

    /// Format one selected callable symbol from its recorded declaration.
    fn call_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        candidate: &dir::CallCandidate,
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let Some(symbol_id) = program.canonical_symbol(candidate.symbol)? else {
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
            &candidate.generic_arguments,
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

    /// Format one selected declaration signature with checked call types.
    #[allow(clippy::too_many_arguments)]
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
        let mut formatted_arguments = Vec::with_capacity(generic_arguments.len());
        for binding in generic_arguments {
            let Some(argument) = format_global_type(binding.argument, self, program)? else {
                return Ok(None);
            };
            formatted_arguments.push(argument);
        }
        let generic_arguments = formatted_arguments;
        let generics = if generic_arguments.is_empty() {
            String::new()
        } else {
            format!("<{}>", generic_arguments.join(", "))
        };
        let mut parameters = Vec::with_capacity(declared_parameters.len());
        for (index, parameter_id) in declared_parameters.iter().enumerate() {
            let parameter = declaration_module.view().get(*parameter_id);
            let Some(mut name) = declaration_module.parameter_name(parameter)? else {
                return Ok(None);
            };
            if parameter.is_optional() {
                name.push('?');
            }
            let Some(binding) = bindings.iter().find(|binding| binding.parameter == index) else {
                return Ok(None);
            };
            let Some(type_text) = format_global_type(binding.ty, self, program)? else {
                return Ok(None);
            };
            let label = format!("{name}: {type_text}");
            let documentation = declaration_module
                .view()
                .get_documentation(*parameter_id)
                .map(|documentation| {
                    declaration_module
                        .strings()
                        .get(documentation.text)
                        .to_string()
                });
            parameters.push(SignatureParameter {
                label,
                documentation,
            });
        }
        let Some(return_type) = format_global_type(return_type, self, program)? else {
            return Ok(None);
        };
        let parameter_text = parameters
            .iter()
            .map(|parameter| parameter.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Ok(Some(SignatureItem {
            label: format!("{name}{generics}({parameter_text}): {return_type}"),
            documentation: program.symbol_doc_text(symbol_id)?,
            parameters,
        }))
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
                let parameter_names = vec![Some("value")];

                let Some(item) = self.synthetic_signature_item(
                    program,
                    candidate.symbol,
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
                    &resolution.arguments,
                    resolution.return_type,
                )?
                else {
                    return Ok(None);
                };

                item
            }
            dir::ConstructTarget::Variant(candidate) => {
                let symbol_id = candidate.case.member;
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

                let Some(item) = self.synthetic_signature_item(
                    program,
                    symbol_id,
                    &name,
                    &candidate.generic_arguments,
                    &parameter_names,
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
            return self.synthetic_signature_item(
                program,
                candidate.symbol,
                &name,
                &candidate.generic_arguments,
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

    /// Format one checked signature without an authored parameter declaration.
    #[allow(clippy::too_many_arguments)]
    fn synthetic_signature_item(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
        name: &str,
        generic_arguments: &[dir::GenericArgumentBinding],
        parameter_names: &[Option<&str>],
        bindings: &[dir::ArgumentBinding],
        return_type: dir::GlobalTypeId,
    ) -> QueryResult<Option<SignatureItem>> {
        let mut formatted_arguments = Vec::with_capacity(generic_arguments.len());
        for binding in generic_arguments {
            let Some(argument) = format_global_type(binding.argument, self, program)? else {
                return Ok(None);
            };
            formatted_arguments.push(argument);
        }
        let generic_arguments = formatted_arguments;
        let generics = if generic_arguments.is_empty() {
            String::new()
        } else {
            format!("<{}>", generic_arguments.join(", "))
        };
        let mut bindings = bindings.iter().collect::<Vec<_>>();
        bindings.sort_by_key(|binding| binding.parameter);
        let mut parameters = Vec::with_capacity(bindings.len());
        for (index, binding) in bindings.into_iter().enumerate() {
            if binding.parameter != index {
                return Ok(None);
            }
            let Some(type_text) = format_global_type(binding.ty, self, program)? else {
                return Ok(None);
            };
            let label = match parameter_names.get(index).copied().flatten() {
                Some(name) => format!("{name}: {type_text}"),
                None => type_text,
            };
            parameters.push(SignatureParameter {
                label,
                documentation: None,
            });
        }
        let Some(return_type) = format_global_type(return_type, self, program)? else {
            return Ok(None);
        };
        let parameter_text = parameters
            .iter()
            .map(|parameter| parameter.label.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Ok(Some(SignatureItem {
            label: format!("{name}{generics}({parameter_text}): {return_type}"),
            documentation: program.symbol_doc_text(symbol_id)?,
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
                .find(|binding| binding_contains_argument(binding, global_id))
                .map(|binding| binding.parameter);
            let Some(parameter) = parameter else {
                return Ok(None);
            };

            return Ok(Some(parameter));
        }

        Ok(None)
    }
}

/// Return whether one checked binding owns a source argument.
fn binding_contains_argument(
    binding: &dir::ArgumentBinding,
    argument: dir::GlobalNodeIdAny,
) -> bool {
    match &binding.argument {
        dir::ArgumentSource::Provided(source) => *source == argument,
        dir::ArgumentSource::Rest(sources) => sources.contains(&argument),
        dir::ArgumentSource::Static(_) | dir::ArgumentSource::Omitted => false,
    }
}
