use destack_dir as dir;

use super::builder::CompletionBuilder;
use crate::{
    CompletionCandidate, CompletionOrigin, Formatter, QueryError, QueryResult, SORT_MEMBER,
};

impl CompletionBuilder<'_, '_, '_> {
    /// Complete one member expression.
    pub(super) fn complete_members(
        &self,
        source: dir::GlobalNodeIdAny,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let members = self.module.members()?;
        let site = dir::MemberSite::Node(source);
        let subject = members
            .subject(site)
            .ok_or(QueryError::missing(format!("member subject: {source:?}")))?;
        let bindings = members.members(site).ok_or(QueryError::missing(format!(
            "member bindings: source={source:?}, subject={subject:?}"
        )))?;
        let formatter = Formatter::new(self.module, self.program);
        let mut results = Vec::new();

        // render members expressible after a dot in precedence order
        for member in bindings {
            let dir::StaticKey::Name(name) = member.key else {
                continue;
            };
            let label = self.module.strings().get(name).to_string();
            let completion = CompletionCandidate::new(
                label,
                member.kind.into(),
                CompletionOrigin::Member,
                SORT_MEMBER,
            );
            results.push(self.describe_member(completion, member, &formatter)?);
        }

        Ok(results)
    }

    /// Build one completion from a selected member.
    pub(super) fn describe_member(
        &self,
        mut completion: CompletionCandidate,
        member: &dir::MemberBinding,
        formatter: &Formatter<'_, '_, '_>,
    ) -> QueryResult<CompletionCandidate> {
        let declaration = member.declarations.first();
        if let Some(symbol) = declaration.map(|declaration| declaration.symbol) {
            completion = self.resolve_declaration(completion, symbol)?;
            if completion.kind.is_callable() {
                completion = completion.with_call();
            }
        }

        // render the selected access type
        let type_id = member
            .access
            .read()
            .or_else(|| member.access.write())
            .ok_or(QueryError::invalid("member selection has no access type"))?;
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
}
