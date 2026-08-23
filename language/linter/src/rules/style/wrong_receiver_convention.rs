use destack_dir as dir;

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
        category: Style,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// The authored ownership behavior of one receiver parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReceiverForm {
    /// A receiver consumed by value.
    Value,
    /// A receiver observed through readonly borrowing.
    Readonly,
    /// A receiver borrowed with mutable or exclusive access.
    Mutable,
}

/// Report method names whose receiver contradicts the standard convention.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let view = module.view();
    let mut output = LintOutput::default();

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
        if signature.role.is_some() || signature.is_override {
            continue;
        }
        let name = module.dir.strings.get(*name);
        let receiver = signature
            .this_parameter
            .and_then(|parameter| receiver_form(parameter, module));

        // compare recognized prefixes with their receiver requirements
        let expected = if has_prefix(name, "from") && !*is_static {
            Some("be static")
        } else if has_prefix(name, "into") && receiver != Some(ReceiverForm::Value) {
            Some("consume its receiver")
        } else if has_prefix(name, "as")
            && !matches!(
                receiver,
                Some(ReceiverForm::Readonly | ReceiverForm::Mutable)
            )
        {
            Some("borrow its receiver")
        } else if has_suffix_pair(name, "to", "Mut") && receiver != Some(ReceiverForm::Mutable) {
            Some("borrow its receiver with mutable access")
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

/// Classify the source form of one explicit or shorthand receiver.
fn receiver_form(
    parameter: dir::LocalNodeId<dir::Parameter>,
    module: &DirModule<'_>,
) -> Option<ReceiverForm> {
    let declared_type = module.view().get(parameter).declared_type()?;

    match module.view().get(declared_type) {
        dir::TypeExpression::This | dir::TypeExpression::OwnedOf { .. } => {
            Some(ReceiverForm::Value)
        }
        dir::TypeExpression::BorrowedOf { mutability, .. }
            if mutability.map(dir::Mutability::access) == Some(dir::Access::Readonly) =>
        {
            Some(ReceiverForm::Readonly)
        }
        dir::TypeExpression::BorrowedOf { .. } => Some(ReceiverForm::Mutable),
        _ => None,
    }
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

    toBytesMut(&readonly this): &exclusive [uint8] {
        todo("Buffer.toBytesMut")
    }
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[wrong-receiver-convention]: intoBytes should consume its receiver
 ──▶ main.ds:2:5
  │
1 │ struct Buffer {
2 │     intoBytes(&readonly this): [uint8] {
  │     ^^^^^^^^^
3 │         todo("Buffer.intoBytes")
4 │     }
  │

warning[wrong-receiver-convention]: fromBytes should be static
 ──▶ main.ds:6:5
  │
4 │     }
5 │
6 │     fromBytes(&readonly this, bytes: [uint8]): Buffer {
  │     ^^^^^^^^^
7 │         todo("Buffer.fromBytes")
8 │     }
  │

warning[wrong-receiver-convention]: asBytes should borrow its receiver
  ──▶ main.ds:10:5
   │
 8 │     }
 9 │
10 │     asBytes(this): [uint8] {
   │     ^^^^^^^
11 │         todo("Buffer.asBytes")
12 │     }
   │

warning[wrong-receiver-convention]: toBytesMut should borrow its receiver with mutable access
  ──▶ main.ds:14:5
   │
12 │     }
13 │
14 │     toBytesMut(&readonly this): &exclusive [uint8] {
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

    toBytesMut(&exclusive this): &exclusive [uint8] {
        todo("Buffer.toBytesMut")
    }
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
