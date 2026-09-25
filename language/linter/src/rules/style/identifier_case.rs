use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require canonical casing for declared identifiers.
    pub IDENTIFIER_CASE {
        id: "identifier-case",
        summary: "Require canonical casing for declared identifiers",
        explanation: r#"
Identifier casing communicates whether a declaration introduces a type or a value.
You SHOULD use PascalCase for types, variants, extensions, and generic parameters, and camelCase for values, functions, labels, and value parameters.

Constants use the same camelCase as other values.
Newtypes MAY use camelCase when they act as decorators.
Acronyms MAY retain an established platform spelling.
Imports, string-named members, foreign declarations, protocol implementations, and generated declarations retain their imposed names.
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
        provenance: [TypeScriptEslint("naming-convention")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The identifier style selected by one declaration kind.
#[derive(Clone, Copy, PartialEq, Eq)]
enum IdentifierStyle {
    /// An ordinary value name.
    Value,
    /// A type-level name.
    Type,
    /// A newtype used as a type or decorator.
    Newtype,
    /// A module namespace.
    Namespace,
    /// A lifetime parameter.
    Lifetime,
}

impl IdentifierStyle {
    /// Return whether one identifier has this style.
    fn matches(self, identifier: &str) -> bool {
        let identifier = match self {
            Self::Lifetime => identifier.strip_prefix('\'').unwrap_or(identifier),
            _ => identifier,
        };

        match self {
            Self::Value | Self::Lifetime => matches_case(identifier, false),
            Self::Type => matches_case(identifier, true),
            Self::Newtype | Self::Namespace => {
                matches_case(identifier, false) || matches_case(identifier, true)
            }
        }
    }

    /// Return the source description of this style.
    fn name(self) -> &'static str {
        match self {
            Self::Value => "camelCase",
            Self::Type => "PascalCase",
            Self::Newtype => "camelCase or PascalCase",
            Self::Namespace => "camelCase or PascalCase",
            Self::Lifetime => "lower camelCase after the apostrophe",
        }
    }

    /// Return the declaration noun used by diagnostics.
    fn noun(self, kind: dir::SymbolKind) -> &'static str {
        match (self, kind) {
            (_, dir::SymbolKind::Label) => "label",
            (Self::Type | Self::Newtype, _) => "type",
            (Self::Namespace, _) => "namespace",
            (Self::Value | Self::Lifetime, _) => "value",
        }
    }

    /// Resolve the style selected by one declaration.
    fn resolve(
        kind: dir::SymbolKind,
        declaration: dir::GlobalNodeIdAny,
        module: &DirModule<'_>,
    ) -> Result<Option<Self>, ProviderError> {
        match kind {
            dir::SymbolKind::ExportAlias => Self::resolve_export(declaration, module),
            kind => Ok(Self::classify(kind)),
        }
    }

    /// Classify one direct declaration kind.
    fn classify(kind: dir::SymbolKind) -> Option<Self> {
        match kind {
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::Class
            | dir::SymbolKind::Enum
            | dir::SymbolKind::Extension
            | dir::SymbolKind::GenericConstParameter
            | dir::SymbolKind::GenericTypeParameter
            | dir::SymbolKind::Interface
            | dir::SymbolKind::NewtypeInterface
            | dir::SymbolKind::Struct
            | dir::SymbolKind::TypeAlias
            | dir::SymbolKind::Variant => Some(Self::Type),
            dir::SymbolKind::Newtype => Some(Self::Newtype),
            dir::SymbolKind::GenericLifetimeParameter => Some(Self::Lifetime),
            dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::Function
            | dir::SymbolKind::Import
            | dir::SymbolKind::Label
            | dir::SymbolKind::Parameter
            | dir::SymbolKind::Variable => Some(Self::Value),
            dir::SymbolKind::ExportAlias => None,
        }
    }

    /// Resolve the style selected by one public export alias target.
    fn resolve_export(
        declaration: dir::GlobalNodeIdAny,
        module: &DirModule<'_>,
    ) -> Result<Option<Self>, ProviderError> {
        // read the export target
        let reference = module.resolved.references.get(declaration).ok_or_else(|| {
            ProviderError::internal(format!(
                "export alias {declaration:?} has no resolved target"
            ))
        })?;
        let targets = match reference {
            dir::Reference::Bound(symbols) => symbols,
            dir::Reference::Namespace { .. } => return Ok(Some(Self::Namespace)),
            dir::Reference::Ambiguous(_)
            | dir::Reference::TypeLiteral(_)
            | dir::Reference::Missing => return Ok(None),
            dir::Reference::Projected { .. } => {
                return Err(ProviderError::internal(format!(
                    "export alias {declaration:?} has a projected target"
                )));
            }
        };

        // require a nonempty symbol group
        if targets.is_empty() {
            return Err(ProviderError::internal(format!(
                "export alias {declaration:?} has no target"
            )));
        }

        // require every exported declaration to select the same style
        let mut selected = None;
        for target in targets {
            let target_module = module.dir.module(target.module_id)?;
            let target_symbol = target_module.bindings.get_symbol(target.local_id);
            let Some(candidate) = Self::classify(target_symbol.kind) else {
                return Err(ProviderError::internal(format!(
                    "export alias {declaration:?} targets alias {target:?}"
                )));
            };
            if selected.is_some_and(|selected| selected != candidate) {
                return Err(ProviderError::internal(format!(
                    "export alias {declaration:?} has incompatible targets"
                )));
            }
            selected = Some(candidate);
        }

        Ok(selected)
    }
}

/// Return whether one identifier starts with the requested case and contains no punctuation.
fn matches_case(identifier: &str, is_pascal: bool) -> bool {
    let mut characters = identifier.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    let has_expected_initial = if is_pascal {
        first.is_uppercase()
    } else {
        first.is_lowercase()
    };
    if !has_expected_initial {
        return false;
    }

    characters.all(char::is_alphanumeric)
}

/// Report symbols whose names do not match their declaration kind.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();
    let imposed = module
        .members
        .member_conformances()
        .filter(|conformance| conformance.member != conformance.requirement)
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();

    // inspect each declaration symbol once
    for (declaration, symbol_id) in module.bindings.declaration_symbols() {
        if declaration.module_id != module.id {
            continue;
        }

        // select named identifiers with locally chosen names
        let symbol = module.bindings.get_symbol(symbol_id);
        let Some(name) = symbol.name() else {
            continue;
        };

        if symbol.kind == dir::SymbolKind::Import
            || !has_identifier_name(&view, declaration.local_id)
            || is_ambient(&view, declaration.local_id)
            || imposed.contains(&symbol_id.into_global(module.id))
        {
            continue;
        }

        // require the case selected by the declaration kind
        let global_symbol = symbol_id.into_global(module.id);
        if module.dir.language_item(global_symbol).is_some()
            || module.dir.language_member(global_symbol)?.is_some()
        {
            continue;
        }
        let style = IdentifierStyle::resolve(symbol.kind, declaration, module)?;
        let Some(style) = style else {
            continue;
        };
        let name = module.dir.strings.get(name);
        let name = if matches!(
            symbol.kind,
            dir::SymbolKind::Parameter | dir::SymbolKind::Variable
        ) {
            name.strip_prefix('_').unwrap_or(name)
        } else {
            name
        };
        if style.matches(name) {
            continue;
        }

        // report the declaration name
        let span = module.main_span(declaration.local_id)?;
        let noun = style.noun(symbol.kind);
        let message = format!("{noun} name `{name}` must use {}", style.name());
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

/// Return whether one declaration has an identifier name.
fn has_identifier_name(view: &dir::View<'_>, node: dir::LocalNodeIdAny) -> bool {
    match node.ty {
        // inspect declaration names
        dir::NodeType::Declaration => view
            .get(dir::LocalNodeId::<dir::Declaration>::new(node.id))
            .name()
            .is_some_and(|name| matches!(name, dir::Name::Identifier(_))),

        // inspect value members
        dir::NodeType::Member => {
            member_identifier(view.get(dir::LocalNodeId::<dir::Member>::new(node.id)))
        }

        // inspect type members
        dir::NodeType::TypeMember => {
            type_member_identifier(view.get(dir::LocalNodeId::<dir::TypeMember>::new(node.id)))
        }

        // inspect enum fields
        dir::NodeType::EnumField => matches!(
            view.get(dir::LocalNodeId::<dir::EnumField>::new(node.id))
                .name,
            dir::Name::Identifier(_)
        ),

        // inspect pattern bindings
        dir::NodeType::Pattern => matches!(
            view.get(dir::LocalNodeId::<dir::Pattern>::new(node.id)),
            dir::Pattern::Binding { .. }
        ),

        // inspect named parameters
        dir::NodeType::Parameter => matches!(
            view.get(dir::LocalNodeId::<dir::Parameter>::new(node.id)),
            dir::Parameter::Named { .. } | dir::Parameter::VariadicNamed { .. }
        ),

        // inspect control labels
        dir::NodeType::Expression => view
            .get(dir::LocalNodeId::<dir::Expression>::new(node.id))
            .control_label()
            .is_some(),

        // inspect explicit dependency aliases
        dir::NodeType::DependencyItem => matches!(
            view.get(dir::LocalNodeId::<dir::DependencyItem>::new(node.id)),
            dir::DependencyItem::Binding { alias: Some(_), .. }
        ),

        // accept identifier-only declarations
        dir::NodeType::GenericParameter | dir::NodeType::TypeMappedParameter => true,
        _ => false,
    }
}

/// Return whether one declaration member uses an identifier name.
fn member_identifier(member: &dir::Member) -> bool {
    match member {
        dir::Member::AssociatedType { .. } | dir::Member::AssociatedConst { .. } => true,
        dir::Member::Field { name, .. }
        | dir::Member::Method {
            name: Some(name), ..
        } => {
            matches!(name, dir::Name::Identifier(_))
        }
        _ => false,
    }
}

/// Return whether one type member uses an identifier name.
fn type_member_identifier(member: &dir::TypeMember) -> bool {
    match member {
        dir::TypeMember::AssociatedType { .. }
        | dir::TypeMember::AssociatedConst { .. }
        | dir::TypeMember::IndexSignature { .. } => true,
        dir::TypeMember::Field { name, .. } | dir::TypeMember::Method { name, .. } => {
            matches!(name, dir::Name::Identifier(_))
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
 ──▶ main.tspp:2:11
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

    /// Report a non-canonical control label.
    #[test]
    fn test_reports_non_canonical_label_name() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
OuterLoop: loop {
    break;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: label name `OuterLoop` must use camelCase
 ──▶ main.tspp:1:1
  │
1 │ OuterLoop: loop {
  │ ^^^^^^^^^
2 │     break;
3 │ }
  │
"#,
        );
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

    /// Accept established initialism casing.
    #[test]
    fn test_accepts_established_initialisms() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct GPUBuffer {}

function toJSON(value: GPUBuffer): string {
    return "";
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept value-cased and type-cased namespace aliases.
    #[test]
    fn test_accepts_namespace_aliases() {
        let session = TestSession::dir_files(
            &IDENTIFIER_CASE,
            "main.tspp",
            r#"
export * as Now from "./now.tspp";

export * as utilities from "./utilities.tspp";
"#,
            &[
                ("now.tspp", "export const instant = 1;"),
                ("utilities.tspp", "export const value = 1;"),
            ],
        );

        session.assert_no_diagnostics();
    }

    /// Accept camelCase module and associated constant names.
    #[test]
    fn test_accepts_constant_names() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
export const fileOpenRead: uint32 = 1;

export const defaultLimit: uint32 = 64;

export newtype FileOpenFlags = uint32;

export extension of FileOpenFlags {
    const read = FileOpenFlags(1);
}

struct Limits {
    static readonly maxSize: uint32 = 64;
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report module, associated, and local constants outside value case.
    #[test]
    fn test_reports_uppercase_constants() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
export const DEFAULT_LIMIT = 64;

struct Limits {
    static readonly MAX_SIZE: uint32 = 64;
}

function load(): void {
    const LOCAL_LIMIT = 32;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: value name `DEFAULT_LIMIT` must use camelCase
 ──▶ main.tspp:1:14
  │
1 │ export const DEFAULT_LIMIT = 64;
  │              ^^^^^^^^^^^^^
2 │
3 │ struct Limits {
  │

warning[identifier-case]: value name `MAX_SIZE` must use camelCase
 ──▶ main.tspp:4:21
  │
2 │
3 │ struct Limits {
4 │     static readonly MAX_SIZE: uint32 = 64;
  │                     ^^^^^^^^
5 │ }
6 │
  │

warning[identifier-case]: value name `LOCAL_LIMIT` must use camelCase
 ──▶ main.tspp:8:11
  │
6 │
7 │ function load(): void {
8 │     const LOCAL_LIMIT = 32;
  │           ^^^^^^^^^^^
9 │ }
  │
"#,
        );
    }

    /// Accept generic parameters and intentionally unused bindings.
    #[test]
    fn test_accepts_generic_and_unused_names() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
struct Buffer<T, 'a, const N: usize> {
    values: &'a readonly [T; N];
}

function observe(_value: Buffer<string, 'static, 4>): void {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept value-cased decorator newtypes.
    #[test]
    fn test_accepts_decorator_newtype_name() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
newtype traced = (string,) | ();

@traced("load")
function load(): void {}
"#,
        );

        session.assert_no_diagnostics();
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
 ──▶ main.tspp:2:5
  │
1 │ newtype interface ForeignProtocol {
2 │     snake_name(): void {}
  │     ^^^^^^^^^^
3 │ }
4 │
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
 ──▶ main.tspp:1:12
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
 ──▶ main.tspp:2:5
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
 ──▶ main.tspp:2:5
  │
1 │ struct User {
2 │     display_name: string;
  │     ^^^^^^^^^^^^
3 │ }
  │
"#,
        );
    }

    /// Classify an export alias from its resolved declaration.
    #[test]
    fn test_reports_export_alias_case() {
        let session = TestSession::dir(
            &IDENTIFIER_CASE,
            r#"
function load(): void {}

export { load as Load };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[identifier-case]: value name `Load` must use camelCase
 ──▶ main.tspp:3:18
  │
1 │ function load(): void {}
2 │
3 │ export { load as Load };
  │                  ^^^^
  │
"#,
        );
    }
}
