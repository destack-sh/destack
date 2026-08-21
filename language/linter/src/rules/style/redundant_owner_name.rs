use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_repository::ProviderError;
use destack_source::Span;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

/// One name that repeats its owner.
struct OwnerRepetition {
    /// The repeated name span.
    span: Span,
    /// The repeated declaration name.
    name: String,
    /// The repeated owner name.
    owner: String,
    /// The removable authored segment.
    repeated: String,
    /// The declaration kind.
    noun: &'static str,
}

declare_lint! {
    /// Disallow names that repeat their type or module owner.
    pub REDUNDANT_OWNER_NAME {
        id: "redundant-owner-name",
        summary: "Disallow names that repeat their type or module owner",
        explanation: r#"
Repeating an owner name duplicates context already supplied by its type or module namespace.
Instead, you SHOULD name members and declarations relative to their nearest meaningful owner.

A name equal to its owner remains valid because removing it would leave no name.
"#,
        example: {
            reported: r#"
enum Color {
    ColorRed,
    Blue,
}
"#,
            accepted: r#"
enum Color {
    Red,
    Blue,
}
"#,
        },
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report declarations and members repeating their nearest named owner.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let implementations = module
        .definitions
        .member_conformances()
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();
    let mut repetitions = Vec::new();
    let mut output = LintOutput::default();

    // inspect module-scope declarations against the physical module name
    if let Some(owner) = module_owner(module) {
        for symbol_id in module.bindings.symbol_ids() {
            let symbol = module.bindings.get_symbol(symbol_id);
            if symbol.scope.id != module.namespace_scope
                || symbol.origin != dir::SymbolOrigin::Module
                || matches!(
                    symbol.kind,
                    dir::SymbolKind::Import | dir::SymbolKind::ExportAlias
                )
            {
                continue;
            }
            let (Some(dir::StaticKey::Name(name)), Some(declaration)) =
                (symbol.key, symbol.declaration)
            else {
                continue;
            };
            let declaration = declaration.local_id;

            record_owner_repetition(
                module,
                declaration,
                module.dir.strings.get(name),
                &owner,
                "declaration",
                &mut repetitions,
            )?;
        }

        // inspect aliases introduced at the module export boundary
        for (_, export) in module.exported.exports.exports() {
            let Some(item) = export.item() else {
                continue;
            };
            let Some(name) = exported_identifier(export, &view, module) else {
                continue;
            };
            record_owner_repetition(
                module,
                item.into_any(),
                name,
                &owner,
                "export",
                &mut repetitions,
            )?;
        }
    }

    // inspect each nominal or structural declaration's owned members
    for (node, declaration) in view.iter_nodes::<dir::Declaration>() {
        let Some(owner) = declaration_owner(node, declaration, module)? else {
            continue;
        };

        // inspect enum variants
        if let dir::Declaration::Enum(declaration) = declaration {
            for field in &declaration.fields {
                let dir::Name::Identifier(name) = view.get(*field).name else {
                    continue;
                };
                record_owner_repetition(
                    module,
                    field.into_any(),
                    module.dir.strings.get(name),
                    &owner,
                    "variant",
                    &mut repetitions,
                )?;
            }
        }

        // inspect nominal declaration members not imposed by an interface
        if let Some(members) = declaration.member_ids() {
            for member in members {
                let source = member.into_global_any(module.id);
                let symbol = module.bindings.declaration_symbol(source).ok_or_else(|| {
                    ProviderError::internal(format!(
                        "checked declaration member {source:?} has no symbol"
                    ))
                })?;
                let is_implementation = implementations.contains(&symbol.into_global(module.id));
                if is_implementation {
                    continue;
                }
                let Some(dir::Name::Identifier(name)) = view.get(*member).name() else {
                    continue;
                };
                record_owner_repetition(
                    module,
                    member.into_any(),
                    module.dir.strings.get(name),
                    &owner,
                    "member",
                    &mut repetitions,
                )?;
            }
        }

        // inspect structural interface members
        if let Some(members) = declaration.type_member_ids() {
            for member in members {
                let Some(dir::Name::Identifier(name)) = view.get(*member).name() else {
                    continue;
                };
                record_owner_repetition(
                    module,
                    member.into_any(),
                    module.dir.strings.get(name),
                    &owner,
                    "member",
                    &mut repetitions,
                )?;
            }
        }
    }

    // report repeated names in physical source order
    repetitions.sort_unstable_by_key(|repetition| {
        (
            repetition.span.file,
            repetition.span.start,
            repetition.span.end,
        )
    });
    for repetition in repetitions {
        let diagnostic = lint
            .diagnostic(
                format!(
                    "{} `{}` repeats owner name `{}`",
                    repetition.noun, repetition.name, repetition.owner
                ),
                repetition.span,
            )
            .help(format!(
                "remove `{}` from this {} name",
                repetition.repeated, repetition.noun
            ));
        output.report(diagnostic);
    }

    Ok(output)
}

/// Return the PascalCase owner represented by the primary module path.
fn module_owner(module: &DirModule<'_>) -> Option<String> {
    let file = module.files.first()?;
    let stem = file.name.strip_suffix(".ds")?.split('.').next()?;
    let stem = if stem == "index" {
        file.path.as_deref()?.parent()?.file_name()?.to_str()?
    } else {
        stem
    };

    Some(pascal_case(stem))
}

/// Convert one kebab-case module component to PascalCase.
fn pascal_case(value: &str) -> String {
    let mut result = String::with_capacity(value.len());

    // capitalize each nonempty module word
    for word in value.split('-').filter(|word| !word.is_empty()) {
        let mut characters = word.chars();
        if let Some(first) = characters.next() {
            result.extend(first.to_uppercase());
            result.extend(characters);
        }
    }

    result
}

/// Return the owner name of one declaration with members.
fn declaration_owner(
    node: dir::LocalNodeId<dir::Declaration>,
    declaration: &dir::Declaration,
    module: &DirModule<'_>,
) -> Result<Option<String>, ProviderError> {
    let direct = match declaration {
        dir::Declaration::Struct(declaration) => Some(declaration.name),
        dir::Declaration::Class(declaration) => declaration.name,
        dir::Declaration::Enum(declaration) => declaration.name,
        dir::Declaration::Interface(declaration) => declaration.name,
        _ => None,
    };
    if let Some(dir::Name::Identifier(name)) = direct {
        return Ok(Some(module.dir.strings.get(name).to_string()));
    }

    // resolve extension members to their checked receiver declaration
    if matches!(declaration, dir::Declaration::Extension(_)) {
        let source = node.into_global_any(module.id);
        let symbol = module.bindings.declaration_symbol(source).ok_or_else(|| {
            ProviderError::internal(format!(
                "checked extension {source:?} has no declaration symbol"
            ))
        })?;
        let symbol = symbol.into_global(module.id);
        let definition = module
            .definitions
            .extension_definition(symbol)
            .ok_or_else(|| {
                ProviderError::internal(format!("checked extension {symbol:?} has no definition"))
            })?;
        let Some(root) = definition.target.declaration() else {
            return Ok(None);
        };
        let owner = module.dir.module(root.module_id)?;
        let name = owner.bindings.get_symbol(root.local_id).name();

        return Ok(name.map(|name| module.dir.strings.get(name).to_string()));
    }

    Ok(None)
}

/// Return one identifier introduced explicitly by an export clause.
fn exported_identifier<'a>(
    export: &dir::NamedExport,
    view: &dir::View<'_>,
    module: &'a DirModule<'_>,
) -> Option<&'a str> {
    let key = export.key().named_key()?;
    let dir::StaticKey::Name(name) = key else {
        return None;
    };

    // skip unchanged local exports already checked at their declaration
    if let dir::ExportBinding::Local { symbols } = &export.binding {
        let is_unchanged = symbols.iter().all(|symbol| {
            module.bindings.get_symbol(*symbol).key == Some(dir::StaticKey::Name(name))
        });
        if is_unchanged {
            return None;
        }
    }

    // exclude explicitly string-named export keys
    let item = export.item()?;
    let dir::DependencyItem::Binding {
        name: source,
        alias,
        ..
    } = view.get(item)
    else {
        return None;
    };
    if alias.is_none() && matches!(source, Some(dir::Name::String(_) | dir::Name::Index(_))) {
        return None;
    }

    Some(module.dir.strings.get(name))
}

/// Record one proper owner prefix or suffix in a declaration name.
fn record_owner_repetition(
    module: &DirModule<'_>,
    node: dir::LocalNodeIdAny,
    name: &str,
    owner: &str,
    noun: &'static str,
    repetitions: &mut Vec<OwnerRepetition>,
) -> Result<(), ProviderError> {
    let Some(repeated) = repeated_owner(name, owner) else {
        return Ok(());
    };

    let span = module.main_span(node)?;
    repetitions.push(OwnerRepetition {
        span,
        name: name.to_string(),
        owner: owner.to_string(),
        repeated: repeated.to_string(),
        noun,
    });

    Ok(())
}

/// Return the removable owner prefix or suffix in one identifier.
fn repeated_owner<'a>(name: &'a str, owner: &str) -> Option<&'a str> {
    if name.len() <= owner.len() {
        return None;
    }

    // compare a prefix while allowing the value's initial camel-case letter
    let prefix_matches = name
        .get(..owner.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(owner));
    let prefix_boundary = name
        .get(owner.len()..)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|character| character.is_ascii_uppercase() || character.is_ascii_digit());
    if prefix_matches && prefix_boundary {
        return name.get(..owner.len());
    }

    // compare a suffix only when it begins at an identifier word boundary
    let suffix_start = name.len() - owner.len();
    let suffix = name.get(suffix_start..)?;
    let suffix_boundary = suffix
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_uppercase());

    (suffix.eq_ignore_ascii_case(owner) && suffix_boundary).then_some(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a declaration repeating its module namespace.
    #[test]
    fn test_reports_module_owner_name() {
        let session = TestSession::dir_path(
            &REDUNDANT_OWNER_NAME,
            "ast.ds",
            r#"
struct AstNode {}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[redundant-owner-name]: declaration `AstNode` repeats owner name `Ast`
 ──▶ ast.ds:1:8
  │
1 │ struct AstNode {}
  │        ^^^^^^^
  │

 = help: remove `Ast` from this declaration name
"#,
        );
    }

    /// Accept a declaration whose complete name equals its module.
    #[test]
    fn test_accepts_exact_module_name() {
        let session = TestSession::dir_path(
            &REDUNDANT_OWNER_NAME,
            "user.ds",
            r#"
class User {}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Report owner repetitions on instance members and enum variants.
    #[test]
    fn test_reports_type_owner_names() {
        let session = TestSession::dir(
            &REDUNDANT_OWNER_NAME,
            r#"
struct User {
    userName: string;
}

enum Status {
    StatusPending,
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[redundant-owner-name]: member `userName` repeats owner name `User`
 ──▶ main.ds:2:5
  │
1 │ struct User {
2 │     userName: string;
  │     ^^^^^^^^
3 │ }
4 │
  │

 = help: remove `user` from this member name
warning[redundant-owner-name]: variant `StatusPending` repeats owner name `Status`
 ──▶ main.ds:6:5
  │
4 │
5 │ enum Status {
6 │     StatusPending,
  │     ^^^^^^^^^^^^^
7 │ }
  │

 = help: remove `Status` from this variant name
"#,
        );
    }

    /// Report aliases and extension members repeating their resolved owners.
    #[test]
    fn test_reports_export_and_extension_owner_names() {
        let session = TestSession::dir_path(
            &REDUNDANT_OWNER_NAME,
            "ast.ds",
            r#"
struct Node {}

extension of Node {
    nodeKind(): string {
        return "node";
    }
}

export { Node as AstNode };
"#,
        );

        session.assert_diagnostics(
            r#"
warning[redundant-owner-name]: member `nodeKind` repeats owner name `Node`
 ──▶ ast.ds:4:5
  │
2 │
3 │ extension of Node {
4 │     nodeKind(): string {
  │     ^^^^^^^^
5 │         return "node";
6 │     }
  │

 = help: remove `node` from this member name
warning[redundant-owner-name]: export `AstNode` repeats owner name `Ast`
 ──▶ ast.ds:9:18
  │
7 │ }
8 │
9 │ export { Node as AstNode };
  │                  ^^^^^^^
  │

 = help: remove `Ast` from this export name
"#,
        );
    }

    /// Accept interface-imposed names on their concrete implementation.
    #[test]
    fn test_accepts_interface_member_name() {
        let session = TestSession::dir(
            &REDUNDANT_OWNER_NAME,
            r#"
newtype interface UserIdentity {
    userName(): string;
}

struct User {}

extension of User implements UserIdentity {
    userName(): string {
        return "user";
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
