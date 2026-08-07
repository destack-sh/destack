use destack_core::FxIndexSet;
use destack_dir as dir;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require canonical casing for declared identifiers.
    pub IDENTIFIER_CASE {
        id: "identifier-case",
        summary: "Require canonical casing for declared identifiers",
        explanation: r#"
Types, variants, extensions, and type parameters use PascalCase. Values, functions, labels, and
value parameters use camelCase. Acronyms are cased as words. Imports, string-named members, foreign
declarations, protocol implementations, and generated declarations retain their imposed names.
"#,
        example: {
            reported: r#"
struct user_record {
    name: string;
}
"#,
            accepted: r#"
struct UserRecord {
    name: string;
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The canonical case required by one declaration kind.
#[derive(Clone, Copy)]
enum IdentifierCase {
    /// A lower camel-case identifier.
    Camel,
    /// An upper camel-case identifier.
    Pascal,
}

impl IdentifierCase {
    /// Return whether one identifier has this case.
    fn matches(self, identifier: &str) -> bool {
        let mut characters = identifier.chars();
        let Some(first) = characters.next() else {
            return false;
        };
        let has_expected_initial = match self {
            Self::Camel => first.is_lowercase(),
            Self::Pascal => first.is_uppercase(),
        };
        if !has_expected_initial {
            return false;
        }

        // treat acronyms as words and exclude identifier punctuation
        let mut was_uppercase = first.is_uppercase();
        for character in characters {
            let is_uppercase = character.is_uppercase();
            if !character.is_alphanumeric() || is_uppercase && was_uppercase {
                return false;
            }
            was_uppercase = is_uppercase;
        }

        true
    }

    /// Return the source name of this case.
    fn name(self) -> &'static str {
        match self {
            Self::Camel => "camelCase",
            Self::Pascal => "PascalCase",
        }
    }

    /// Return the declaration noun used by diagnostics.
    fn noun(self) -> &'static str {
        match self {
            Self::Camel => "value",
            Self::Pascal => "type",
        }
    }
}

impl From<dir::SymbolKind> for IdentifierCase {
    /// Convert one declaration kind into its required identifier case.
    fn from(kind: dir::SymbolKind) -> Self {
        match kind {
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::Class
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Extension
            | dir::SymbolKind::GenericTypeParameter
            | dir::SymbolKind::Interface
            | dir::SymbolKind::Newtype
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Struct
            | dir::SymbolKind::TypeAlias
            | dir::SymbolKind::Variant => Self::Pascal,
            dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::Function
            | dir::SymbolKind::GenericValueParameter
            | dir::SymbolKind::Import
            | dir::SymbolKind::Label
            | dir::SymbolKind::Parameter
            | dir::SymbolKind::Variable => Self::Camel,
        }
    }
}

/// Report authored symbols whose names do not match their declaration kind.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();
    let imposed = module
        .definitions
        .member_conformances()
        .filter(|conformance| conformance.member != conformance.requirement)
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();

    // inspect each checked declaration symbol once
    for (declaration, symbol_id) in module.bindings.declaration_symbols() {
        if declaration.module_id != module.id {
            continue;
        }

        // select named authored identifiers with locally chosen names
        let symbol = module.bindings.get_symbol(symbol_id);
        let Some(name) = symbol.name() else {
            continue;
        };
        if symbol.kind == dir::SymbolKind::Import
            || !module.is_authored(declaration.local_id)
            || !is_authored_identifier(&view, declaration.local_id)
            || is_ambient(&view, declaration.local_id)
            || imposed.contains(&symbol_id.into_global(module.id))
        {
            continue;
        }

        // require the case selected by the declaration kind
        let case = IdentifierCase::from(symbol.kind);
        let name = module.dir.strings.get(name);
        let has_generic_prefix = symbol.kind == dir::SymbolKind::GenericTypeParameter
            && name
                .strip_prefix('T')
                .is_some_and(|name| !name.is_empty() && IdentifierCase::Pascal.matches(name));
        if case.matches(name) || has_generic_prefix {
            continue;
        }

        // report the authored declaration name
        let span = module.main_span(declaration.local_id)?;
        let message = format!("{} name `{name}` must use {}", case.noun(), case.name());
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

/// Return whether one node declares or belongs to an ambient declaration.
fn is_ambient(view: &dir::View<'_>, node: dir::LocalNodeIdAny) -> bool {
    if let Ok(declaration) = node.try_into_typed::<dir::Declaration>() {
        return view.get(declaration).is_ambient();
    }

    view.ancestor::<dir::Declaration>(node)
        .is_some_and(|declaration| view.get(declaration).is_ambient())
}

/// Return whether one declaration was written as an identifier.
fn is_authored_identifier(view: &dir::View<'_>, node: dir::LocalNodeIdAny) -> bool {
    // dispatch by the authored declaration node type
    if let Ok(declaration) = node.try_into_typed::<dir::Declaration>() {
        return view
            .get(declaration)
            .name()
            .is_some_and(|name| matches!(name, dir::Name::Identifier(_)));
    }
    if let Ok(member) = node.try_into_typed::<dir::Member>() {
        return member_identifier(view.get(member));
    }
    if let Ok(member) = node.try_into_typed::<dir::TypeMember>() {
        return type_member_identifier(view.get(member));
    }

    if let Ok(field) = node.try_into_typed::<dir::EnumField>() {
        return matches!(view.get(field).name, dir::Name::Identifier(_));
    }

    if let Ok(pattern) = node.try_into_typed::<dir::Pattern>() {
        return matches!(view.get(pattern), dir::Pattern::Binding { .. });
    }

    if let Ok(parameter) = node.try_into_typed::<dir::Parameter>() {
        return matches!(
            view.get(parameter),
            dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. }
        );
    }

    matches!(
        node.ty,
        dir::NodeType::GenericParameter | dir::NodeType::TypeMappedParameter
    )
}

/// Return whether one declaration member uses an identifier key.
fn member_identifier(member: &dir::Member) -> bool {
    match member {
        dir::Member::AssociatedType { .. } | dir::Member::AssociatedConst { .. } => true,
        dir::Member::Field { key, .. } | dir::Member::Method { key: Some(key), .. } => {
            matches!(key, dir::Key::Name(dir::Name::Identifier(_)))
        }
        _ => false,
    }
}

/// Return whether one type member uses an identifier key.
fn type_member_identifier(member: &dir::TypeMember) -> bool {
    match member {
        dir::TypeMember::AssociatedType { .. }
        | dir::TypeMember::AssociatedConst { .. }
        | dir::TypeMember::IndexSignature { .. } => true,
        dir::TypeMember::Field { key, .. } | dir::TypeMember::Method { key, .. } => {
            matches!(key, dir::Key::Name(dir::Name::Identifier(_)))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a non-canonical value binding.
    #[test]
    fn test_reports_non_canonical_value_name() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
function load(): void {
    const User_name = "Ada";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: value name `User_name` must use camelCase
 ──▶ main.ds:2:11
  │
1 │ function load(): void {
2 │     const User_name = "Ada";
  │           ^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Accept canonical names from both case families.
    #[test]
    fn test_accepts_canonical_identifier_cases() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct UserRecord<TValue> {
    displayName: TValue;
}
function loadUser(userId: string): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept acronyms cased as words.
    #[test]
    fn test_accepts_acronyms_as_words() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct GpuBuffer {
    resourceId: int32;
}
function toJson(value: GpuBuffer): string {
    return "";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report consecutive capitals in acronyms.
    #[test]
    fn test_reports_uppercase_acronyms() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct GPUBuffer {}
function toJSON(value: GPUBuffer): string {
    return "";
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: type name `GPUBuffer` must use PascalCase
 ──▶ main.ds:1:8
  │
1 │ struct GPUBuffer {}
  │        ^^^^^^^^^
2 │ function toJSON(value: GPUBuffer): string {
3 │     return "";
  │

warning[identifier-case]: value name `toJSON` must use camelCase
 ──▶ main.ds:2:10
  │
1 │ struct GPUBuffer {}
2 │ function toJSON(value: GPUBuffer): string {
  │          ^^^^^^
3 │     return "";
4 │ }
  │
"#,
        );
    }

    /// Accept string-named members.
    #[test]
    fn test_accepts_imposed_member_names() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
interface ForeignShape {
    "snake_name": string;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept names imposed by ambient protocol declarations.
    #[test]
    fn test_accepts_protocol_imposed_member_names() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
declare interface ForeignProtocol {
    snake_name(): void;
}
struct Value {}
extension of Value implements ForeignProtocol {
    snake_name(): void {}
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report a non-canonical default protocol member.
    #[test]
    fn test_reports_non_canonical_default_protocol_member() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
newtype interface ForeignProtocol {
    snake_name(): void {}
}
struct Value {}
extension of Value implements ForeignProtocol {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: value name `snake_name` must use camelCase
 ──▶ main.ds:2:5
  │
1 │ newtype interface ForeignProtocol {
2 │     snake_name(): void {}
  │     ^^^^^^^^^^
3 │ }
4 │ struct Value {}
  │
"#,
        );
    }

    /// Report a non-canonical generic type parameter.
    #[test]
    fn test_reports_non_canonical_type_parameter() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct Box<value_type> {
    value: value_type;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: type name `value_type` must use PascalCase
 ──▶ main.ds:1:12
  │
1 │ struct Box<value_type> {
  │            ^^^^^^^^^^
2 │     value: value_type;
3 │ }
  │
"#,
        );
    }

    /// Report a non-canonical enum variant.
    #[test]
    fn test_reports_non_canonical_variant_name() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
enum Mode {
    read_only = 1,
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: type name `read_only` must use PascalCase
 ──▶ main.ds:2:5
  │
1 │ enum Mode {
2 │     read_only = 1,
  │     ^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Report a non-canonical field name.
    #[test]
    fn test_reports_non_canonical_field_name() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct User {
    display_name: string;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: value name `display_name` must use camelCase
 ──▶ main.ds:2:5
  │
1 │ struct User {
2 │     display_name: string;
  │     ^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }
}
