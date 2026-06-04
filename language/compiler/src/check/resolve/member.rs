use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

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

/// Result of looking up one member on a receiver type.
pub(in crate::check) enum MemberLookup {
    /// Member lookup is waiting for solver input.
    Pending,
    /// No symbol-backed member exists.
    Missing,
    /// One or more symbol-backed members exist.
    Found(Vec<MemberCandidate>),
}

impl MemberLookup {
    /// Return a member lookup with one found candidate.
    pub(in crate::check) fn found(candidate: MemberCandidate) -> Self {
        Self::Found(vec![candidate])
    }

    /// Return a member lookup from collected candidates.
    pub(in crate::check) fn from_candidates(candidates: Vec<MemberCandidate>) -> Self {
        if candidates.is_empty() {
            Self::Missing
        } else {
            Self::Found(candidates)
        }
    }
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
    /// Look up symbol-backed members from one type term.
    pub(in crate::check) fn lookup_type_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = match reduction.value.as_ref() {
            Some(reduced) => reduced,
            None => return Ok(MemberLookup::Pending),
        };

        self.lookup_type_member_matching(origin, module, term, key)
    }

    /// Look up symbol-backed members that satisfy one protocol.
    pub(in crate::check) fn lookup_protocol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
        protocol: &MemberProtocol,
    ) -> CompilerResult<MemberLookup> {
        let reduction = self.reduce_type_term(origin, term)?;
        let term = match reduction.value.as_ref() {
            Some(reduced) => reduced,
            None => return Ok(MemberLookup::Pending),
        };

        match self.decide_member_protocol(module, term, protocol)? {
            Decision::Yes => self.lookup_type_member_matching(origin, module, term, key),
            Decision::Undecidable => Ok(MemberLookup::Pending),
            Decision::No => Ok(MemberLookup::Missing),
        }
    }

    /// Look up symbol-backed members from one reduced type term.
    pub(in crate::check) fn lookup_type_member_matching(
        &mut self,
        origin: Origin,
        module: ModuleId,
        term: &TypeTerm,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        match term {
            TypeTerm::Form { payload, .. } => {
                let Some(term) = self.type_operand_term(*payload)? else {
                    return Ok(MemberLookup::Pending);
                };

                self.lookup_type_member_matching(origin, module, &term, key)
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } if arguments.is_empty() => {
                if let Some(parameter) = self.type_generic_parameter_for_symbol(*symbol)? {
                    self.lookup_parameter_member(module, origin, parameter, key)
                } else {
                    self.lookup_symbol_member(origin, module, *symbol, arguments, *key)
                }
            }
            TypeTerm::Reference {
                origin: _,
                symbol,
                arguments,
            } => self.lookup_symbol_member(origin, module, *symbol, arguments, *key),
            TypeTerm::Parameter(parameter) => {
                self.lookup_parameter_member(module, origin, *parameter, key)
            }
            TypeTerm::Shape(shape) => {
                self.lookup_shape_member(&self.inference.term(*shape).members, key)
            }
            TypeTerm::Union { elements } => {
                self.union_member_candidates(origin, module, elements, key)
            }
            TypeTerm::Operation(operation) => {
                self.operation_member_candidates(origin, module, *operation, key)
            }
            _ => Ok(MemberLookup::Missing),
        }
    }

    /// Look up members through a generic parameter constraint.
    fn lookup_parameter_member(
        &mut self,
        module: ModuleId,
        origin: Origin,
        parameter: GenericParameterId,
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let Some(constraint) = self.type_generic_constraint(parameter)? else {
            return Ok(MemberLookup::Missing);
        };
        let Some(term) = self.reduced_generic_type_constraint(origin, constraint)? else {
            return Ok(MemberLookup::Pending);
        };

        self.lookup_type_member_matching(origin, module, &term, key)
    }

    /// Look up a symbol-backed member from one shape field.
    fn lookup_shape_member(
        &self,
        members: &[ShapeMember],
        key: &dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
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
                return Ok(MemberLookup::Pending);
            };
            let TypeOperand::Variable(variable) = *ty else {
                return Ok(MemberLookup::Missing);
            };
            let Origin::Symbol(symbol) = self.variable(variable).source else {
                return Ok(MemberLookup::Missing);
            };

            let candidate = MemberCandidate {
                symbol,
                ty: term,
                instance: None,
            };

            return Ok(MemberLookup::found(candidate));
        }

        Ok(MemberLookup::Missing)
    }

    /// Look up an applied symbol member.
    fn lookup_symbol_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let mut candidates = Vec::new();

        // collect inherent members
        for member in self.member_symbols(symbol, key) {
            let substitution = self.generic_substitution(symbol, arguments)?;
            if self.reduce_symbol_availability(module, member, (&substitution).into())?
                != Decision::Yes
            {
                continue;
            }
            let operand = self.import_symbol_type_operand(module, member)?;
            if substitution.is_empty() {
                let Some(term) = self.type_operand_term(operand)? else {
                    return Ok(MemberLookup::Pending);
                };

                candidates.push(MemberCandidate {
                    symbol: member,
                    ty: term,
                    instance: None,
                });
                continue;
            }
            let Some(term) = self.type_operand_term(operand)? else {
                return Ok(MemberLookup::Pending);
            };
            let Some(term) = term.substitute(module, &substitution, self)? else {
                return Ok(MemberLookup::Pending);
            };
            let instance = Some(GenericInstance {
                owner: symbol,
                arguments: arguments.to_vec().into(),
            });

            candidates.push(MemberCandidate {
                symbol: member,
                ty: term,
                instance,
            });
        }

        if !candidates.is_empty() {
            return Ok(MemberLookup::Found(candidates));
        }

        for (extension, member, substitution) in
            self.lookup_extension_members(origin, module, symbol, arguments, key)?
        {
            if self.reduce_symbol_availability(module, member, (&substitution).into())?
                != Decision::Yes
            {
                continue;
            }
            let variable = self.import_symbol_type_operand(module, member)?;
            let Some(mut term) = self.type_operand_term(variable)? else {
                return Ok(MemberLookup::Pending);
            };
            let instance = if substitution.is_empty() {
                None
            } else {
                let Some(substituted) = term.substitute(module, &substitution, self)? else {
                    return Ok(MemberLookup::Pending);
                };

                term = substituted;
                self.substitution_application(extension, &substitution)?
            };

            candidates.push(MemberCandidate {
                symbol: member,
                ty: term,
                instance,
            });
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Decide whether one type satisfies one member protocol.
    fn decide_member_protocol(
        &mut self,
        module: ModuleId,
        receiver: &TypeTerm,
        protocol: &MemberProtocol,
    ) -> CompilerResult<Decision> {
        let symbol = self.language_symbol(module, protocol.item);
        let target = TypeTerm::Reference {
            origin: Origin::Symbol(symbol),
            symbol,
            arguments: protocol.arguments.to_vec(),
        };

        self.decide_type_term_relation(TypeRelation::Satisfies, receiver, &target)
    }

    /// Look up applicable extension members for one applied nominal receiver.
    pub(in crate::check) fn lookup_extension_members(
        &mut self,
        origin: Origin,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<
        Vec<(
            dir::GlobalSymbolId,
            dir::GlobalSymbolId,
            GenericSubstitution,
        )>,
    > {
        let extensions = self.extension_member_candidates(module, target, key)?;
        let mut members = Vec::new();

        // collect extensions whose target pattern and constraints hold
        for extension in extensions {
            let Some(substitution) = self.match_extension_target(
                origin,
                extension.symbol,
                extension.target,
                target,
                arguments,
            )?
            else {
                continue;
            };

            if self
                .decide_extension_where_clauses(&extension.where_clauses, (&substitution).into())?
            {
                members.push((extension.symbol, extension.member, substitution));
            }
        }

        Ok(members)
    }

    /// Return extension member candidates available from one module.
    fn extension_member_candidates(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<ExtensionCandidate>> {
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        let symbols = self.import_extension_target_symbols(module, target)?;
        for symbol in symbols {
            self.collect_extension_members(module, symbol, key, &mut seen, &mut candidates)?;
        }

        // add inherent extensions declared with the receiver type
        if target.module_id != module {
            let symbols = self.import_extension_target_symbols(target.module_id, target)?;
            for symbol in symbols {
                self.collect_extension_members(module, symbol, key, &mut seen, &mut candidates)?;
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
            self.collect_extension_members(module, symbol, key, &mut seen, &mut candidates)?;
        }

        Ok(candidates)
    }

    /// Collect extension member candidates by declaration symbol.
    fn collect_extension_members(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
        seen: &mut IndexSet<dir::GlobalSymbolId>,
        candidates: &mut Vec<ExtensionCandidate>,
    ) -> CompilerResult<()> {
        let Some(extension) = self.extension_definition(module, symbol)? else {
            return Ok(());
        };

        // collect every member overload declared on this extension
        for member in self.member_symbols(symbol, key) {
            if seen.insert(member) {
                candidates.push(ExtensionCandidate {
                    symbol,
                    target: extension.target_type,
                    where_clauses: extension.where_clauses.clone(),
                    member,
                });
            }
        }

        Ok(())
    }

    /// Match an extension target pattern against an applied receiver.
    fn match_extension_target(
        &mut self,
        origin: Origin,
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
            origin,
            receiver.module_id,
            extension,
            &pattern,
            &actual,
            &mut substitution,
        )?;

        Ok(is_match.then_some(substitution))
    }

    /// Decide whether substituted extension where clauses hold.
    fn decide_extension_where_clauses(
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
                    return Ok(false);
                }
            };
        }

        Ok(true)
    }
}
