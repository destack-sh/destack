use destack_dir as dir;

use super::CompletionCollector;
use crate::{
    CompletionCandidate, CompletionOrigin, Formatter, QueryError, QueryResult, SORT_MEMBER,
};

impl CompletionCollector<'_, '_, '_> {
    /// Collect members for one exact lookup site.
    pub(super) fn collect_members(
        &self,
        site: dir::MemberSite,
    ) -> QueryResult<Vec<CompletionCandidate>> {
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
            let completion = CompletionCandidate::new(
                label,
                member.kind.into(),
                CompletionOrigin::Member,
                SORT_MEMBER,
            )
            .with_member(site, member.key)
            .with_type_id(type_id);
            let completion = match member.declarations.first() {
                Some(declaration) => {
                    let completion = self.collect_declaration(completion, declaration.symbol)?;
                    if completion.kind.is_callable() {
                        completion.with_call()
                    } else {
                        completion
                    }
                }
                None => completion,
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
        let declaration = member.declarations.first();

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
        let callable = if is_callable {
            declaration.and_then(|declaration| declaration.callable_type)
        } else {
            None
        };
        let formatter = Formatter::new(self.module, self.program);
        let detail = match callable {
            Some(callable) => {
                let names = declaration
                    .map(|declaration| declaration.symbol)
                    .map(|symbol| self.program.symbol_parameter_names(symbol))
                    .transpose()?
                    .flatten();

                formatter.callable_type(callable, names.as_deref())?
            }
            None => formatter.global_type(type_id)?,
        };

        Ok(completion.with_detail(detail).with_type_id(type_id))
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
        let detail = Formatter::new(self.module, self.program).global_type(type_id)?;

        Ok(completion.with_detail(detail).with_type_id(type_id))
    }
}
