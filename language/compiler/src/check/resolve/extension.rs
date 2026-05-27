use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::check::{
    CheckState, Decision, GenericArgument, GenericSubstitution, MemberProtocol, TypeRelation,
    TypeTerm, VariableId,
};
use crate::{CompilerError, CompilerResult};

/// Extension member candidate visible to component checking.
#[derive(Debug, Clone, PartialEq)]
struct ExtensionCandidate {
    /// The extension declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The extension target type.
    target: VariableId,
    /// The extension where clauses.
    where_clauses: Vec<ExtensionWhereClause>,
    /// The resolved member symbol.
    member: dir::GlobalSymbolId,
}

/// Where clause attached to an extension candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExtensionWhereClause {
    /// The source where clause node.
    source: dir::GlobalNodeIdAny,
    /// The constrained type.
    left: VariableId,
    /// The required constraint type.
    right: VariableId,
}

impl CheckState<'_> {
    /// Return an applicable extension member for one applied nominal receiver.
    pub(in crate::check) fn extension_member(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<
        Option<(
            dir::GlobalSymbolId,
            dir::GlobalSymbolId,
            GenericSubstitution,
        )>,
    > {
        let extensions = self.extension_candidates(module, target, key)?;

        // accept the first extension whose target pattern and constraints hold
        for extension in extensions {
            let Some(mut substitution) =
                self.extension_substitution(extension.symbol, extension.target, target, arguments)?
            else {
                continue;
            };
            if let Some(protocol) = protocol
                && !self.symbol_matches_protocol(extension.symbol, protocol, &mut substitution)?
            {
                continue;
            }

            if self.extension_where_clauses_hold(&extension.where_clauses, &substitution)? {
                return Ok(Some((extension.symbol, extension.member, substitution)));
            }
        }

        Ok(None)
    }

    /// Return extension candidates visible from one module.
    fn extension_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<ExtensionCandidate>> {
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        self.collect_component_extension_candidates(module, key, &mut seen, &mut candidates)?;

        // add inherent extensions declared with the receiver type
        if target.module_id != module {
            self.collect_receiver_extension_candidates(
                module,
                target,
                key,
                &mut seen,
                &mut candidates,
            )?;
        }

        let imports = self
            .input(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, target)| target)
            .collect::<Vec<_>>();

        // add explicitly imported extension declarations
        for symbol in imports {
            self.add_extension_candidate(module, symbol, key, &mut seen, &mut candidates)?;
        }

        Ok(candidates)
    }

    /// Add component extension candidates declared in one module.
    fn collect_component_extension_candidates(
        &mut self,
        module: ModuleId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let symbols = {
            let view = self.input(module).view();
            let mut symbols = Vec::new();

            // collect extension symbols before creating variables
            for (id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
                let id: dir::LocalNodeId<dir::Declaration> = id;
                let dir::Declaration::Extension(_) = declaration else {
                    continue;
                };
                let symbol = self
                    .declaration_symbol(module, id.into_any())
                    .ok_or_else(|| CompilerError::Internal {
                        message: format!("extension declaration {id:?} has no symbol"),
                    })?;
                if self.member_symbol(module, symbol, key).is_none() {
                    continue;
                }

                symbols.push(symbol);
            }

            symbols
        };

        for symbol in symbols {
            self.add_extension_candidate(module, symbol, key, seen, candidates)?;
        }

        Ok(())
    }

    /// Add receiver-module extension candidates.
    fn collect_receiver_extension_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        if self.inputs.contains_key(&target.module_id) {
            self.collect_component_extension_candidates(target.module_id, key, seen, candidates)?;

            return Ok(());
        }

        let symbols = {
            let dependency = self.load_dependency_input(target.module_id)?;
            let mut symbols = Vec::new();

            // collect inherent dependency extensions for this receiver
            for extension_id in dependency.extensions.target_extensions(target) {
                let extension = dependency.extensions.get_extension(extension_id);
                if extension.is_inherent() {
                    symbols.push(extension.symbol);
                }
            }

            symbols
        };

        for symbol in symbols {
            self.add_extension_candidate(module, symbol, key, seen, candidates)?;
        }

        Ok(())
    }

    /// Add one extension candidate by declaration symbol.
    fn add_extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let candidate = if self.inputs.contains_key(&symbol.module_id) {
            self.component_extension_candidate(symbol, key)?
        } else {
            self.dependency_extension_candidate(module, symbol, key)?
        };
        let Some(candidate) = candidate else {
            return Ok(());
        };
        if seen.insert(candidate.symbol) {
            candidates.push(candidate);
        }

        Ok(())
    }

    /// Return one component extension candidate by declaration symbol.
    fn component_extension_candidate(
        &mut self,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<ExtensionCandidate>> {
        let (extension, member) = {
            let Some(source) = self.symbol_source_node(symbol.module_id, symbol) else {
                return Ok(None);
            };
            if source.ty != dir::NodeType::Declaration {
                return Ok(None);
            }
            let declaration = dir::LocalNodeId::<dir::Declaration>::new(source.id);
            let declaration = self.input(symbol.module_id).view().get(declaration).clone();
            let dir::Declaration::Extension(extension) = declaration else {
                return Ok(None);
            };
            let Some(member) = self.member_symbol(symbol.module_id, symbol, key) else {
                return Ok(None);
            };

            (extension, member)
        };

        let target = self.intern_local_type_variable(symbol.module_id, extension.target_type);
        let where_clauses = extension
            .where_clauses
            .into_iter()
            .map(|where_clause| {
                let source = where_clause.into_global_any(symbol.module_id);
                let where_clause = self
                    .input(symbol.module_id)
                    .view()
                    .get(where_clause)
                    .clone();
                let left = self.intern_local_type_variable(symbol.module_id, where_clause.left);
                let right = self.intern_local_type_variable(symbol.module_id, where_clause.right);

                ExtensionWhereClause {
                    source,
                    left,
                    right,
                }
            })
            .collect();

        Ok(Some(ExtensionCandidate {
            symbol,
            target,
            where_clauses,
            member,
        }))
    }

    /// Return one checked dependency extension candidate by declaration symbol.
    fn dependency_extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<ExtensionCandidate>> {
        let dependency_candidate = {
            let dependency = self.load_dependency_input(symbol.module_id)?;
            let Some(extension_id) = dependency.extensions.symbol_extension_id(symbol) else {
                return Ok(None);
            };
            let extension = dependency.extensions.get_extension(extension_id).clone();
            let members = Self::binding_member_symbols(
                &dependency.bindings,
                Some(&dependency.view()),
                symbol,
                dir::MemberSlot::Key(key),
            );
            let Some(member) = members.first().copied() else {
                return Ok(None);
            };
            let symbol_entry = dependency.bindings.get_symbol(symbol.local_id);
            let Some(source) = symbol_entry.declaration else {
                return Err(CompilerError::Internal {
                    message: format!("dependency extension {symbol:?} has no declaration"),
                });
            };
            if source.local_id.ty != dir::NodeType::Declaration {
                return Err(CompilerError::Internal {
                    message: format!("dependency extension {symbol:?} source is not a declaration"),
                });
            }
            let declaration = dir::LocalNodeId::<dir::Declaration>::new(source.local_id.id);
            let declaration = dependency.view().get(declaration).clone();
            let dir::Declaration::Extension(declaration) = declaration else {
                return Err(CompilerError::Internal {
                    message: format!("dependency extension {symbol:?} source is not an extension"),
                });
            };
            let where_clauses = declaration
                .where_clauses
                .into_iter()
                .map(|where_clause| {
                    let source = where_clause.into_global_any(symbol.module_id);
                    let where_clause_node = dependency.view().get(where_clause).clone();
                    let left = dependency
                        .types
                        .get_node_type_id(where_clause_node.left.into_global_any(symbol.module_id))
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "checked dependency where clause {where_clause:?} has no left type"
                            ),
                        })?;
                    let right = dependency
                        .types
                        .get_node_type_id(where_clause_node.right.into_global_any(symbol.module_id))
                        .ok_or_else(|| CompilerError::Internal {
                            message: format!(
                                "checked dependency where clause {where_clause:?} has no right type"
                            ),
                        })?;

                    Ok((source, left, right))
                })
                .collect::<CompilerResult<Vec<_>>>()?;
            let types = dependency.types.clone();
            let statics = dependency.statics.clone();
            let generics = dependency.generics.clone();

            (extension, member, where_clauses, types, statics, generics)
        };
        let (extension, member, where_clauses, types, statics, generics) = dependency_candidate;
        let target_type = self.import_dependency_type(
            module,
            symbol.module_id,
            extension.target_type,
            &types,
            &statics,
            &generics,
        );
        let target = self.materialize_type_id(target_type.into_global(module));
        let where_clauses = where_clauses
            .into_iter()
            .map(|(source, left, right)| {
                let left = self.import_dependency_type(
                    module,
                    symbol.module_id,
                    left,
                    &types,
                    &statics,
                    &generics,
                );
                let left = self.materialize_type_id(left.into_global(module));
                let right = self.import_dependency_type(
                    module,
                    symbol.module_id,
                    right,
                    &types,
                    &statics,
                    &generics,
                );
                let right = self.materialize_type_id(right.into_global(module));

                ExtensionWhereClause {
                    source,
                    left,
                    right,
                }
            })
            .collect();

        Ok(Some(ExtensionCandidate {
            symbol,
            target,
            where_clauses,
            member,
        }))
    }

    /// Match an extension target pattern against an applied receiver.
    fn extension_substitution(
        &mut self,
        extension: dir::GlobalSymbolId,
        target_variable: VariableId,
        receiver: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<GenericSubstitution>> {
        let Some(pattern) = self.solved_type_term(target_variable)? else {
            return Ok(None);
        };
        let actual = TypeTerm::Reference {
            source: None,
            symbol: receiver,
            arguments: arguments.to_vec().into(),
        };
        let mut substitution = GenericSubstitution::empty();
        let is_match = self.match_type_pattern(
            target_variable.module,
            extension,
            &pattern,
            &actual,
            &mut substitution,
        )?;

        Ok(is_match.then_some(substitution))
    }

    /// Return whether substituted extension where clauses hold.
    fn extension_where_clauses_hold(
        &mut self,
        where_clauses: &[ExtensionWhereClause],
        substitution: &GenericSubstitution,
    ) -> CompilerResult<bool> {
        for where_clause in where_clauses {
            let Some(left) = self.solved_type_term(where_clause.left)? else {
                return Ok(false);
            };
            let Some(right) = self.solved_type_term(where_clause.right)? else {
                return Ok(false);
            };
            let Some(left) = left.substitute(where_clause.left.module, substitution, self)? else {
                return Ok(false);
            };
            let Some(right) = right.substitute(where_clause.right.module, substitution, self)?
            else {
                return Ok(false);
            };

            let decision =
                self.decide_type_term_relation(TypeRelation::Satisfies, &left, &right)?;
            let decision = if decision == Decision::Undecidable {
                self.decide_structural_satisfies(where_clause.source.module_id, &left, &right)?
            } else {
                decision
            };
            if decision != Decision::Yes {
                return Ok(false);
            };
        }

        Ok(true)
    }
}
