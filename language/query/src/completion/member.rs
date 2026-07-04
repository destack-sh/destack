use std::collections::HashSet;

use destack_dir as dir;

use super::builder::CompletionBuilder;
use super::call::call_snippet;
use super::{Completion, CompletionKind, SORT_BUILTIN, SORT_LOCAL_SYMBOL};
use crate::format::format_global_type;
use crate::{MemberCandidate, MemberKind, MemberName, ScopeAtOffset, SymbolUse, visible_symbols};

/// Completion-specific behavior for member kinds.
trait CompletionMemberKind {
    /// Return the completion kind for this member kind.
    fn completion_kind(self) -> CompletionKind;

    /// Return whether this member should insert call syntax.
    fn inserts_call(self) -> bool;
}

impl CompletionMemberKind for MemberKind {
    fn completion_kind(self) -> CompletionKind {
        match self {
            MemberKind::Method => CompletionKind::Method,
            MemberKind::Constructor => CompletionKind::Constructor,
            MemberKind::Field => CompletionKind::Field,
            MemberKind::CallSignature => CompletionKind::Function,
            MemberKind::ConstructSignature => CompletionKind::Constructor,
            MemberKind::EnumMember => CompletionKind::EnumMember,
            MemberKind::AssociatedType => CompletionKind::TypeParameter,
            MemberKind::AssociatedConst => CompletionKind::Constant,
        }
    }

    fn inserts_call(self) -> bool {
        matches!(self, MemberKind::Method | MemberKind::Constructor)
    }
}

impl CompletionBuilder<'_, '_> {
    /// Build a completion item from a resolved member entry.
    fn member_completion(&self, member: MemberCandidate) -> Option<Completion> {
        let MemberName::String(name) = member.name else {
            return None;
        };

        let kind = member.kind.completion_kind();

        let mut completion = Completion::new(name, kind)
            .with_sort_order(SORT_LOCAL_SYMBOL)
            .as_local();

        if member.is_extension {
            completion = completion.with_extension_member();
        }

        if let Some(member_type_id) = member.type_id {
            completion = completion.with_value_shape(self.type_value_shape(member_type_id));

            if let Some(nominal_symbol) = self.type_nominal(member_type_id) {
                completion = completion.with_nominal_symbol(nominal_symbol);
            }

            completion = completion.with_related_nominals(self.related_nominals(member_type_id));

            let type_text = format_global_type(member_type_id, self.module).unwrap_or_else(|| {
                panic!("unable to format completion member type {member_type_id:?}")
            });
            completion = completion.with_detail(type_text);
        } else if let Some(symbol_id) = member.symbol_id {
            if let Some(value_shape) = self.symbol_value_shape(symbol_id) {
                completion = completion.with_value_shape(value_shape);
            }

            if let Some(nominal_symbol) = self.symbol_nominal(symbol_id) {
                completion = completion.with_nominal_symbol(nominal_symbol);
            }

            completion = completion.with_related_nominals(self.symbol_related_nominals(symbol_id));

            if let Some(type_text) = self.format_symbol_type_detail(symbol_id) {
                completion = completion.with_detail(type_text);
            }
        }

        if let Some(symbol_id) = member.symbol_id {
            completion = self.attach_completion_documentation(completion, symbol_id);
        }

        if member.kind.inserts_call() {
            if let Some(symbol_id) = member.symbol_id {
                if let Some(parameter_names) = self.module.symbol_parameter_names(symbol_id) {
                    let snippet = call_snippet(&completion.label, &parameter_names);
                    completion = completion.with_insert_text(snippet.text);
                    if snippet.is_snippet {
                        completion = completion.as_snippet();
                    }
                }
            }
            if completion.insert_text.is_none() {
                let label = completion.label.clone();
                completion = completion.with_insert_text(format!("{label}()"));
            }
        }

        Some(completion)
    }

    /// Complete members of a type after `.`.
    pub(super) fn complete_members(
        &self,
        receiver_type: Option<dir::GlobalTypeId>,
        receiver_symbol: Option<dir::GlobalSymbolId>,
    ) -> Vec<Completion> {
        let mut results = Vec::new();

        if let Some(type_id) = receiver_type {
            let members = self.module.resolve_type_members(self.program, type_id);

            for member in members {
                let Some(completion) = self.member_completion(member) else {
                    continue;
                };

                results.push(completion);
            }

            if !results.is_empty() {
                return results;
            }
        }

        if let Some(symbol_id) = receiver_symbol {
            let members = self
                .module
                .resolve_reference_members(self.program, symbol_id);
            for member in members {
                let Some(completion) = self.member_completion(member) else {
                    continue;
                };

                results.push(completion);
            }
        }

        results
    }

    /// Complete fields inside an object literal.
    pub(super) fn complete_object_literal(
        &self,
        contextual_type: Option<dir::GlobalTypeId>,
        existing_fields: &[String],
        scope: ScopeAtOffset,
        excluded_labels: &HashSet<String>,
    ) -> Vec<Completion> {
        let mut results = Vec::new();
        let mut seen_names = HashSet::new();

        if let Some(type_id) = contextual_type {
            let members = self.module.resolve_type_members(self.program, type_id);

            for member in members {
                if member.kind != MemberKind::Field {
                    continue;
                }

                let MemberName::String(name) = member.name else {
                    continue;
                };

                if existing_fields.contains(&name) {
                    continue;
                }

                let mut completion = Completion::new(&name, CompletionKind::Field)
                    .with_insert_text(format!("{name}: $0"))
                    .as_snippet()
                    .with_sort_order(5)
                    .as_contextual();

                if let Some(member_type_id) = member.type_id {
                    let type_text =
                        format_global_type(member_type_id, self.module).unwrap_or_else(|| {
                            panic!("unable to format object literal member type {member_type_id:?}")
                        });
                    completion = completion.with_detail(type_text);
                }

                results.push(completion);
                seen_names.insert(name);
            }
        }

        let symbols = self.module.symbols();
        for visible in visible_symbols(
            symbols,
            scope.scope_id,
            scope.scope_mark,
            Some(SymbolUse::Value),
        ) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = self.module.strings().get(name_id).to_string();
            if excluded_labels.contains(&name) {
                continue;
            }

            if !seen_names.insert(name.clone()) {
                continue;
            }

            if existing_fields.contains(&name) {
                continue;
            }

            let kind = CompletionKind::from(visible.symbol.kind);
            results.push(
                Completion::new(name, kind)
                    .with_sort_order(SORT_BUILTIN)
                    .as_local(),
            );
        }

        results
    }
}
