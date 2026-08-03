use std::collections::{BTreeMap, btree_map};

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, Patch, PatchSet, Span};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

use crate::source::is_simple_identifier;
use crate::{
    Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
    SymbolOccurrence,
};

/// Request rename edits at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// The new name for the symbol.
    pub new_name: String,
}

/// Response payload for rename queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameResponse {
    /// Rename edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Rename the symbol at one position.
    pub fn rename(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
        new_name: &str,
    ) -> QueryResult<Option<PatchSet>> {
        if !is_simple_identifier(new_name) {
            return Ok(None);
        }

        // resolve the exact rename target
        let Some(selection) = self.resolve_rename_target(program, file_id, offset)? else {
            return Ok(None);
        };
        let symbols = selection.expand_rename_symbols(program)?;

        // reject no-op names
        if selection.placeholder == new_name {
            return Ok(None);
        }

        // collect the complete indexed occurrence set
        let occurrences = self.collect_symbol_rename_occurrences(
            program,
            &symbols,
            selection.is_local_declaration,
        )?;
        let role = RenameRole::resolve(program, &symbols)?;

        let edits = selection.edits(program, &occurrences, new_name, role)?;

        Ok(Some(edits))
    }
}

/// One exact rename selection.
pub(crate) struct RenameSelection {
    /// The authored rename occurrence.
    pub(crate) occurrence: SymbolOccurrence,
    /// The declaration identities renamed together.
    pub(crate) symbols: Vec<dir::GlobalSymbolId>,
    /// The current authored name.
    pub(crate) placeholder: String,
    /// Whether the selection names one local import binding.
    is_local_declaration: bool,
}

impl RenameSelection {
    /// Expand the selection to every declaration renamed together with it.
    fn expand_rename_symbols(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<dir::GlobalSymbolId>> {
        let mut worklist = self.symbols.clone();
        let mut expanded = Vec::new();
        let mut seen = FxHashSet::default();

        while let Some(symbol_id) = worklist.pop() {
            if !seen.insert(symbol_id) {
                continue;
            }
            expanded.push(symbol_id);
            let module = program.module(symbol_id.module_id)?;
            let symbol = module.bindings()?.get_symbol(symbol_id.local_id);
            let Some(declaration) = symbol.declaration else {
                continue;
            };

            // rename members with their same-key siblings and implementations
            if matches!(
                declaration.local_id.ty,
                dir::NodeType::Member | dir::NodeType::TypeMember
            ) {
                worklist.extend(module.member_rename_siblings(program, symbol_id)?);
            }
        }
        expanded.sort();

        Ok(expanded)
    }

    /// Build exact file edits from indexed occurrences.
    fn edits(
        &self,
        program: &ProgramQueryContext<'_>,
        occurrences: &[RenameOccurrence],
        new_name: &str,
        role: RenameRole,
    ) -> QueryResult<PatchSet> {
        let mut shorthands_by_module = BTreeMap::new();
        let mut replacements = BTreeMap::new();

        // retain occurrences whose authored name follows the declaration
        for occurrence in occurrences {
            let module = program.module(occurrence.module.module_id)?;
            let authored_name = module.source_text(occurrence.span)?;
            if authored_name != self.placeholder {
                return Err(QueryError::conflict(format!(
                    "rename occurrence text: {:?}",
                    occurrence.span
                )));
            }

            // read the exact edit form from authored shorthand structure
            let shorthands = match shorthands_by_module.entry(occurrence.module.module_id) {
                btree_map::Entry::Occupied(entry) => entry.into_mut(),
                btree_map::Entry::Vacant(entry) => {
                    let shorthands = RenameShorthands::build(&module)?;

                    entry.insert(shorthands)
                }
            };
            let replacement =
                shorthands.replacement(occurrence.span, &self.placeholder, new_name, role)?;
            let key = (
                occurrence.span.file,
                occurrence.span.start,
                occurrence.span.end,
            );
            if let Some(previous) = replacements.insert(key, replacement.clone())
                && previous != replacement
            {
                return Err(QueryError::conflict(format!(
                    "rename edit: {:?}",
                    occurrence.span
                )));
            }
        }

        // group exact replacements by source file
        let mut edits_by_file: BTreeMap<FileId, Vec<Patch>> = BTreeMap::new();
        for ((file_id, start, end), replacement) in replacements {
            let span = Span::new(file_id, start, end);
            edits_by_file
                .entry(file_id)
                .or_default()
                .push(Patch::replace(span, replacement));
        }

        let mut edits = PatchSet::new();
        for (file_id, patches) in edits_by_file {
            edits.push(FilePatch::with_patches(file_id, patches));
        }

        Ok(edits)
    }
}

/// The authored role of a renamed symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenameRole {
    /// A lexical binding used as an object or pattern value.
    Binding,
    /// A member used as an object or pattern key.
    Member,
}

impl RenameRole {
    /// Resolve the one authored role shared by a rename group.
    fn resolve(
        program: &ProgramQueryContext<'_>,
        symbols: &[dir::GlobalSymbolId],
    ) -> QueryResult<Self> {
        let Some(first) = symbols.first().copied() else {
            return Err(QueryError::missing("rename group"));
        };
        let selected = Self::symbol_role(program, first)?;

        // classify each exact declaration by its authored node role
        for symbol in &symbols[1..] {
            let role = Self::symbol_role(program, *symbol)?;
            if selected != role {
                return Err(QueryError::conflict(format!(
                    "rename roles: {:?}, {:?}",
                    first, *symbol
                )));
            }
        }

        Ok(selected)
    }

    /// Resolve the authored role of one rename symbol.
    fn symbol_role(
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Self> {
        let module = program.module(symbol_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(symbol_id.local_id);
        let declaration = symbol.declaration.ok_or(QueryError::missing(format!(
            "rename declaration: {symbol_id:?}"
        )))?;
        let role = match declaration.local_id.ty {
            dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField => {
                Self::Member
            }
            _ => Self::Binding,
        };

        Ok(role)
    }
}

/// One exact indexed rename occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenameOccurrence {
    /// The module that owns the occurrence.
    module: Module,
    /// The exact authored name span.
    span: Span,
}

/// Authored shorthand names requiring structured rename edits.
struct RenameShorthands {
    /// Shorthand object and pattern names keyed by their authored span.
    shorthand_names: FxHashMap<Span, String>,
}

impl RenameShorthands {
    /// Index authored shorthand names in one module.
    fn build(module: &ModuleQueryContext<'_>) -> QueryResult<Self> {
        let view = module.view()?;
        let mut shorthand_names = FxHashMap::default();

        // index object literal shorthand names
        for (property_id, property) in view.iter_nodes_of_type::<dir::Property>() {
            let dir::Property::Field {
                key: dir::Key::Name(name),
                is_shorthand: true,
                ..
            } = property
            else {
                continue;
            };
            let node = property_id.into_global_any(module.module_id());
            let span = module
                .node_selection_span(view, property_id.into())?
                .ok_or(QueryError::missing(format!("rename shorthand: {node:?}")))?;
            let name = module.strings().get(name.string()).to_string();
            shorthand_names.insert(span, name);
        }

        // index destructuring shorthand names
        for (field_id, field) in view.iter_nodes_of_type::<dir::PatternField>() {
            let dir::PatternField::Named {
                name,
                is_shorthand: true,
                ..
            } = field
            else {
                continue;
            };
            let node = field_id.into_global_any(module.module_id());
            let span = module
                .node_selection_span(view, field_id.into())?
                .ok_or(QueryError::missing(format!("rename shorthand: {node:?}")))?;
            let name = module.strings().get(name.string()).to_string();
            shorthand_names.insert(span, name);
        }

        Ok(Self { shorthand_names })
    }

    /// Return replacement text for one exact rename occurrence.
    fn replacement(
        &self,
        span: Span,
        old_name: &str,
        new_name: &str,
        role: RenameRole,
    ) -> QueryResult<String> {
        let Some(shorthand_name) = self.shorthand_names.get(&span) else {
            return Ok(new_name.to_string());
        };
        if shorthand_name != old_name {
            return Err(QueryError::conflict(format!("rename shorthand: {span:?}")));
        }

        let replacement = match role {
            RenameRole::Binding => format!("{old_name}: {new_name}"),
            RenameRole::Member => format!("{new_name}: {old_name}"),
        };

        Ok(replacement)
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the symbol targeted by rename at a file offset.
    pub(crate) fn resolve_rename_target(
        &self,
        query: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
    ) -> QueryResult<Option<RenameSelection>> {
        let Some(occurrence) = self.reference_at_offset(file_id, offset)? else {
            return Ok(None);
        };

        // keep explicit local import aliases as local rename targets
        if let Some(symbol) = occurrence.symbol() {
            let placeholder = self.local_import_alias_name(symbol)?;
            if let Some(placeholder) = placeholder {
                return Ok(Some(RenameSelection {
                    occurrence,
                    symbols: vec![symbol],
                    placeholder,
                    is_local_declaration: true,
                }));
            }
        }

        // resolve every canonical declaration in the selected overload set
        let mut symbols = Vec::new();
        for symbol in &occurrence.symbols {
            symbols.extend(query.canonical_symbols(*symbol)?);
        }
        symbols.sort();
        symbols.dedup();
        let Some(first) = symbols.first().copied() else {
            return Ok(None);
        };
        let Some(placeholder) = query.symbol_name(first)? else {
            return Ok(None);
        };

        // require one stable authored name across the rename group
        for symbol in &symbols[1..] {
            let Some(name) = query.symbol_name(*symbol)? else {
                return Ok(None);
            };
            if name != placeholder {
                return Err(QueryError::conflict(format!(
                    "rename names: {:?}, {:?}",
                    first, *symbol
                )));
            }
        }

        let selection = RenameSelection {
            occurrence,
            symbols,
            placeholder,
            is_local_declaration: false,
        };

        Ok(Some(selection))
    }

    /// Collect all indexed occurrences for one declaration group.
    fn collect_symbol_rename_occurrences(
        &self,
        program: &ProgramQueryContext<'_>,
        symbols: &[dir::GlobalSymbolId],
        is_local_declaration: bool,
    ) -> QueryResult<Vec<RenameOccurrence>> {
        let mut occurrences = Vec::new();

        // require every authored declaration targeted by this rename
        for symbol in symbols {
            let module = program.module(symbol.module_id)?;
            let definition_span = if is_local_declaration {
                module.symbol_local_definition_span(*symbol)?
            } else {
                program.symbol_definition_span(*symbol)?
            }
            .ok_or(QueryError::missing(format!(
                "rename definition: {:?}",
                *symbol
            )))?;
            occurrences.push(RenameOccurrence {
                module: module.module(),
                span: definition_span,
            });

            // collect references across indexed program modules
            if is_local_declaration {
                occurrences.extend(
                    program
                        .declaration_references(*symbol, None)?
                        .into_iter()
                        .map(|(module, span)| RenameOccurrence { module, span }),
                );
            } else {
                for reference in program.symbol_program_references(*symbol)? {
                    let entry = reference.entry;

                    if entry.is_import_alias {
                        continue;
                    }

                    occurrences.push(RenameOccurrence {
                        module: reference.module,
                        span: entry.span,
                    });
                }
            }
        }

        // remove exact duplicate rows from overload and profile indexes
        occurrences.sort_by_key(|occurrence| {
            (
                occurrence.module.profile_id,
                occurrence.module.module_id,
                occurrence.span.file,
                occurrence.span.start,
                occurrence.span.end,
            )
        });
        occurrences.dedup();

        Ok(occurrences)
    }
}
