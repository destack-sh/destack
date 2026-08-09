use destack_dir as dir;

use super::CompletionCollector;
use crate::{CompletionCandidate, CompletionOrigin, Formatter, QueryError, QueryResult};

impl CompletionCollector<'_, '_, '_> {
    /// Collect members for one exact lookup site.
    pub(super) fn collect_members(
        &self,
        site: dir::MemberSite,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        // FUGU #Incomplete: MemberDeclaration must identify selected read and write declarations
        let members = self.module.members()?;
        let subject = members
            .subject(site)
            .ok_or(QueryError::missing(format!("member subject: {site:?}")))?;
        let bindings = members.members(site).ok_or(QueryError::missing(format!(
            "member bindings: site={site:?}, subject={subject:?}"
        )))?;
        let mut results = Vec::new();

        // collect readable members in precedence order
        for member in bindings {
            if !member.access.is_readable() {
                continue;
            }
            let dir::StaticKey::Name(name) = member.key else {
                continue;
            };
            let type_id = member
                .access
                .read()
                .ok_or(QueryError::invalid("readable member has no read type"))?;
            let label = self.module.strings().get(name).to_string();
            let completion =
                CompletionCandidate::new(label, member.kind.into(), CompletionOrigin::Member)
                    .with_member(site, member.key)
                    .with_type_id(type_id);
            let completion = match member.declarations.as_slice() {
                [declaration] => {
                    let completion = self.collect_declaration(completion, declaration.symbol)?;
                    if completion.kind.is_callable() {
                        completion.with_call()
                    } else {
                        completion
                    }
                }
                _ => completion,
            };
            results.push(completion);
        }

        Ok(results)
    }

    /// Resolve one selected member.
    pub(super) fn resolve_member(
        &self,
        completion: CompletionCandidate,
        site: dir::MemberSite,
        key: dir::StaticKey,
    ) -> QueryResult<CompletionCandidate> {
        let member = self
            .module
            .members()?
            .binding(site, key)
            .ok_or(QueryError::missing(format!(
                "completion member binding: {site:?}, {key:?}"
            )))?;
        let declaration = match member.declarations.as_slice() {
            [declaration] => Some(declaration),
            _ => None,
        };

        // render the selected access type
        let type_id = member
            .access
            .read()
            .ok_or(QueryError::invalid("readable member has no read type"))?;
        let is_callable = matches!(
            member.kind,
            dir::MemberKind::Method
                | dir::MemberKind::Constructor
                | dir::MemberKind::CallSignature
                | dir::MemberKind::ConstructSignature
        );
        let formatter = Formatter::new(self.module, self.program);
        let suffix = if is_callable {
            let callable = match declaration {
                Some(declaration) => declaration
                    .callable_type
                    .ok_or(QueryError::missing("completion member callable type"))?,
                None => type_id,
            };
            let names = declaration
                .map(|declaration| self.program.symbol_parameter_names(declaration.symbol))
                .transpose()?
                .flatten();

            formatter.callable_suffix(callable, names.as_deref())?
        } else {
            format!(": {}", formatter.global_type(type_id)?)
        };
        let completion = completion.with_label_suffix(suffix).with_type_id(type_id);

        // insert shared callable members without choosing one declaration
        if is_callable && declaration.is_none() {
            let snippet =
                super::call::CallSnippet::positional(&completion.label, type_id, self.program)?;

            if snippet.is_snippet {
                Ok(completion.with_snippet(snippet.text))
            } else {
                Ok(completion.with_insert_text(snippet.text))
            }
        } else {
            Ok(completion)
        }
    }

    /// Resolve one contextual object field.
    pub(super) fn resolve_object_field(
        &self,
        completion: CompletionCandidate,
        site: dir::MemberSite,
        key: dir::StaticKey,
    ) -> QueryResult<CompletionCandidate> {
        let member = self
            .module
            .members()?
            .binding(site, key)
            .ok_or(QueryError::missing(format!(
                "completion object field: {site:?}, {key:?}"
            )))?;
        let type_id = member.access.store();
        let type_text = Formatter::new(self.module, self.program).global_type(type_id)?;

        Ok(completion
            .with_label_suffix(format!(": {type_text}"))
            .with_type_id(type_id))
    }
}
