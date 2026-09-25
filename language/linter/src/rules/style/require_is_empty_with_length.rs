use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require isEmpty alongside a length accessor.
    pub REQUIRE_IS_EMPTY_WITH_LENGTH {
        id: "require-is-empty-with-length",
        summary: "Require isEmpty alongside a length accessor",
        explanation: r#"
A visible `length` property without `isEmpty` makes callers derive a common collection predicate themselves.
Instead, you SHOULD provide `isEmpty` in the same static or instance namespace.
"#,
        example: {
            reported: r#"
interface Sequence {
    get length(): isize;
}
"#,
            accepted: r#"
interface Sequence {
    get length(): isize;
    get isEmpty(): boolean;
}
"#,
        },
        provenance: [Clippy("len_without_is_empty")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// One declared collection query.
enum CollectionQuery {
    /// A length property requiring an emptiness query.
    Length {
        node: dir::LocalNodeIdAny,
        space: dir::MemberSpace,
    },
    /// An emptiness property or method.
    IsEmpty { space: dir::MemberSpace },
}

/// Report visible length properties without an emptiness query.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

    // inspect nominal declaration member lists
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        if let Some(members) = declaration.member_ids() {
            let mut queries = Vec::new();
            for member in members {
                if let Some(query) = collection_member(*member, view.get(*member), module)? {
                    queries.push(query);
                }
            }

            report_missing_query(module, lint, &queries, &mut output)?;
        }

        // inspect structural interface member lists
        if let Some(members) = declaration.type_member_ids() {
            let mut queries = Vec::new();
            for member in members {
                if let Some(query) =
                    structural_collection_member(*member, view.get(*member), module)?
                {
                    queries.push(query);
                }
            }

            report_missing_query(module, lint, &queries, &mut output)?;
        }
    }

    Ok(output)
}

/// Select a visible collection query from one declaration member.
fn collection_member(
    node: dir::LocalNodeId<dir::Member>,
    member: &dir::Member,
    module: &DirModule<'_>,
) -> Result<Option<CollectionQuery>, ProviderError> {
    if member.visibility() == Some(dir::Visibility::Private) {
        return Ok(None);
    }
    let Some(dir::MemberSlot::Key(key)) = member.slot() else {
        return Ok(None);
    };
    let Some(space) = member.space() else {
        return Ok(None);
    };
    let result = match member {
        dir::Member::Field {
            declared_type,
            default,
            ..
        } => declared_type
            .map(dir::LocalNodeId::into_any)
            .or_else(|| default.map(dir::LocalNodeId::into_any)),
        dir::Member::Method { signature, .. } => {
            let is_query = signature.role == Some(dir::FunctionRole::Getter)
                || signature.role.is_none() && signature.parameters.is_empty();

            is_query
                .then_some(signature.return_type)
                .flatten()
                .map(dir::LocalNodeId::into_any)
        }
        _ => None,
    };

    collection_query(node.into_any(), space, key, result, module)
}

/// Select a collection query from one structural member.
fn structural_collection_member(
    node: dir::LocalNodeId<dir::TypeMember>,
    member: &dir::TypeMember,
    module: &DirModule<'_>,
) -> Result<Option<CollectionQuery>, ProviderError> {
    let Some(dir::MemberSlot::Key(key)) = member.slot() else {
        return Ok(None);
    };
    let Some(space) = member.space() else {
        return Ok(None);
    };
    let result = match member {
        dir::TypeMember::Field { declared_type, .. } => {
            declared_type.map(dir::LocalNodeId::into_any)
        }
        dir::TypeMember::Method { signature, .. } => {
            let is_query = signature.role == Some(dir::FunctionRole::Getter)
                || signature.role.is_none() && signature.parameters.is_empty();

            is_query
                .then_some(signature.return_type)
                .flatten()
                .map(dir::LocalNodeId::into_any)
        }
        _ => None,
    };

    collection_query(node.into_any(), space, key, result, module)
}

/// Classify one property-shaped member name as a collection query.
fn collection_query(
    node: dir::LocalNodeIdAny,
    space: dir::MemberSpace,
    key: dir::StaticKey,
    result: Option<dir::LocalNodeIdAny>,
    module: &DirModule<'_>,
) -> Result<Option<CollectionQuery>, ProviderError> {
    let Some(result) = result else {
        return Ok(None);
    };
    let result = module.node_type(result)?;

    // recognize the two standardized query names
    let query = match key {
        key if key == dir::StaticKey::Name(dir::StringId::for_text("length"))
            && result.scalar_domain() == Some(dir::ScalarDomain::Integer) =>
        {
            Some(CollectionQuery::Length { node, space })
        }
        key if key == dir::StaticKey::Name(dir::StringId::for_text("isEmpty"))
            && result.is_boolean() =>
        {
            Some(CollectionQuery::IsEmpty { space })
        }
        _ => None,
    };

    Ok(query)
}

/// Report each visible length property missing isEmpty in the same namespace.
fn report_missing_query(
    module: &DirModule<'_>,
    lint: &Lint,
    members: &[CollectionQuery],
    output: &mut LintOutput,
) -> Result<(), ProviderError> {
    // compare length properties with emptiness queries in the same namespace
    for member in members {
        let CollectionQuery::Length { node, space } = member else {
            continue;
        };
        let has_is_empty = members.iter().any(|candidate| {
            matches!(candidate, CollectionQuery::IsEmpty { space: candidate } if candidate == space)
        });
        if has_is_empty {
            continue;
        }

        let span = module.main_span(*node)?;
        output.report(lint.diagnostic("length property has no matching isEmpty query", span));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a structural length getter without isEmpty.
    #[test]
    fn test_reports_interface_length() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
interface Sequence {
    get length(): isize;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-is-empty-with-length]: length property has no matching isEmpty query
 ──▶ main.tspp:2:9
  │
1 │ interface Sequence {
2 │     get length(): isize;
  │         ^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept an isEmpty method beside a length property.
    #[test]
    fn test_accepts_is_empty_method() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
interface Sequence {
    get length(): isize;
    isEmpty(): boolean;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Ignore a geometric length with a floating-point result.
    #[test]
    fn test_accepts_geometric_length() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
interface Vector {
    get length(): float64;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Reject an isEmpty query with a non-boolean result.
    #[test]
    fn test_reports_non_boolean_is_empty() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
interface Sequence {
    get length(): isize;
    get isEmpty(): string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-is-empty-with-length]: length property has no matching isEmpty query
 ──▶ main.tspp:2:9
  │
1 │ interface Sequence {
2 │     get length(): isize;
  │         ^^^^^^
3 │     get isEmpty(): string;
4 │ }
  │
"#,
        );
    }

    /// Keep static and instance namespaces distinct.
    #[test]
    fn test_reports_instance_length_with_static_is_empty() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
declare class Sequence {
    get length(): isize;
    static get isEmpty(): boolean;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[require-is-empty-with-length]: length property has no matching isEmpty query
 ──▶ main.tspp:2:9
  │
1 │ declare class Sequence {
2 │     get length(): isize;
  │         ^^^^^^
3 │     static get isEmpty(): boolean;
4 │ }
  │
"#,
        );
    }

    /// Accept a private length property without a public collection API.
    #[test]
    fn test_accepts_private_length() {
        let session = TestSession::dir(
            &REQUIRE_IS_EMPTY_WITH_LENGTH,
            r#"
class Buffer {
    private length: isize = 0;
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
