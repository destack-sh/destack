use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{DirModule, Lint, LintOutput, LintResult};

declare_lint! {
    /// Disallow passing parameters by value when they are never consumed or mutated.
    pub NEEDLESS_PASS_BY_VALUE {
        id: "needless-pass-by-value",
        summary: "Disallow passing parameters by value when they are never consumed or mutated",
        explanation: r#"
Taking ownership of a move-only parameter prevents callers from retaining it when the body only reads the value.
Instead, you SHOULD accept a readonly borrow when the function neither mutates nor consumes the parameter.
Keep ownership when transferring the value is part of the callable's behavior.
"#,
        example: {
            reported: r#"
class Packet {
    code: int32;
}

function packetCode(packet: ^Packet): int32 {
    return packet.code;
}
"#,
            accepted: r#"
class Packet {
    code: int32;
}

function packetCode(packet: &readonly Packet): int32 {
    return packet.code;
}
"#,
        },
        provenance: [Clippy("needless_pass_by_value")],
        category: Performance,
        level: Warning,
        fixable: None,
        check: DirModule(check),
    }
}

/// Report move-only parameters whose uses accept readonly borrowing.
fn check(module: &DirModule<'_>, lint: &Lint) -> LintResult {
    let mut output = LintOutput::default();

    // aggregate each local binding's uses once
    let mut uses = FxIndexMap::<_, dir::BindingUse>::default();
    for occurrence in module.flows.binding_occurrences() {
        if occurrence.symbol.module_id == module.id {
            *uses.entry(occurrence.symbol.local_id).or_default() |= occurrence.uses;
        }
    }

    // inspect directly declared parameters of implemented callables
    for (declaration, symbol) in module.bindings.declaration_symbols() {
        let Ok(parameter) = declaration.local_id.try_into_typed::<dir::Parameter>() else {
            continue;
        };
        let Some(callable) = module.enclosing_callable(parameter.into_any()) else {
            continue;
        };
        if module.callable_body(callable).is_none()
            || module.is_statically_absent(parameter.into_any())
        {
            continue;
        }

        // preserve ownership, mutation, captures, and exclusion required by checked uses
        if uses.get(&symbol).is_some_and(|uses| {
            uses.may_mutate()
                || uses.contains(dir::BindingUse::MUTABLE)
                || uses.contains(dir::BindingUse::MOVE)
                || uses.contains(dir::BindingUse::CAPTURE)
                || uses.contains(dir::BindingUse::EXCLUSIVE)
        }) {
            continue;
        }

        // report only owned types that sema proved cannot copy
        let symbol = symbol.into_global(module.id);
        let ty = module.types.get_symbol_type_id(symbol).ok_or_else(|| {
            ProviderError::internal(format!("checked parameter {symbol:?} has no type"))
        })?;
        if module.dir.default_ownership(ty)? != Some(dir::Ownership::Owned)
            || module.dir.copies(ty)? != Some(false)
        {
            continue;
        }

        // report the complete parameter declaration
        let span = module.main_span(parameter.into_any())?;
        let diagnostic = lint
            .diagnostic("move-only parameter is never consumed", span)
            .help("accept a readonly borrow unless the function must take ownership");
        output.report(diagnostic);
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report an owned parameter used only for readonly field access.
    #[test]
    fn test_reports_read_only_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function packetCode(v0: ^Packet): int32 {
    return v0.code;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.ds:3:21
  │
1 │ class Packet { code: int32; }
2 │
3 │ function packetCode(v0: ^Packet): int32 {
  │                     ^^^^^^^^^^^
4 │     return v0.code;
5 │ }
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Report an owned parameter read on both branches.
    #[test]
    fn test_reports_forwarded_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function packetCode(v0: ^Packet, flag: boolean): int32 {
    if (flag) { return v0.code; }
    return v0.code;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.ds:3:21
  │
1 │ class Packet { code: int32; }
2 │
3 │ function packetCode(v0: ^Packet, flag: boolean): int32 {
  │                     ^^^^^^^^^^^
4 │     if (flag) { return v0.code; }
5 │     return v0.code;
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Ignore ownership transfers after an unconditional return.
    #[test]
    fn test_reports_parameter_consumed_only_in_unreachable_block() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function packetCode(v0: ^Packet): int32 {
    return v0.code;
    const moved: ^_ = v0;
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.ds:3:21
  │
1 │ class Packet { code: int32; }
2 │
3 │ function packetCode(v0: ^Packet): int32 {
  │                     ^^^^^^^^^^^
4 │     return v0.code;
5 │     const moved: ^_ = v0;
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Accept an owned parameter returned from the function.
    #[test]
    fn test_accepts_consumed_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function keep(v0: ^Packet): ^Packet { return v0; }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter whose owned field is returned.
    #[test]
    fn test_accepts_consumed_field() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Data { value: int32; }
struct Packet { data: ^Data; }

function takeData(v0: Packet): ^Data { return v0.data; }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept mutation of an owned parameter.
    #[test]
    fn test_accepts_mutated_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function setCode(v0: ^Packet, v1: int32): void { v0.code = v1; }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter transferred into another call.
    #[test]
    fn test_accepts_call_argument() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }
function consume(value: ^Packet): ^Packet { return value; }

function forward(v0: ^Packet): ^Packet { return consume(v0); }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a borrowed parameter.
    #[test]
    fn test_accepts_borrowed_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }

function packetCode(v0: &readonly Packet): int32 { return v0.code; }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a freely copyable value parameter.
    #[test]
    fn test_accepts_copy_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
function increment(v0: int32): int32 { return v0 + 1; }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Preserve exclusion required by a readonly call.
    #[test]
    fn test_accepts_exclusive_read() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
class Packet { code: int32; }
function read(value: &readonly exclusive Packet): int32 { return value.code; }

function packetCode(v0: ^Packet): int32 { return read(&readonly exclusive v0); }
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Leave an undecided generic Copy requirement alone.
    #[test]
    fn test_accepts_open_parameter() {
        let session = TestSession::dir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
function keep<T>(value: ^T): void {}
"#,
        );

        session.assert_no_diagnostics();
    }
}
