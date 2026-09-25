use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;

use crate::{
    ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
    SymbolOccurrence, Target,
};

/// Target of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTarget {
    /// The rename source.
    pub target: Target,
    /// The resolved rename symbols.
    pub symbols: Vec<dir::GlobalSymbolId>,
    /// The current name.
    pub placeholder: String,
}

/// A rename target request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetRequest {
    /// The queried position.
    pub position: QueryPosition,
}

/// A rename target response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetResponse {
    /// Rename target, if available.
    pub target: Option<RenameTarget>,
}

/// One exact rename selection.
pub(super) struct RenameSelection {
    /// The authored rename occurrence.
    pub(super) occurrence: SymbolOccurrence,
    /// The declaration identities renamed together.
    pub(super) symbols: Vec<dir::GlobalSymbolId>,
    /// The current authored name.
    pub(super) placeholder: String,
    /// Whether the selection names one explicit local import alias.
    pub(super) is_local_import_alias: bool,
}

impl ModuleQueryContext<'_> {
    /// Return the rename target at the given position.
    pub fn rename_target(
        &self,
        request: RenameTargetRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<RenameTargetResponse> {
        let Some(selection) = RenameSelection::resolve(request.position, self, program)? else {
            return Ok(RenameTargetResponse { target: None });
        };
        let target = Target::new(self.module(), selection.occurrence.span);
        let target = RenameTarget {
            target,
            symbols: selection.symbols,
            placeholder: selection.placeholder,
        };

        Ok(RenameTargetResponse {
            target: Some(target),
        })
    }
}

impl RenameSelection {
    /// Resolve the exact declaration group targeted by rename.
    pub(super) fn resolve(
        position: QueryPosition,
        module: &ModuleQueryContext<'_>,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Option<Self>> {
        let Some(occurrence) = module
            .cursor(position.file_id, position.offset)?
            .reference(program)?
        else {
            return Ok(None);
        };

        // keep explicit local import aliases as local rename targets
        if let Some(symbol) = occurrence.symbol() {
            let placeholder = module.local_import_alias_name(symbol)?;
            if let Some(placeholder) = placeholder {
                return Ok(Some(Self {
                    occurrence,
                    symbols: vec![symbol],
                    placeholder,
                    is_local_import_alias: true,
                }));
            }
        }

        // collect the declaration targets selected by dependency bindings
        let mut symbols = Vec::new();
        for symbol in &occurrence.symbols {
            symbols.extend(program.symbol_targets(*symbol)?);
        }
        symbols.sort();
        symbols.dedup();
        let Some(first) = symbols.first().copied() else {
            return Ok(None);
        };

        // exclude structural keys without declaration identities
        let target_module = program.module(first.module_id)?;
        let symbol = target_module.bindings()?.get_symbol(first.local_id);
        let is_structural_member = symbol
            .declaration
            .is_some_and(|source| source.local_id.ty == dir::NodeType::TypeMember)
            && target_module.definition_member(program, first)?.is_none();
        if is_structural_member {
            return Ok(None);
        }

        // require an authored target name
        let Some(placeholder) = program.symbol_name(first)? else {
            return Ok(None);
        };

        // include declarations connected by exact overload and implementation relations
        let rename_declarations = program.rename_declarations(first)?;
        if symbols
            .iter()
            .any(|symbol| rename_declarations.binary_search(symbol).is_err())
        {
            return Ok(None);
        }
        let symbols = rename_declarations;

        // require one authored name across the rename group
        for symbol in &symbols[1..] {
            let name = program
                .symbol_name(*symbol)?
                .ok_or(QueryError::missing(format!("rename name: {symbol:?}")))?;
            if name != placeholder {
                return Err(QueryError::conflict(format!(
                    "rename names: {:?}, {:?}",
                    first, *symbol
                )));
            }
        }

        Ok(Some(Self {
            occurrence,
            symbols,
            placeholder,
            is_local_import_alias: false,
        }))
    }
}

impl ProgramQueryContext<'_> {
    /// Return declarations renamed with one symbol.
    fn rename_declarations(
        &self,
        root: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut symbols = vec![root];
        let mut selected = FxHashSet::from_iter([root]);
        let mut cursor = 0;

        // walk exact overload and member implementation relations
        while cursor < symbols.len() {
            // select the next connected declaration
            let symbol = symbols[cursor];
            cursor += 1;

            // read every exact declaration relation
            let mut connected = self.function_overloads(symbol)?;
            connected.extend(self.member_implementations(symbol)?);
            connected.extend(self.member_declarations(symbol)?);

            // queue declarations not visited yet
            for connected_symbol in connected {
                if selected.insert(connected_symbol) {
                    symbols.push(connected_symbol);
                }
            }
        }

        // normalize result order
        symbols.sort();

        Ok(symbols)
    }

    /// Return every function declaration sharing one lexical binding.
    fn function_overloads(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let module = self.module(symbol_id.module_id)?;
        let bindings = module.bindings()?;
        let symbol = bindings.get_symbol(symbol_id.local_id);
        if symbol.kind != dir::SymbolKind::Function {
            return Ok(vec![symbol_id]);
        }

        // require the lexical function key
        let Some(key) = symbol.key else {
            return Err(QueryError::missing(format!(
                "function overload key: {symbol_id:?}"
            )));
        };

        // retain every function bound by the same key in the same scope
        let mut overloads = Vec::new();
        bindings
            .get_scope(symbol.scope)
            .for_symbols_by_key(key, |overload| {
                if bindings.get_symbol(overload).kind == dir::SymbolKind::Function {
                    overloads.push(overload.into_global(symbol_id.module_id));
                }
            });

        // require a nonempty ordered overload group
        overloads.sort();
        if overloads.is_empty() {
            return Err(QueryError::missing(format!(
                "function overloads: {symbol_id:?}"
            )));
        }

        Ok(overloads)
    }
}
