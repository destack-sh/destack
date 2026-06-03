use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, ExtensionWhereClause, GenericArgument, GenericInstance,
    GenericParameterId, GenericSubstitution, MemberProtocol, Origin, ShapeMember, Substitution,
    TypeOperand, TypeRelation, TypeTerm,
};

/// A symbol-backed member candidate found by lookup.
pub(in crate::check) struct MemberCandidate {
    /// The resolved member symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The resolved member type.
    pub(in crate::check) ty: TypeTerm,
    /// The resolved generic instance, when lookup instantiated an owner.
    pub(in crate::check) instance: Option<GenericInstance>,
}

/// Extension member candidate available to component checking.
#[derive(Debug, Clone, PartialEq)]
struct ExtensionCandidate {
    /// The extension declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The extension target type.
    target: TypeOperand,
    /// The extension where clauses.
    where_clauses: Vec<ExtensionWhereClause>,
    /// The resolved member symbol.
    member: dir::GlobalSymbolId,
}

impl CheckState<'_> {
    /// Return member symbols declared under one owner and key.
    pub(in crate::check) fn lookup_member_symbols(
        &self,
        owner: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> SmallVec<[dir::GlobalSymbolId; 4]> {
        let lookup = if let Some(module) = self.modules.get(&owner.module_id) {
            module
                .binding_table()
                .lookup_key_member(owner.local_id, key)
        } else {
            self.dependency(owner.module_id)
                .bindings
                .lookup_key_member(owner.local_id, key)
        };

        match lookup {
            dir::SymbolLookup::Missing => SmallVec::new(),
            dir::SymbolLookup::Found(symbol) => {
                let mut symbols = SmallVec::new();
                symbols.push(symbol.into_global(owner.module_id));

                symbols
            }
            dir::SymbolLookup::Ambiguous(symbols) => symbols
                .into_iter()
                .map(|symbol| symbol.into_global(owner.module_id))
                .collect(),
        }
    }

    /// Return symbol-backed member candidates from one reduced type term.
    pub(in crate::check) fn member_type_candidates(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = match reduction.value.as_ref() {
            Some(reduced) => reduced,
            None => term,
        };

        self.member_type_candidates_matching(origin, module, term, key, None)
    }

    /// Return a symbol-backed member candidate that satisfies one protocol.
    pub(in crate::check) fn member_type_candidate_for_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<MemberCandidate>> {
        let Some(mut candidates) =
            self.member_type_candidates_for_protocol(origin, module, term, key, protocol)?
        else {
            return Ok(None);
        };
        if candidates.len() != 1 {
            return Ok(None);
        }

        Ok(candidates.pop())
    }

    /// Return symbol-backed member candidates that satisfy one protocol.
    pub(in crate::check) fn member_type_candidates_for_protocol(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = match reduction.value.as_ref() {
            Some(reduced) => reduced,
            None => term,
        };

        self.member_type_candidates_matching(origin, module, term, key, Some(protocol))
    }

    /// Return symbol-backed member candidates from one reduced type term.
    pub(in crate::check) fn member_type_candidates_matching(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        if protocol.is_some() {
            todo!("filter member candidates through nominal protocol table")
        }

        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(None);
                };

                self.member_type_candidates_matching(origin, module, &term, key, protocol)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.type_generic_parameter_for_symbol(*symbol)? {
                    self.parameter_member_candidates(module, origin, parameter, key, protocol)
                } else {
                    self.instantiate_symbol_member_candidate(
                        module, *symbol, arguments, *key, protocol,
                    )
                    .map(|candidate| candidate.map(|candidate| vec![candidate]))
                }
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self
                .instantiate_symbol_member_candidate(module, *symbol, arguments, *key, protocol)
                .map(|candidate| candidate.map(|candidate| vec![candidate])),
            TypeTerm::Parameter(parameter) => {
                self.parameter_member_candidates(module, origin, *parameter, key, protocol)
            }
            TypeTerm::Shape(shape) => self
                .shape_member_candidate(&self.inference.term(*shape).members, key, protocol)
                .map(|candidate| candidate.map(|candidate| vec![candidate])),
            TypeTerm::Union { elements } => {
                self.union_member_candidates(origin, module, elements, key, protocol)
            }
            TypeTerm::Operation(operation) => {
                self.operation_member_candidates(origin, module, *operation, key, protocol)
            }
            _ => Ok(None),
        }
    }

    /// Return member candidates through a generic parameter constraint.
    fn parameter_member_candidates(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<Vec<MemberCandidate>>> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(None);
        };
        let Some(term) = self.reduced_generic_type_constraint(origin, constraint)? else {
            return Ok(None);
        };

        self.member_type_candidates_matching(origin, module, &term, key, protocol)
    }

    /// Return a symbol-backed candidate from one shape method field.
    fn shape_member_candidate(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        if protocol.is_some() {
            return Ok(None);
        }

        for member in members {
            let ShapeMember::Field {
                key: member_key,
                ty,
                ..
            } = member
            else {
                continue;
            };
            if !member_key.matches(key) {
                continue;
            };
            let Some(term) = self.type_operand_term(*ty)? else {
                return Ok(None);
            };
            let TypeOperand::Variable(variable) = *ty else {
                return Ok(None);
            };
            let Origin::Symbol(symbol) = self.variable(variable).source else {
                return Ok(None);
            };

            return Ok(Some(MemberCandidate {
                symbol,
                ty: term,
                instance: None,
            }));
        }

        Ok(None)
    }

    /// Instantiate a member candidate from one applied nominal declaration.
    fn instantiate_symbol_member_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
        protocol: Option<&MemberProtocol>,
    ) -> CompilerResult<Option<MemberCandidate>> {
        for member in self.lookup_member_symbols(symbol, key) {
            let substitution = self.generic_substitution(symbol, arguments)?;
            if self.reduce_symbol_availability(module, member, (&substitution).into())?
                != Decision::Yes
            {
                return Ok(None);
            }
            let operand = self.import_symbol_type_operand(module, member);
            if substitution.is_empty() {
                let Some(term) = self.type_operand_term(operand)? else {
                    return Ok(None);
                };

                return Ok(Some(MemberCandidate {
                    symbol: member,
                    ty: term,
                    instance: None,
                }));
            }
            let Some(term) = self.type_operand_term(operand)? else {
                return Ok(None);
            };
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(None);
            };
            let instance = Some(GenericInstance {
                owner: symbol,
                arguments: arguments.to_vec().into(),
            });

            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: term,
                instance: instance,
            }));
        }

        let Some((extension, member, substitution)) =
            self.extension_member(module, symbol, arguments, key, protocol)?
        else {
            return Ok(None);
        };
        if self.reduce_symbol_availability(module, member, (&substitution).into())? != Decision::Yes
        {
            return Ok(None);
        }
        let variable = self.import_symbol_type_operand(module, member);
        if substitution.is_empty() {
            let Some(term) = self.type_operand_term(variable)? else {
                return Ok(None);
            };

            return Ok(Some(MemberCandidate {
                symbol: member,
                ty: term,
                instance: None,
            }));
        }
        let Some(term) = self.type_operand_term(variable)? else {
            return Ok(None);
        };
        let Some(term) = term.substitute(module, &substitution, self)? else {
            return Ok(None);
        };
        let instance = self.substitution_application(extension, &substitution)?;

        Ok(Some(MemberCandidate {
            symbol: member,
            ty: term,
            instance,
        }))
    }

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
            let Some(substitution) =
                self.extension_substitution(extension.symbol, extension.target, target, arguments)?
            else {
                continue;
            };
            if protocol.is_some() {
                todo!("filter extension candidates through nominal protocol table")
            }

            if self
                .extension_where_clauses_hold(&extension.where_clauses, (&substitution).into())?
            {
                return Ok(Some((extension.symbol, extension.member, substitution)));
            }
        }

        Ok(None)
    }

    /// Return extension candidates available from one module.
    fn extension_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<ExtensionCandidate>> {
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        let symbols = self.extension_symbols_for_target(module, target)?;
        for symbol in symbols {
            self.collect_extension_candidate(module, symbol, key, &mut seen, &mut candidates)?;
        }

        // add inherent extensions declared with the receiver type
        if target.module_id != module {
            let symbols = self.extension_symbols_for_target(target.module_id, target)?;
            for symbol in symbols {
                self.collect_extension_candidate(module, symbol, key, &mut seen, &mut candidates)?;
            }
        }

        let imports = self
            .module(module)
            .resolved
            .imports
            .symbol_targets()
            .map(|(_, target)| target)
            .collect::<Vec<_>>();

        // add explicitly imported extension declarations
        for symbol in imports {
            self.collect_extension_candidate(module, symbol, key, &mut seen, &mut candidates)?;
        }

        Ok(candidates)
    }

    /// Collect one extension candidate by declaration symbol.
    fn collect_extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let candidate = self.extension_candidate(module, symbol, key)?;
        let Some(candidate) = candidate else {
            return Ok(());
        };
        if seen.insert(candidate.symbol) {
            candidates.push(candidate);
        }

        Ok(())
    }

    /// Return one extension candidate by declaration symbol.
    fn extension_candidate(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Option<ExtensionCandidate>> {
        let Some(extension) = self.extension_definition(module, symbol)? else {
            return Ok(None);
        };
        let Some(member) = self.lookup_member_symbols(symbol, key).into_iter().next() else {
            return Ok(None);
        };

        Ok(Some(ExtensionCandidate {
            symbol,
            target: extension.target_type,
            where_clauses: extension.where_clauses,
            member,
        }))
    }

    /// Match an extension target pattern against an applied receiver.
    fn extension_substitution(
        &mut self,
        extension: dir::GlobalSymbolId,
        target_variable: TypeOperand,
        receiver: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<GenericSubstitution>> {
        let Some(pattern) = self.type_operand_term(target_variable)? else {
            return Ok(None);
        };
        let actual = TypeTerm::Reference {
            origin: Origin::Symbol(receiver),
            symbol: receiver,
            arguments: arguments.to_vec().into(),
        };
        let mut substitution = GenericSubstitution::empty();
        let is_match = self.match_type_pattern(
            receiver.module_id,
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
        substitution: Substitution<'_>,
    ) -> CompilerResult<bool> {
        for where_clause in where_clauses {
            let Some(left) = self.type_operand_term(where_clause.left)? else {
                return Ok(false);
            };
            let Some(right) = self.type_operand_term(where_clause.right)? else {
                return Ok(false);
            };
            let Some(left) = left.substitute(where_clause.source.module_id, substitution, self)?
            else {
                return Ok(false);
            };
            let Some(right) =
                right.substitute(where_clause.source.module_id, substitution, self)?
            else {
                return Ok(false);
            };

            let decision =
                self.decide_type_term_relation(TypeRelation::Satisfies, &left, &right)?;
            match decision {
                Decision::Yes => {}
                Decision::No => return Ok(false),
                Decision::Undecidable => {
                    todo!("resolve extension where clauses through inference constraints")
                }
            };
        }

        Ok(true)
    }
}
