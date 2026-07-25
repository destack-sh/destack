use destack_mir as mir;

use crate::tests::TestProgram;
use crate::verify::VerifyError;

const DROP_MIR: &str = r#"
type Item { }

external function effect(): void

function dropItem(v0: ref<Item, borrowed, exclusive>): void {
entry(v0: ref<Item, borrowed, exclusive>):
    call effect(): () => void
    return
}
"#;

/// Reject a Drop hook that consumes its receiver.
#[test]
fn test_reject_consuming_drop_hook() {
    let mut program = TestProgram::mir(
        r#"
type Item { }

function dropItem(v0: Item): void {
entry(v0: Item):
    return
}
"#,
    );
    program.mark_drop_hook("Item", "dropItem");

    let diagnostics = program.run_drop_hooks();

    assert!(matches!(diagnostics.as_slice(), [diagnostic]
        if matches!(diagnostic.diagnostic(), VerifyError::InvalidDropSignature { .. })));
}

/// Reject a Drop hook that may panic.
#[test]
fn test_reject_panicking_drop_hook() {
    let mut program = TestProgram::mir(DROP_MIR);
    program.mark_drop_hook("Item", "dropItem");
    let function = program.function_by_name("effect");
    program.lowered.effects.function_mut(function).behavior =
        mir::FunctionBehavior::none().with_panic();

    let diagnostics = program.run_drop_hooks();

    assert!(matches!(diagnostics.as_slice(), [diagnostic]
        if matches!(diagnostic.diagnostic(), VerifyError::DropMayPanic { .. })));
}

/// Reject a Drop hook that may allocate.
#[test]
fn test_reject_allocating_drop_hook() {
    let mut program = TestProgram::mir(DROP_MIR);
    program.mark_drop_hook("Item", "dropItem");
    let function = program.function_by_name("effect");
    program.lowered.effects.function_mut(function).behavior =
        mir::FunctionBehavior::none().with_allocates();

    let diagnostics = program.run_drop_hooks();

    assert!(matches!(diagnostics.as_slice(), [diagnostic]
        if matches!(diagnostic.diagnostic(), VerifyError::DropMayAllocate { .. })));
}

/// Reject a Drop hook that may observe entropy.
#[test]
fn test_reject_nondeterministic_drop_hook() {
    let mut program = TestProgram::mir(DROP_MIR);
    program.mark_drop_hook("Item", "dropItem");
    let function = program.function_by_name("effect");
    let behavior = &mut program.lowered.effects.function_mut(function).behavior;
    *behavior = mir::FunctionBehavior::none();
    behavior.determinism = mir::Determinism::NonDeterministic;

    let diagnostics = program.run_drop_hooks();

    assert!(matches!(diagnostics.as_slice(), [diagnostic]
        if matches!(diagnostic.diagnostic(), VerifyError::DropMayObserveEntropy { .. })));
}
