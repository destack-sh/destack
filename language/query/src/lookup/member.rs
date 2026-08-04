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
        // walk implementers reached through the owner's heritage rows
        for entry in program.base_heritage(owner)? {
            if entry.kind != HeritageKind::Implements {
                continue;
            }
            let derived_module = program.module(entry.declaration.module_id)?;
            let Some(definition) = derived_module.definitions()?.definition(entry.declaration)
            else {
                continue;
            };

            // read the stored member mapping of the matching implementation
            for implementation in definition.implementations() {
                if implementation.interface.ty != entry.ty {
                    continue;
                }
                let Some(implemented) = implementation.member(requirement) else {
                    continue;
                };
                for declaration in &implemented.declarations {
                    let declaration_module = program.module(declaration.module_id)?;
                    if let Some(symbol) = declaration_module
                        .bindings()?
                        .declaration_symbol(*declaration)
                    {
                        symbols.push(symbol.into_global(declaration.module_id));
                    }
                }
            }
        }

        Ok(())
    }
}
