use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{Answer, CheckState, Decision, Dependency, Relation, answer};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select one explicit instantiation once its target name decides.
    pub(in crate::check) fn select_instantiation(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<Answer<()>> {
        let module = node.module_id;
        let source = node.local_id.into_any();
        let node = node.into_any();
        let origin = self.node_site(node)?.origin();

        // read the decided target name
        let left_node = left.into_global_any(module);
        let symbol = match self.decision(left_node) {
            Some(Decision::Name(resolution)) => match resolution.symbols() {
                [symbol] => *symbol,
                _ => {
                    let Some(path) = self.module(module).view().tree().reference_path(left) else {
                        return Err(CompilerError::Internal {
                            message: format!(
                                "overloaded instantiation target {left_node:?} has no reference path"
                            ),
                        });
                    };
                    self.report_ambiguous_reference(module, left.into_any(), &path);
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }
            },
            Some(Decision::Rejected) => {
                self.commit_decision(node, Decision::Rejected)?;
                self.commit_error_node(node)?;

                return Ok(Answer::Ready(()));
            }
            Some(other) => {
                return Err(CompilerError::Internal {
                    message: format!("instantiation target {left_node:?} decided as {other:?}"),
                });
            }
            None => return Ok(Answer::pending([Dependency::Decision(left_node)])),
        };

        // collect the written argument types
        let mut applied = SmallVec::<[dir::GlobalTypeId; 2]>::new();
        for argument in arguments {
            let argument = argument.into_global_any(module);
            let ty = answer!(self.node_type(argument)?);
            applied.push(ty);
        }

        // read the selected declaration template
        let template = self.symbol_template(symbol);
        if template.is_none() && !applied.is_empty() {
            let name = self.format_symbol(symbol);
            self.report_wrong_generic_arity(module, source, name, 0, applied.len());
            self.commit_decision(node, Decision::Rejected)?;
            self.commit_error_node(node)?;

            return Ok(Answer::Ready(()));
        }

        // specialize the selected value type by the applied arguments
        let Some(declared) = self.symbol_type_maybe(symbol) else {
            return Ok(Answer::pending([Dependency::SymbolType(symbol)]));
        };
        let specialized = match template {
            Some(template) => {
                let parameters = self.generic_template_parameters(template);
                let Some(substitution) =
                    self.apply_template_arguments(origin, template, &applied)?
                else {
                    let name = self.format_symbol(symbol);
                    let written_count = self.written_parameter_count(&parameters);
                    self.report_wrong_generic_arity(
                        module,
                        source,
                        name,
                        written_count,
                        applied.len(),
                    );
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                };

                // check written arguments against declared bounds
                let sources = SmallVec::<[dir::GlobalNodeIdAny; 4]>::from_iter(
                    std::iter::repeat_n(node, substitution.arguments.len()),
                );
                if let Some(rejection) = answer!(self.check_generic_arguments(
                    origin,
                    &parameters,
                    &substitution.arguments,
                    &sources,
                    &substitution,
                )?) {
                    self.relate(
                        self.origin_at(origin, rejection.source),
                        Relation::Satisfies,
                        None,
                        rejection.argument,
                        rejection.bound,
                    )?;
                    self.commit_decision(node, Decision::Rejected)?;
                    self.commit_error_node(node)?;

                    return Ok(Answer::Ready(()));
                }

                match self.ty(declared)? {
                    dir::Type::Reference(reference) if reference.symbol == symbol => {
                        let arguments = self.intern_type_ids(module, &substitution.arguments)?;

                        self.intern_type(
                            module,
                            dir::Type::Instance(dir::GenericInstance { symbol, arguments }),
                        )?
                    }
                    _ => self.substitute_type(module, declared, &substitution)?,
                }
            }
            None => declared,
        };

        let arguments = self.symbol_generic_argument_bindings(symbol, &applied)?;
        let resolution = dir::InstantiationResolution::new(symbol, arguments);
        self.commit_node_type(node, specialized)?;
        self.commit_decision(node, Decision::Instantiation(resolution))?;

        Ok(Answer::Ready(()))
    }
}
