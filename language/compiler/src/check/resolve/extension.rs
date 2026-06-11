use destack_dir as dir;
use destack_source::ModuleId;
use indexmap::IndexSet;

use crate::CompilerResult;
use crate::check::{
    Answer, CheckState, Definition, ExtensionTarget, ExtensionWhereClause, GenericArgument,
    MemberKey, MemberLookup, MemberSpace, Origin, SubstitutionSet, TypeOperand, TypeRelation,
    TypeTerm,
};

impl CheckState<'_> {
    /// Resolve extension members for one applied nominal target.
    pub(in crate::check) fn resolve_extension_member(
        &mut self,
        origin: Origin,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        receiver: TypeOperand,
        key: MemberKey,
    ) -> CompilerResult<MemberLookup> {
        let symbols = self.extension_member_symbols(module, target, key);
        let mut candidates = Vec::new();
        let mut seen = IndexSet::new();

        // visit extension declarations in resolution order
        for symbol in symbols {
            let Some(Definition::Extension(extension)) = self.definitions.definition(symbol) else {
                continue;
            };
            let extension_target = extension.target;
            let where_clauses = extension.where_clauses.to_vec();
            let mut substitution = SubstitutionSet::empty();

            // match target and where clauses before reading members
            match self.match_extension_target(
                origin,
                symbol,
                &extension_target,
                target,
                arguments,
                key,
                &mut substitution,
            )? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }
            match self.decide_extension_where_clauses(&where_clauses, &substitution)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => continue,
                Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
            }

            // resolve matching extension members
            let instance = self.generic_instance_from_substitution(symbol, &substitution)?;
            let members = self.definitions.members(symbol, key).to_vec();
            for member in members {
                if member.symbol().is_some_and(|symbol| !seen.insert(symbol)) {
                    continue;
                }
                let candidate = self.resolve_member_candidate(
                    origin,
                    module,
                    receiver,
                    member,
                    &substitution,
                    instance.clone(),
                )?;
                match candidate {
                    Answer::Ready(Some(candidate)) => candidates.push(candidate),
                    Answer::Ready(None) => {}
                    Answer::Pending(blockers) => return Ok(MemberLookup::Pending(blockers)),
                }
            }
        }

        Ok(MemberLookup::from_candidates(candidates))
    }

    /// Return extension symbols that can contribute one member key.
    fn extension_member_symbols(
        &self,
        module: ModuleId,
        target: dir::GlobalSymbolId,
        key: MemberKey,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut symbols = Vec::new();

        // collect local extensions beside the reference module
        symbols.extend(self.definitions.extension_symbols(module, target, key));
        symbols.extend(self.definitions.blanket_extension_symbols(module, key));

        // collect inherent extensions beside external receivers
        if target.module_id != module {
            symbols.extend(
                self.definitions
                    .extension_symbols(target.module_id, target, key),
            );
        }

        // collect explicitly imported extensions
        for (_, symbol) in self.module(module).resolved.imports.symbol_targets() {
            if !self.definitions.members(symbol, key).is_empty() {
                symbols.push(symbol);
            }
        }

        symbols
    }

    /// Match an extension target against a receiver declaration or type.
    fn match_extension_target(
        &mut self,
        origin: Origin,
        extension: dir::GlobalSymbolId,
        target: &ExtensionTarget,
        receiver: dir::GlobalSymbolId,
        arguments: &[GenericArgument],
        key: MemberKey,
        substitution: &mut SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        // static declaration lookup only proves the nominal root
        if key.space == MemberSpace::Static && arguments.is_empty() {
            let is_match = target.nominal_root() == Some(receiver);

            return Ok(Answer::Ready(is_match));
        }

        let target = target.r#type();
        let Some(pattern) = self.type_operand_term_id(target)? else {
            return Ok(Answer::pending(target.dependencies(self)));
        };
        let actual = self.type_term_operand(TypeTerm::Reference {
            origin: Origin::Symbol(receiver),
            symbol: receiver,
            arguments: arguments.to_vec().into(),
        });
        let Some(actual) = self.type_operand_term_id(actual)? else {
            return Ok(Answer::pending(actual.dependencies(self)));
        };

        let is_match = self.match_type_pattern(
            origin,
            receiver.module_id,
            extension,
            pattern,
            actual,
            substitution,
        )?;

        Ok(is_match)
    }

    /// Decide whether extension where clauses hold under substitution.
    fn decide_extension_where_clauses(
        &mut self,
        where_clauses: &[ExtensionWhereClause],
        substitution: &SubstitutionSet,
    ) -> CompilerResult<Answer<bool>> {
        for where_clause in where_clauses {
            let left = self.substitute_type_operand(
                where_clause.source.module_id,
                substitution,
                where_clause.left,
            )?;
            let right = self.substitute_type_operand(
                where_clause.source.module_id,
                substitution,
                where_clause.right,
            )?;

            // require the substituted where clause relation
            match self.decide_type_relation(TypeRelation::Satisfies, left, right)? {
                Answer::Ready(true) => {}
                Answer::Ready(false) => return Ok(Answer::Ready(false)),
                Answer::Pending(blockers) => return Ok(Answer::Pending(blockers)),
            }
        }

        Ok(Answer::Ready(true))
    }
}
