use destack_dir as dir;
use destack_dir::HeritageKind;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return the declarations renamed together with one member symbol.
    pub(crate) fn member_rename_siblings(
        &self,
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut siblings = Vec::new();
        let Some((owner, definition, member)) = self.definitions()?.member(symbol_id) else {
            return Ok(siblings);
        };
        let Some(key) = member.key() else {
            return Ok(siblings);
        };

        // rename same-key declarations on the owner together
        for candidate in definition.members() {
            if candidate.key() == Some(key)
                && let Some(candidate_symbol) = candidate.symbol()
            {
                siblings.push(candidate_symbol);
            }
        }

        // include interface requirements and their implementing declarations
        let requirement = member.source();
        self.member_implementation_symbols(program, owner, requirement, &mut siblings)?;

        Ok(siblings)
    }

    /// Collect the declaration symbols implementing one interface member.
    pub(crate) fn member_implementation_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
        owner: dir::GlobalSymbolId,
        requirement: dir::GlobalNodeIdAny,
        symbols: &mut Vec<dir::GlobalSymbolId>,
    ) -> QueryResult<()> {
        // resolve the requirement to its declaring member
        let home = program.module(owner.module_id)?;
        let Some(definition) = home.definitions()?.definition(owner) else {
            return Ok(());
        };
        let Some(required) = definition
            .members()
            .iter()
            .find(|member| member.source() == requirement)
        else {
            return Ok(());
        };

        // read the required member's space and key
        let space = required.space();
        let Some(key) = required.key() else {
            return Ok(());
        };

        // walk implementers reached through the owner's heritage rows
        for entry in program.base_heritage(owner)? {
            if entry.kind != HeritageKind::Implements {
                continue;
            }

            // search the conformance declaration before the conforming root
            let mut providers = vec![entry.declaration];
            if entry.derived != entry.declaration {
                providers.push(entry.derived);
            }
            for provider in providers {
                let derived_module = program.module(provider.module_id)?;
                let Some(definition) = derived_module.definitions()?.definition(provider) else {
                    continue;
                };

                // collect the implementer's declarations under the same member key
                let mut found = false;
                for member in definition.members_with_key(space, key) {
                    if let Some(symbol) = member.symbol() {
                        symbols.push(symbol);
                        found = true;
                    }
                }
                if found {
                    break;
                }
            }
        }

        Ok(())
    }
}
