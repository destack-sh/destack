use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    CheckState, Decision, ExtensionDefinition, ExtensionWhereClause, GenericArgument, MemberLookup,
    Origin, SubstitutionSet, TypeMemberDefinition, TypeOperand, TypeRelation, TypeTerm,
};

/// Extension member candidate before receiver matching.
#[derive(Debug, Clone, PartialEq)]
struct ExtensionMemberCandidate {
    /// The extension declaration symbol.
    symbol: dir::GlobalSymbolId,
    /// The extension target type.
    target: TypeOperand,
    /// The extension where clauses.
    where_clauses: Vec<ExtensionWhereClause>,
    /// The resolved member.
    member: TypeMemberDefinition,
}

/// Extension symbol lookup requested by member resolution.
enum ExtensionSymbolQuery {
    /// Inherent extensions declared for one nominal receiver.
    Target(dir::GlobalSymbolId),
    /// Blanket extensions declared for any matching receiver.
    Blanket,
}

/// Extension member candidates gathered for one lookup key.
struct ExtensionMemberCandidates {
    /// Member symbols already gathered.
    seen: IndexSet<dir::GlobalSymbolId>,
    /// Extension members in lookup order.
    members: Vec<ExtensionMemberCandidate>,
}

impl ExtensionMemberCandidates {
    /// Create an empty extension member candidate set.
    fn new() -> Self {
        Self {
            seen: IndexSet::new(),
            members: Vec::new(),
        }
    }

    /// Add members from one extension definition.
    fn extend(
        &mut self,
        symbol: dir::GlobalSymbolId,
        extension: ExtensionDefinition,
        key: dir::StaticKey,
    ) {
        let target = extension.target.r#type();
        let members = extension.type_members(&key);
        let where_clauses = extension.where_clauses;

        // collect every member overload declared on this extension
        for member in members {
            if member
                .symbol
                .is_some_and(|member| !self.seen.insert(member))
            {
                continue;
            }

            self.members.push(ExtensionMemberCandidate {
                symbol,
                target,
                where_clauses: where_clauses.clone(),
                member,
            });
        }
    }

    /// Return gathered extension member candidates.
    fn into_vec(self) -> Vec<ExtensionMemberCandidate> {
        self.members
    }
}

impl CheckState<'_> {
    /// Return applicable extension members for one applied nominal receiver.
    pub(in crate::check) fn extension_member_lookup(
        &mut self,
        origin: Origin,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: dir::StaticKey,
    ) -> CompilerResult<MemberLookup> {
        let extensions = self.extension_member_declarations(module, target, key)?;
        let mut candidates = Vec::new();

        // match extension targets against the receiver
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

            if !self.decide_extension_where_clauses(&extension.where_clauses, &substitution)? {
                continue;
            }

            let instance = self.substitution_application(extension.symbol, &substitution)?;
            match self.collect_member_candidate(
                module,
                extension.member,
                &substitution,
                instance,
                &mut candidates,
            )? {
                Decision::Yes | Decision::No => {}
                Decision::Undecidable => return Ok(MemberLookup::Pending),
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return extension member declarations available from one module.
    fn extension_member_declarations(
        &mut self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: dir::StaticKey,
    ) -> CompilerResult<Vec<ExtensionMemberCandidate>> {
        let mut candidates = ExtensionMemberCandidates::new();

        // collect target extensions visible in the receiver module
        for symbol in self.extension_symbols(module, ExtensionSymbolQuery::Target(target))? {
            self.extend_extension_member_declarations(module, symbol, key, &mut candidates)?;
        }

        // collect blanket extensions visible in the receiver module
        for symbol in self.extension_symbols(module, ExtensionSymbolQuery::Blanket)? {
            self.extend_extension_member_declarations(module, symbol, key, &mut candidates)?;
        }

        // collect inherent extensions declared beside dependency receivers
        if target.module_id != module {
            for symbol in
                self.extension_symbols(target.module_id, ExtensionSymbolQuery::Target(target))?
            {
                self.extend_extension_member_declarations(module, symbol, key, &mut candidates)?;
            }
        }

        // collect explicitly imported extension declarations
        if self.is_component_module(module) {
            let imports = self
                .module(module)
                .resolved
                .imports
                .symbol_targets()
                .map(|(_, target)| target)
                .collect::<Vec<_>>();

            for symbol in imports {
                self.extend_extension_member_declarations(module, symbol, key, &mut candidates)?;
            }
        }

        Ok(candidates.into_vec())
    }

    /// Add extension member declarations by extension symbol.
    fn extend_extension_member_declarations(
        &mut self,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        key: dir::StaticKey,
        candidates: &mut ExtensionMemberCandidates,
    ) -> CompilerResult<()> {
        let Some(extension) = self.extension_definition(module, symbol)? else {
            return Ok(());
        };

        candidates.extend(symbol, extension, key);

        Ok(())
    }

    /// Return inherent extension symbols declared in one module.
    fn extension_symbols(
        &mut self,
        declaration_module: ModuleId,
        query: ExtensionSymbolQuery,
    ) -> CompilerResult<Vec<dir::GlobalSymbolId>> {
        if self.is_component_module(declaration_module) {
            let symbols = match query {
                ExtensionSymbolQuery::Target(target) => self
                    .definitions
                    .extension_symbols_for_target(declaration_module, target),
                ExtensionSymbolQuery::Blanket => self
                    .definitions
                    .blanket_extension_symbols(declaration_module),
            };

            return Ok(symbols);
        }

        let dependency = self.dependency(declaration_module);
        let mut symbols = Vec::new();

        // collect dependency extension declarations
        match query {
            ExtensionSymbolQuery::Target(target) => {
                for symbol in dependency.definitions.target_extensions(target) {
                    let Some(extension) = dependency.definitions.extension_definition(symbol)
                    else {
                        continue;
                    };

                    if extension.is_inherent() {
                        symbols.push(extension.symbol);
                    }
                }
            }
            ExtensionSymbolQuery::Blanket => {
                for symbol in dependency.definitions.blanket_extensions() {
                    let Some(extension) = dependency.definitions.extension_definition(symbol)
                    else {
                        continue;
                    };

                    if extension.is_inherent() {
                        symbols.push(extension.symbol);
                    }
                }
            }
        }

        Ok(symbols)
    }

    /// Match an extension target pattern against an applied receiver.
    fn match_extension_target(
        &mut self,
        origin: Origin,
        extension: dir::GlobalSymbolId,
        target_variable: TypeOperand,
        receiver: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
    ) -> CompilerResult<Option<SubstitutionSet>> {
        let Some(pattern) = self.type_operand_term(target_variable)? else {
            return Ok(None);
        };
        let actual = TypeTerm::Reference {
            origin: Origin::Symbol(receiver),
            symbol: receiver,
            arguments: arguments.to_vec().into(),
        };
        let mut substitution = SubstitutionSet::empty();
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
        substitution: &SubstitutionSet,
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

            // require the substituted where clause relation
            match self.decide_type_term_relation(TypeRelation::Satisfies, &left, &right)? {
                Decision::Yes => {}
                Decision::No | Decision::Undecidable => return Ok(false),
            }
        }

        Ok(true)
    }
}
