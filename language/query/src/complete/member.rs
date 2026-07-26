use destack_dir as dir;
use rustc_hash::FxHashSet;

use super::builder::CompletionBuilder;
use super::call::call_snippet;
use crate::format::{format_global_callable_type, format_global_type};
use crate::{
    CompletionCandidate, CompletionItemKind, CompletionOrigin, MemberCandidate, MemberKind,
    MemberName, QueryError, QueryResult, SORT_BUILTIN, SORT_LOCAL_SYMBOL, ScopeAtOffset, SymbolUse,
    visible_symbols,
};

impl From<MemberKind> for CompletionItemKind {
    /// Convert one resolved member kind into its completion kind.
    fn from(kind: MemberKind) -> Self {
        match kind {
            MemberKind::Method => Self::Method,
            MemberKind::Constructor => Self::Constructor,
            MemberKind::Field => Self::Field,
            MemberKind::Property => Self::Property,
            MemberKind::CallSignature => Self::Function,
            MemberKind::ConstructSignature => Self::Constructor,
            MemberKind::EnumMember => Self::EnumMember,
            MemberKind::AssociatedType => Self::AssociatedType,
            MemberKind::AssociatedConst => Self::AssociatedConst,
        }
    }
}

impl MemberKind {
    /// Return whether this member should insert call syntax.
    fn inserts_call(self) -> bool {
        matches!(self, MemberKind::Method | MemberKind::Constructor)
    }
}

impl CompletionBuilder<'_, '_, '_> {
    /// Build a completion item from a resolved member entry.
    fn member_completion(
        &self,
        member: MemberCandidate,
    ) -> QueryResult<Option<CompletionCandidate>> {
        let MemberName::String(name) = member.name else {
            return Ok(None);
        };

        let kind = member.kind.into();

        let mut completion =
            CompletionCandidate::new(name, kind, CompletionOrigin::Local, SORT_LOCAL_SYMBOL);

        if member.is_extension {
            completion = completion.with_extension_member();
        }

        if let Some(member_type_id) = member.type_id {
            let parameter_names = match member.symbol_id {
                Some(symbol_id) => self.program.symbol_parameter_names(symbol_id)?,
                None => None,
            };
            let type_text = match parameter_names {
                Some(parameter_names) => format_global_callable_type(
                    member_type_id,
                    &parameter_names,
                    self.module,
                    self.program,
                )?,
                None => format_global_type(member_type_id, self.module, self.program)?,
            };
            if let Some(type_text) = type_text {
                completion = completion.with_detail(type_text);
            }
        } else if let Some(symbol_id) = member.symbol_id
            && let Some(type_text) = self.format_symbol_type_detail(symbol_id)?
        {
            completion = completion.with_detail(type_text);
        }

        // show the declaring enum rather than repeating the selected variant
        if kind == CompletionItemKind::EnumMember {
            let missing_symbol = || {
                QueryError::missing(format!(
                    "completion member symbol: {:?}",
                    completion.label.clone()
                ))
            };
            let symbol_id = member.symbol_id.ok_or_else(missing_symbol)?;
            let symbol_id = self
                .program
                .canonical_symbol(symbol_id)?
                .ok_or_else(missing_symbol)?;
            let module = self.program.module(symbol_id.module_id)?;
            let container = module
                .local_symbol_container_name(symbol_id.local_id)
                .ok_or(QueryError::missing(format!(
                    "completion container: {symbol_id:?}"
                )))?;
            completion = completion.with_detail(container);
        }

        if let Some(symbol_id) = member.symbol_id {
            completion = self.attach_symbol_completion(completion, symbol_id)?;
        }

        if member.kind.inserts_call()
            && let Some(symbol_id) = member.symbol_id
            && let Some(parameter_names) = self.program.symbol_parameter_names(symbol_id)?
        {
            let snippet = call_snippet(&completion.label, &parameter_names);
            completion = completion.with_insert_text(snippet.text);
            if snippet.is_snippet {
                completion = completion.with_snippet();
            }
        }

        Ok(Some(completion))
    }

    /// Complete members of a type after `.`.
    pub(super) fn complete_members(
        &self,
        receiver_type: Option<dir::GlobalTypeId>,
        is_optional: bool,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let Some(type_id) = receiver_type else {
            return Ok(Vec::new());
        };
        let members = self.module.resolve_type_members(
            self.program,
            self.environment,
            type_id,
            is_optional,
        )?;
        let mut results = Vec::new();

        // format every member supplied by the checked receiver type
        for member in members {
            let Some(completion) = self.member_completion(member)? else {
                continue;
            };
            results.push(completion);
        }

        Ok(results)
    }

    /// Complete visible shorthand values inside an object literal.
    pub(super) fn complete_object_literal_shorthands(
        &self,
        existing_fields: &[String],
        scope: ScopeAtOffset,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let mut results = Vec::new();
        let mut seen_names = FxHashSet::default();

        // collect visible values that can form shorthand fields
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
            if !seen_names.insert(name.clone()) {
                continue;
            }

            if existing_fields.contains(&name) {
                continue;
            }

            let symbol_id = dir::GlobalSymbolId {
                module_id: self.module.module_id(),
                local_id: visible.id,
            };
            let completion = CompletionCandidate::new(
                name,
                CompletionItemKind::Field,
                CompletionOrigin::Local,
                SORT_BUILTIN,
            );
            results.push(self.attach_symbol_completion(completion, symbol_id)?);
        }

        Ok(results)
    }
}
