use tspp_core::FxIndexSet;
use tspp_dir as dir;
use tspp_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Require receiver forms matching the method's name convention.
    pub WRONG_RECEIVER_CONVENTION {
        id: "wrong-receiver-convention",
        summary: "Require receiver forms matching the method's name convention",
        explanation: r#"
Method prefixes communicate whether an operation borrows, mutates, or consumes its receiver.
Instead, `as` methods SHOULD borrow, `into` methods SHOULD consume, `from` methods SHOULD be static, and `toMut` methods SHOULD borrow with mutable access.
"#,
        example: {
            reported: r#"
struct Buffer {
    intoBytes(&readonly this): [uint8] {
        todo("Buffer.intoBytes")
    }
}
"#,
            accepted: r#"
struct Buffer {
    intoBytes(this): [uint8] {
        todo("Buffer.intoBytes")
    }
}
"#,
        },
        provenance: [Clippy("wrong_self_convention")],
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The ownership behavior of one receiver parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiverForm {
    /// A receiver consumed by value.
    Value,
    /// A receiver observed through readonly borrowing.
    Readonly,
    /// A receiver borrowed with mutable access.
    Mutable,
    /// A receiver borrowed with generic access.
    Borrowed,
}

/// Report method names whose receiver contradicts the standard convention.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let implementations = module
        .members
        .member_conformances()
        .map(|conformance| conformance.member)
        .collect::<FxIndexSet<_>>();
    let mut output = LintOutput::default();

    // collect the members of every value declaration, whose bare `this` is a value
    let mut value_members = FxIndexSet::default();
    for (_, declaration) in view.iter_nodes::<dir::Declaration>() {
        let members = match declaration {
            dir::Declaration::Struct(declaration) => &declaration.members,
            dir::Declaration::Enum(declaration) => &declaration.members,
            dir::Declaration::Global(_)
            | dir::Declaration::Module(_)
            | dir::Declaration::Type(_)
            | dir::Declaration::Class(_)
            | dir::Declaration::Interface(_)
            | dir::Declaration::Extension(_)
            | dir::Declaration::Function(_) => continue,
        };
        value_members.extend(members.iter().copied());
    }

    // inspect authored nominal methods with identifier names
    for (node, member) in view.iter_nodes::<dir::Member>() {
        let dir::Member::Method {
            name: Some(dir::Name::Identifier(name)),
            signature,
            is_static,
            ..
        } = member
        else {
            continue;
        };
        let symbol = module.declaration_symbol(node)?;
        if signature.role.is_some() || signature.is_override || implementations.contains(&symbol) {
            continue;
        }
        let name = module.dir.strings.get(*name);

        // compare recognized prefixes with their receiver requirements
        let expected = if has_prefix(name, "from") && !*is_static {
            Some("be static")
        } else if *is_static {
            None
        } else if has_prefix(name, "into") {
            (receiver_form(node, signature, module, &value_members)? != Some(ReceiverForm::Value))
                .then_some("consume its receiver")
        } else if has_prefix(name, "as") {
            (!matches!(
                receiver_form(node, signature, module, &value_members)?,
                Some(ReceiverForm::Readonly | ReceiverForm::Mutable | ReceiverForm::Borrowed)
            ))
            .then_some("borrow its receiver")
        } else if has_suffix_pair(name, "to", "Mut") {
            (receiver_form(node, signature, module, &value_members)? != Some(ReceiverForm::Mutable))
                .then_some("borrow its receiver with mutable access")
        } else {
            None
        };
        let Some(expected) = expected else {
            continue;
        };

        let span = module.main_span(node.into_any())?;
        let message = format!("{name} should {expected}");
        output.report(lint.diagnostic(message, span));
    }

    Ok(output)
}

/// Classify the form of one explicit or implicit receiver.
fn receiver_form(
    member: dir::LocalNodeId<dir::Member>,
    signature: &dir::FunctionSignature,
    module: &DirModule<'_>,
    value_members: &FxIndexSet<dir::LocalNodeId<dir::Member>>,
) -> Result<Option<ReceiverForm>, ProviderError> {
    // read the explicit receiver type or the implicit binding
    let type_id = if let Some(parameter) = signature.this_parameter {
        let symbol = module.declaration_symbol(parameter)?;
        module
            .types
            .get_symbol_type_id(symbol)
            .ok_or_else(|| ProviderError::internal(format!("receiver {symbol:?} has no type")))?
    } else {
        let global = member.into_global_any(module.id);
        let symbol = module
            .bindings
            .implicit_receiver_symbol(global)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "instance method {global:?} has no implicit receiver"
                ))
            })?;
        let symbol = symbol.into_global(module.id);
        module.types.get_symbol_type_id(symbol).ok_or_else(|| {
            ProviderError::internal(format!("implicit receiver {symbol:?} has no type"))
        })?
    };

    // classify a bare `this` by its declaration, a value declaration owning its receiver
    if matches!(module.dir.get_type(type_id)?, dir::Type::This) {
        return Ok(value_members
            .contains(&member)
            .then_some(ReceiverForm::Value));
    }

    // classify reduced ownership and access
    let form = match module.dir.default_ownership(type_id)? {
        Some(dir::Ownership::Owned) => ReceiverForm::Value,
        Some(dir::Ownership::Borrowed) => match module.dir.borrow_access(type_id)? {
            Some(Some(dir::Access::Readonly | dir::Access::Immutable)) => ReceiverForm::Readonly,
            Some(Some(dir::Access::Mutable | dir::Access::Exclusive)) => ReceiverForm::Mutable,
            Some(None) => ReceiverForm::Borrowed,
            None => {
                return Err(ProviderError::internal(format!(
                    "borrowed receiver {type_id:?} has no borrow form"
                )));
            }
        },
        Some(dir::Ownership::Managed | dir::Ownership::Raw) | None => return Ok(None),
    };

    Ok(Some(form))
}

/// Return whether a camelCase name starts with one complete prefix word.
fn has_prefix(name: &str, prefix: &str) -> bool {
    let Some(remainder) = name.strip_prefix(prefix) else {
        return false;
    };

    remainder.is_empty() || remainder.starts_with(char::is_uppercase)
}

/// Return whether a camelCase name begins and ends with the selected convention words.
fn has_suffix_pair(name: &str, prefix: &str, suffix: &str) -> bool {
    has_prefix(name, prefix) && name.ends_with(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report consuming and constructing names with contradictory receivers.
    #[test]
    fn test_reports_wrong_receiver_forms() {
        let session = TestSession::dir(
            &WRONG_RECEIVER_CONVENTION,
            r#"
struct Buffer {
    intoBytes(&readonly this): [uint8] {
        todo("Buffer.intoBytes")
    }

    fromBytes(&readonly this, bytes: [uint8]): Buffer {
        todo("Buffer.fromBytes")
    }

    asBytes(this): [uint8] {
        todo("Buffer.asBytes")
    }

    toBytesMut(&readonly this): &[uint8] {
        todo("Buffer.toBytesMut")
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[wrong-receiver-convention]: intoBytes should consume its receiver
 ──▶ main.tspp:2:5
  │
1 │ struct Buffer {
2 │     intoBytes(&readonly this): [uint8] {
  │     ^^^^^^^^^
3 │         todo("Buffer.intoBytes")
4 │     }
  │

warning[wrong-receiver-convention]: fromBytes should be static
 ──▶ main.tspp:6:5
  │
4 │     }
5 │
6 │     fromBytes(&readonly this, bytes: [uint8]): Buffer {
  │     ^^^^^^^^^
7 │         todo("Buffer.fromBytes")
8 │     }
  │

warning[wrong-receiver-convention]: asBytes should borrow its receiver
  ──▶ main.tspp:10:5
   │
 8 │     }
 9 │
10 │     asBytes(this): [uint8] {
   │     ^^^^^^^
11 │         todo("Buffer.asBytes")
12 │     }
   │

warning[wrong-receiver-convention]: toBytesMut should borrow its receiver with mutable access
  ──▶ main.tspp:14:5
   │
12 │     }
13 │
14 │     toBytesMut(&readonly this): &[uint8] {
   │     ^^^^^^^^^^
15 │         todo("Buffer.toBytesMut")
16 │     }
   │
"#,
        );
    }

    /// Accept borrowing, consuming, and static construction conventions.
    #[test]
    fn test_accepts_receiver_conventions() {
        let session = TestSession::dir(
            &WRONG_RECEIVER_CONVENTION,
            r#"
struct Buffer {
    asBytes(&readonly this): &[uint8] {
        todo("Buffer.asBytes")
    }

    intoBytes(this): [uint8] {
        todo("Buffer.intoBytes")
    }

    static fromBytes(bytes: [uint8]): Buffer {
        todo("Buffer.fromBytes")
    }

    toBytesMut(&this): &[uint8] {
        todo("Buffer.toBytesMut")
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept static and conformance-imposed receiver forms.
    #[test]
    fn test_accepts_nonlocal_receiver_forms() {
        let session = TestSession::dir(
            &WRONG_RECEIVER_CONVENTION,
            r#"
interface Into<T> {
    into(this): T;
}

struct Buffer {
    static asInt(bits: int32, value: int32): int32 {
        return value;
    }
}

extension of Buffer implements Into<int32> {
    into(): int32 {
        return 0;
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an access-generic borrowed receiver.
    #[test]
    fn test_accepts_access_generic_receiver() {
        let session = TestSession::dir(
            &WRONG_RECEIVER_CONVENTION,
            r#"
import { Access, WithAccess } from "tspp:memory";

struct Buffer {}

extension<const A: Access> of Buffer {
    as(this: WithAccess<&Buffer, A>): WithAccess<&uint8, A> {
        todo("Buffer.as")
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept an implicit receiver synthesized as a readonly borrow.
    #[test]
    fn test_accepts_implicit_receiver() {
        let session = TestSession::dir(
            &WRONG_RECEIVER_CONVENTION,
            r#"
struct Buffer {
    asPointer(): *uint8 {
        todo("Buffer.asPointer")
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
