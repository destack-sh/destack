use std::collections::HashMap;

use destack_core::StringPool;
use destack_dir as dir;
use destack_dir::HeritageKind;
use destack_serde::Reflect;
use destack_source::{FileId, FilePatch, ModuleId, Patch, PatchSet, Span};
use serde::{Deserialize, Serialize};

use crate::source::{is_simple_identifier, sort_and_dedup_spans};
use crate::{
    MemberKeyName, ModuleQueryContext, Position, ProgramQueryContext, ReferenceFilter, SymbolHit,
    Target, declaration_display_name,
};

/// Target of a rename query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTarget {
    /// The semantic rename target.
    pub target: Target,
    /// The range of the symbol to rename.
    pub range: Span,
    /// The current name (placeholder for rename dialog).
    pub placeholder: String,
}

/// Request the rename target at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetRequest {
    /// The queried position.
    pub position: Position,
}

/// Response payload for rename target queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameTargetResponse {
    /// Rename target, if available.
    pub result: Option<RenameTarget>,
}

/// Request rename edits at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct RenameRequest {
    /// The queried position.
    pub position: Position,
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
    /// Return the rename target at the given position.
    pub fn rename_target(&self, offset: u32) -> Option<RenameTarget> {
        // resolve the rename target at the cursor
        let (symbol_at, _, name) = self.resolve_rename_target(offset)?;

        // return the range and current name
        let target = Target::new(self.module(), symbol_at.span).with_symbol_id(symbol_at.symbol_id);

        Some(RenameTarget {
            target,
            range: symbol_at.span,
            placeholder: name,
        })
    }
}

/// Rename span collection grouped by file.
struct RenameSpans {
    /// The collected spans by file.
    by_file: HashMap<FileId, Vec<Span>>,
}

impl RenameSpans {
    /// Create an empty rename span collection.
    fn new() -> Self {
        Self {
            by_file: HashMap::new(),
        }
    }

    /// Add spans to the collection.
    fn add(&mut self, spans: Vec<Span>) {
        for span in spans {
            self.by_file.entry(span.file).or_default().push(span);
        }
    }

    /// Normalize span ordering and overlap handling.
    fn normalize(&mut self) {
        for spans in self.by_file.values_mut() {
            sort_and_dedup_spans(spans);
            Self::prune_overlaps(spans);
        }
    }

    /// Convert this span collection into a patch set.
    fn into_patch_set(self, new_name: &str) -> PatchSet {
        let mut batch_edit = PatchSet::new();
        for (file_id, spans) in self.by_file {
            let edits = spans
                .into_iter()
                .map(|span| Patch::replace(span, new_name.to_string()))
                .collect();
            batch_edit.push(FilePatch::with_patches(file_id, edits));
        }

        batch_edit
    }

    /// Remove overlapping spans by keeping the most specific span at each overlap.
    fn prune_overlaps(spans: &mut Vec<Span>) {
        if spans.len() < 2 {
            return;
        }

        let mut filtered = Vec::with_capacity(spans.len());
        for span in spans.iter().copied() {
            let Some(last_span) = filtered.last_mut() else {
                filtered.push(span);
                continue;
            };

            if !last_span.intersects(span) {
                filtered.push(span);
                continue;
            }

            if span.len() < last_span.len()
                || (span.len() == last_span.len() && span.start >= last_span.start)
            {
                *last_span = span;
            }
        }

        *spans = filtered;
    }
}

/// Interface member kind used for implementation matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterfaceMemberKind {
    /// A callable member declaration.
    Method,
    /// A field-like member declaration.
    Field,
}

/// Interface member information used for implementation propagation.
#[derive(Debug, Clone)]
struct InterfaceMemberTarget {
    /// The canonical interface symbol.
    interface_symbol: dir::GlobalSymbolId,
    /// The member name to propagate.
    member_name: String,
    /// The member shape to match.
    member_kind: InterfaceMemberKind,
}

/// Interface member declaration shape used for rename propagation.
struct InterfaceMember<'a> {
    /// The member kind.
    kind: InterfaceMemberKind,
    /// The member name key.
    key: &'a dir::Key,
}

impl<'a> InterfaceMember<'a> {
    /// Read one interface-member declaration shape.
    fn from_member(member: &'a dir::Member) -> Option<Self> {
        match member {
            dir::Member::Method { key, .. } => Some(Self {
                kind: InterfaceMemberKind::Method,
                key: key.as_ref()?,
            }),
            dir::Member::Field { key, .. } => Some(Self {
                kind: InterfaceMemberKind::Field,
                key,
            }),
            _ => None,
        }
    }
}

/// Rename behavior for keywords.
trait RenameKeyword {
    /// Return whether this modifier keyword can target the declaration for rename.
    fn is_rename_target_modifier(self) -> bool;
}

impl RenameKeyword for dir::Keyword {
    fn is_rename_target_modifier(self) -> bool {
        matches!(
            self,
            dir::Keyword::Export
                | dir::Keyword::Declare
                | dir::Keyword::Abstract
                | dir::Keyword::Async
                | dir::Keyword::Static
                | dir::Keyword::Public
                | dir::Keyword::Protected
                | dir::Keyword::Private
                | dir::Keyword::Readonly
                | dir::Keyword::Final
                | dir::Keyword::Accessor
                | dir::Keyword::Default
                | dir::Keyword::Override
        )
    }
}

impl ModuleQueryContext<'_> {
    /// Rename the symbol at the given position.
    ///
    /// Returns edits for all files that need to be modified.
    pub fn rename(
        &self,
        program: &ProgramQueryContext<'_>,
        offset: u32,
        new_name: &str,
    ) -> Option<PatchSet> {
        // validate new_name is a valid identifier
        if !is_simple_identifier(new_name) {
            return None;
        }

        // resolve the rename target at the cursor
        let (_, canonical_id, old_name) = self.resolve_rename_target(offset)?;
        let interface_member_target = self.resolve_interface_member_target(canonical_id);
        let preserve_local_definition = self.local_import_alias_name(canonical_id).is_some();

        // collect primary symbol spans and group by file
        let mut rename_spans = RenameSpans::new();
        let primary_spans = self.collect_symbol_rename_spans(
            program,
            canonical_id,
            &old_name,
            preserve_local_definition,
        );
        rename_spans.add(primary_spans);

        // include default import aliases that bind this export in other modules
        let default_import_alias_symbols = self.default_import_alias_symbols(program, canonical_id);
        for alias_symbol in default_import_alias_symbols {
            let Some(alias_name) = self.local_import_alias_name(alias_symbol) else {
                continue;
            };

            let alias_spans =
                self.collect_symbol_rename_spans(program, alias_symbol, &alias_name, true);
            rename_spans.add(alias_spans);
        }

        // include implementation member spans when renaming interface members
        if let Some(interface_member_target) = interface_member_target {
            let implementation_members = self.collect_interface_member_implementations(
                program,
                &interface_member_target,
                &old_name,
            );

            for member_symbol in implementation_members {
                if member_symbol == canonical_id {
                    continue;
                }

                let spans =
                    self.collect_symbol_rename_spans(program, member_symbol, &old_name, false);
                rename_spans.add(spans);
            }
        }

        // normalize span ordering and remove duplicates per file
        rename_spans.normalize();

        Some(rename_spans.into_patch_set(new_name))
    }

    /// Resolve the symbol targeted by rename at a file offset.
    fn resolve_rename_target(
        &self,
        offset: u32,
    ) -> Option<(SymbolHit, dir::GlobalSymbolId, String)> {
        // find the symbol at offset
        let symbol_at = self.find_symbol_at_offset(offset)?;

        // reject non modifier keywords at the cursor
        let token = self.token_at_offset(offset);
        if token.is_some_and(|token| match token.parse::<dir::Keyword>() {
            Ok(keyword) => !keyword.is_rename_target_modifier(),
            Err(_) => false,
        }) {
            return None;
        }

        let symbol_id = symbol_at.symbol_id;

        // keep explicit local import aliases as local rename targets
        if let Some(local_alias_name) = self.local_import_alias_name(symbol_id) {
            return Some((symbol_at, symbol_id, local_alias_name));
        }

        // resolve canonical symbol and stable rename name
        let canonical_id = self.canonical_symbol(symbol_id);
        let name = self.resolve_rename_name(canonical_id)?;

        Some((symbol_at, canonical_id, name))
    }

    /// Collect all rename spans for one canonical symbol.
    fn collect_symbol_rename_spans(
        &self,
        program: &ProgramQueryContext<'_>,
        canonical_id: dir::GlobalSymbolId,
        target_name: &str,
        preserve_local_definition: bool,
    ) -> Vec<Span> {
        // seed spans with the declaration site
        let mut spans = Vec::new();
        let definition_span = if preserve_local_definition {
            let module = self.module_context(canonical_id.module_id);

            module.symbol_local_definition_span(canonical_id)
        } else {
            self.symbol_definition_span(canonical_id)
        };
        if let Some(definition_span) = definition_span {
            spans.push(definition_span);
        }

        // collect references across indexed program modules
        let reference_search = ReferenceFilter::rename(target_name);
        let reference_spans =
            self.collect_symbol_reference_spans(program, canonical_id, reference_search);
        spans.extend(reference_spans);

        // normalize for deterministic edits
        sort_and_dedup_spans(&mut spans);
        spans
    }

    /// Collect symbol reference spans across indexed program modules.
    fn collect_symbol_reference_spans(
        &self,
        program: &ProgramQueryContext<'_>,
        canonical_id: dir::GlobalSymbolId,
        options: ReferenceFilter<'_>,
    ) -> Vec<Span> {
        let mut spans = Vec::new();

        let references = self.program_symbol_references(program, canonical_id, options);
        spans.extend(references.into_iter().map(|(_, span)| span));

        spans
    }

    /// Resolve the stable rename source name for a symbol.
    fn resolve_rename_name(&self, canonical_id: dir::GlobalSymbolId) -> Option<String> {
        // prefer the canonical symbol metadata name when present
        if let Some(name) = self.symbol_name(canonical_id) {
            return Some(name);
        }

        // use declaration based name extraction
        self.resolve_name_from_declaration(canonical_id)
    }

    /// Resolve a symbol name from its declaration when symbol metadata has no name.
    fn resolve_name_from_declaration(&self, canonical_id: dir::GlobalSymbolId) -> Option<String> {
        // resolve query context for the symbol module
        let module = self.module_context(canonical_id.module_id);

        // resolve the declaration node id
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            symbol.declaration?
        };

        let view = module.view();
        match declaration.local_id.ty {
            dir::NodeType::Member => {
                let member_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "member declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let member = view.get::<dir::Member>(member_id);
                let key = member.key()?;
                key.member_name(module.strings())
            }
            dir::NodeType::EnumField => {
                let field_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "enum field declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let field = view.get::<dir::EnumField>(field_id);
                Some(module.strings().get(field.name.string()).to_string())
            }
            dir::NodeType::Declaration => {
                let declaration_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "declaration symbol has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let declaration = view.get::<dir::Declaration>(declaration_id);
                declaration_display_name(module.strings(), declaration)
            }
            dir::NodeType::Parameter => {
                let parameter_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "parameter declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let parameter = view.get::<dir::Parameter>(parameter_id);
                match parameter {
                    dir::Parameter::Named { name, .. } => {
                        Some(module.strings().get(*name).to_string())
                    }
                    dir::Parameter::VariadicNamed { name, .. } => {
                        Some(module.strings().get(*name).to_string())
                    }
                    dir::Parameter::Pattern { .. } | dir::Parameter::VariadicPattern { .. } => None,
                    dir::Parameter::Error => panic!("error parameter reached rename refactor"),
                }
            }
            dir::NodeType::Pattern => {
                let pattern_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "pattern declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let pattern = view.get::<dir::Pattern>(pattern_id);
                match pattern {
                    dir::Pattern::Binding { name, .. } => {
                        Some(module.strings().get(*name).to_string())
                    }
                    _ => None,
                }
            }
            dir::NodeType::PatternField => {
                let field_id = declaration.local_id.try_into().unwrap_or_else(|_| {
                    panic!(
                        "pattern field declaration has incompatible node id: {:?}",
                        declaration.local_id
                    )
                });
                let field = view.get::<dir::PatternField>(field_id);
                match field {
                    dir::PatternField::Named { name, pattern, .. } => {
                        let symbol = module.node_symbol(field_id.into())?;
                        if let Some(pattern) = pattern {
                            return module.rename_pattern_binding_name(
                                module.strings(),
                                view,
                                *pattern,
                                symbol,
                            );
                        }

                        Some(module.strings().get(name.string()).to_string())
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Return the binding name for one pattern subtree.
    fn rename_pattern_binding_name(
        &self,
        strings: &StringPool,
        view: dir::View<'_>,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
        target_symbol: dir::LocalSymbolId,
    ) -> Option<String> {
        match view.get::<dir::Pattern>(pattern_id) {
            dir::Pattern::Binding { name, pattern, .. } => {
                if self.node_symbol(pattern_id.into()) == Some(target_symbol) {
                    return Some(strings.get(*name).to_string());
                }

                pattern.and_then(|pattern| {
                    self.rename_pattern_binding_name(strings, view, pattern, target_symbol)
                })
            }
            dir::Pattern::Default { pattern, .. }
            | dir::Pattern::Must(pattern)
            | dir::Pattern::BorrowOf { right: pattern, .. }
            | dir::Pattern::MoveOf { right: pattern, .. }
            | dir::Pattern::DereferenceOf { right: pattern } => {
                self.rename_pattern_binding_name(strings, view, *pattern, target_symbol)
            }
            dir::Pattern::Tuple { .. }
            | dir::Pattern::NominalTuple { .. }
            | dir::Pattern::Sequence { .. }
            | dir::Pattern::Object { .. }
            | dir::Pattern::NominalObject { .. }
            | dir::Pattern::Union { .. }
            | dir::Pattern::Wildcard
            | dir::Pattern::Expression { .. }
            | dir::Pattern::Range { .. } => None,
        }
    }
}

impl ModuleQueryContext<'_> {
    /// Resolve the interface member target for an interface member symbol.
    fn resolve_interface_member_target(
        &self,
        canonical_id: dir::GlobalSymbolId,
    ) -> Option<InterfaceMemberTarget> {
        // resolve query context for the symbol module
        let module = self.module_context(canonical_id.module_id);

        // resolve the member declaration node
        let declaration = {
            let symbols = module.symbols();
            let symbol = symbols.get_symbol(canonical_id.local_id);
            symbol.declaration?
        };

        if declaration.local_id.ty != dir::NodeType::Member {
            return None;
        }

        let view = module.view();
        let Ok(member_id) = declaration.local_id.try_into() else {
            return None;
        };
        let member = view.get::<dir::Member>(member_id);
        let member = InterfaceMember::from_member(member)?;
        let member_name = member.key.member_name(module.strings())?;

        // resolve the parent declaration and ensure it is an interface
        let parent = view.get_parent_for(member_id)?;
        if parent.ty != dir::NodeType::Declaration {
            return None;
        }
        let Ok(declaration_id) = parent.try_into() else {
            return None;
        };
        let declaration = view.get::<dir::Declaration>(declaration_id);
        let dir::Declaration::Interface(_) = declaration else {
            return None;
        };
        let interface_symbol = module.node_symbol(declaration_id.into())?;
        let interface_symbol = module.canonical_symbol(dir::GlobalSymbolId::new(
            module.module_id(),
            interface_symbol,
        ));

        Some(InterfaceMemberTarget {
            interface_symbol,
            member_name,
            member_kind: member.kind,
        })
    }

    /// Collect implementation member symbols for a resolved interface member target.
    fn collect_interface_member_implementations(
        &self,
        program: &ProgramQueryContext<'_>,
        target: &InterfaceMemberTarget,
        expected_name: &str,
    ) -> Vec<dir::GlobalSymbolId> {
        let mut members = Vec::new();
        let interface_module = self.module_context(target.interface_symbol.module_id);
        let interface_symbol = interface_module.canonical_symbol(target.interface_symbol);
        let implementing_symbols: Vec<dir::GlobalSymbolId> = program
            .base_heritage(interface_symbol)
            .into_iter()
            .filter(|entry| entry.kind == HeritageKind::Implements)
            .map(|entry| entry.derived)
            .collect();

        if implementing_symbols.is_empty() {
            return members;
        }

        let implementing_module_ids: HashMap<ModuleId, Vec<dir::LocalSymbolId>> =
            implementing_symbols
                .into_iter()
                .fold(HashMap::new(), |mut modules, symbol_id| {
                    modules
                        .entry(symbol_id.module_id)
                        .or_default()
                        .push(symbol_id.local_id);
                    modules
                });

        for (module_id, implementing_symbols) in implementing_module_ids {
            let module = self.module_context(module_id);

            let view = module.view();
            for (member_id, member) in view.iter_nodes_of_type::<dir::Member>() {
                let Some(parent) = view.get_parent_for(member_id) else {
                    continue;
                };
                if parent.ty != dir::NodeType::Declaration {
                    continue;
                }
                let Ok(declaration_id) = parent.try_into() else {
                    continue;
                };
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let owner_symbol = match declaration {
                    dir::Declaration::Class(_)
                    | dir::Declaration::Struct(_)
                    | dir::Declaration::Interface(_) => {
                        let Some(symbol) = module.node_symbol(declaration_id.into()) else {
                            continue;
                        };
                        symbol
                    }
                    _ => continue,
                };
                if !implementing_symbols.contains(&owner_symbol) {
                    continue;
                }

                let Some(member) = InterfaceMember::from_member(member) else {
                    continue;
                };
                if member.kind != target.member_kind {
                    continue;
                }

                let Some(member_name) = member.key.member_name(module.strings()) else {
                    continue;
                };
                if member_name != expected_name && member_name != target.member_name {
                    continue;
                }

                let Some(member_symbol) = module.node_symbol(member_id.into()) else {
                    continue;
                };
                let symbol_id = dir::GlobalSymbolId::new(module.module_id(), member_symbol);
                let symbol_id = module.canonical_symbol(symbol_id);
                members.push(symbol_id);
            }
        }

        members.sort();
        members.dedup();
        members
    }
}
