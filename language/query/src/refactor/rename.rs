use std::collections::{BTreeMap, btree_map};

use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FileId, FilePatch, Patch, PatchSet, Span};

use super::rename_target::RenameSelection;
use crate::source::is_simple_identifier;
use crate::{
    Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
};

/// A rename request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// The new name for the symbol.
    pub new_name: String,
}

/// A rename response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameResponse {
    /// Rename edit, if available.
    pub edit: Option<PatchSet>,
}

impl ModuleQueryContext<'_> {
    /// Rename the symbol at one position.
    pub fn rename(
        &self,
        request: RenameRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<RenameResponse> {
        let position = request.position;
        let new_name = request.new_name;
        if !is_simple_identifier(&new_name) {
            return Ok(RenameResponse { edit: None });
        }

        // resolve the exact rename target
        let Some(selection) = RenameSelection::resolve(position, self, program)? else {
            return Ok(RenameResponse { edit: None });
        };

        // reject no-op names
        if selection.placeholder == new_name {
            return Ok(RenameResponse { edit: None });
        }

        // require every shared occurrence to name only this rename group
        if !selection.is_unambiguous(program)? {
            return Ok(RenameResponse { edit: None });
        }

        // collect the complete indexed occurrence set
        let occurrences = selection.collect_occurrences(program)?;
        let role = RenameRole::resolve(program, &selection.symbols)?;

        let edit = selection.build_edit(program, &occurrences, &new_name, role)?;

        Ok(RenameResponse { edit: Some(edit) })
    }
}

impl RenameSelection {
    /// Return whether every indexed occurrence selects only this rename group.
    fn is_unambiguous(&self, program: &ProgramQueryContext<'_>) -> QueryResult<bool> {
        let mut occurrences_by_module = FxHashMap::default();

        // collect exact indexed occurrences of every selected declaration
        for symbol in &self.symbols {
            for reference in program.symbol_program_references(*symbol)? {
                occurrences_by_module
                    .entry(reference.module.module_id)
                    .or_insert_with(FxHashSet::default)
                    .insert((reference.entry.source, reference.entry.span));
            }
        }

        // require the complete indexed target group at every occurrence
        for (module_id, occurrences) in occurrences_by_module {
            let references = program.reference_index(module_id)?;
            for reference in references.target_references() {
                let occurrence = (reference.source, reference.span);
                if !occurrences.contains(&occurrence) {
                    continue;
                }

                // compare declarations selected by the indexed reference
                for target in program.symbol_targets(reference.symbol)? {
                    if self.symbols.binary_search(&target).is_err() {
                        return Ok(false);
                    }
                }
            }
        }

        Ok(true)
    }

    /// Collect all indexed occurrences for one declaration group.
    fn collect_occurrences(
        &self,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<Vec<RenameOccurrence>> {
        let mut occurrences = Vec::new();

        // require every authored declaration targeted by this rename
        for symbol in &self.symbols {
            let module = program.module(symbol.module_id)?;
            let definition_span = if self.is_local_import_alias {
                module.symbol_local_definition_span(program, *symbol)?
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
            if self.is_local_import_alias {
                occurrences.extend(
                    program
                        .declaration_references(*symbol)?
                        .into_iter()
                        .map(|(module, span)| RenameOccurrence { module, span }),
                );
            } else {
                for reference in program.symbol_program_references(*symbol)? {
                    let entry = reference.entry;

                    if entry.is_alias {
                        continue;
                    }

                    occurrences.push(RenameOccurrence {
                        module: reference.module,
                        span: entry.span,
                    });
                }
            }
        }

        // remove exact duplicate entries from overload and profile indexes
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

    /// Build exact file edits from indexed occurrences.
    fn build_edit(
        &self,
        program: &ProgramQueryContext<'_>,
        occurrences: &[RenameOccurrence],
        new_name: &str,
        role: RenameRole,
    ) -> QueryResult<PatchSet> {
        let mut shorthand_indexes = BTreeMap::new();
        let mut replacements = BTreeMap::new();

        // require every occurrence to retain the selected name
        for occurrence in occurrences {
            let module = program.module(occurrence.module.module_id)?;
            let authored_name = module.source_text(occurrence.span)?;
            if authored_name != self.placeholder {
                return Err(QueryError::conflict(format!(
                    "rename occurrence text: {:?}, expected={:?}, found={authored_name:?}",
                    occurrence.span, self.placeholder
                )));
            }

            // read the exact edit form from authored shorthand structure
            let shorthand_index = match shorthand_indexes.entry(occurrence.module.module_id) {
                btree_map::Entry::Occupied(entry) => entry.into_mut(),
                btree_map::Entry::Vacant(entry) => {
                    let shorthand_index = RenameShorthandIndex::build(&module)?;

                    entry.insert(shorthand_index)
                }
            };
            let replacement =
                shorthand_index.replacement(occurrence.span, &self.placeholder, new_name, role)?;
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
        let selected = Self::classify_symbol(program, first)?;

        // classify each exact declaration by its authored node role
        for symbol in &symbols[1..] {
            let role = Self::classify_symbol(program, *symbol)?;
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
    fn classify_symbol(
        program: &ProgramQueryContext<'_>,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Self> {
        let module = program.module(symbol_id.module_id)?;
        let symbol = module.bindings()?.get_symbol(symbol_id.local_id);
        let role = match symbol.declaration {
            Some(declaration) => match declaration.local_id.ty {
                dir::NodeType::Member | dir::NodeType::TypeMember | dir::NodeType::EnumField => {
                    Self::Member
                }
                _ => Self::Binding,
            },
            None if module.definition_member(program, symbol_id)?.is_some() => Self::Member,
            None => {
                return Err(QueryError::missing(format!(
                    "rename declaration: {symbol_id:?}"
                )));
            }
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
struct RenameShorthandIndex {
    /// Shorthand object and pattern names keyed by their authored span.
    shorthand_names: FxHashMap<Span, String>,
}

impl RenameShorthandIndex {
    /// Index authored shorthand names in one module.
    fn build(module: &ModuleQueryContext<'_>) -> QueryResult<Self> {
        let view = module.view()?;
        let mut shorthand_names = FxHashMap::default();

        // index object literal shorthand names
        for (property_id, property) in view.iter_nodes::<dir::Property>() {
            let dir::Property::Field {
                name: dir::Name::Identifier(name),
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
            let name = module.strings().get(*name).to_string();
            shorthand_names.insert(span, name);
        }

        // index destructuring shorthand names
        for (field_id, field) in view.iter_nodes::<dir::PatternField>() {
            let dir::PatternField::Named {
                name: dir::Name::Identifier(name),
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
            let name = module.strings().get(*name).to_string();
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
