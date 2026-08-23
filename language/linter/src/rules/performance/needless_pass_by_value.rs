use destack_artifact::DiagnosticAnchor;
use destack_core::BitSet;
use destack_mir as mir;
use destack_repository::ProviderError;

use crate::rules::declare_lint;
use crate::{Lint, LintOutput, LintResult, MirModule};

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
struct Packet {
    code: int32;
}

function packetCode(packet: Packet): int32 {
    return packet.code;
}
"#,
            accepted: r#"
struct Packet {
    code: int32;
}

function packetCode(packet: &readonly Packet): int32 {
    return packet.code;
}
"#,
        },
        category: Performance,
        level: Warning,
        fixable: None,
        check: MirModule(check),
    }
}

/// Report move-only parameters whose bodies never consume their ownership.
fn check(module: &mut MirModule, lint: &Lint) -> LintResult {
    let tree = &module.lowered.tree;
    let mut output = LintOutput::default();

    // inspect every defined function
    for (function_id, function) in tree.iter_nodes::<mir::Function>() {
        // skip imported functions
        if !function.is_defined() {
            continue;
        }

        let definitions = module.analyses.definition(function_id, tree);
        let consumed = consumed_parameters(function, &definitions, tree)?;
        let spans = tree.function_parameter_spans(function_id);

        // report retained move-only parameters
        for (index, parameter) in function.parameters.iter().enumerate() {
            // skip copyable and consumed values
            if tree.get(parameter.ty).copy(tree).is_yes() || consumed.contains(index) {
                continue;
            }

            let span = spans.get(index).ok_or_else(|| {
                ProviderError::internal(format!(
                    "MIR function {:?} has no span for parameter {index}",
                    function_id
                ))
            })?;

            let diagnostic = lint
                .diagnostic(
                    "move-only parameter is never consumed",
                    DiagnosticAnchor::Span(span.span),
                )
                .help("accept a readonly borrow unless the function must take ownership");
            output.report(diagnostic);
        }
    }

    Ok(output)
}

/// Return the parameters moved along any reachable MIR path.
fn consumed_parameters(
    function: &mir::Function,
    definitions: &mir::DefinitionTable,
    tree: &mir::Tree,
) -> Result<BitSet, ProviderError> {
    let mut pending = Vec::new();
    let entry = function
        .entry()
        .ok_or_else(|| ProviderError::internal("defined MIR function has no entry block"))?;

    // collect whole-value and projected moves from reachable blocks
    for block in mir::collect_reachable_blocks(function, tree, entry) {
        let block = tree.get(block);
        for instruction in &block.instructions {
            let instruction = tree.get(*instruction);
            pending.extend(instruction.consumes(tree));

            if let Some(source) = projection_move_source(instruction, function, tree) {
                pending.push(source);
            }
        }

        pending.extend(tree.get(block.terminator).consumes(tree));
    }

    let mut consumed = BitSet::new(function.parameters.len());
    let mut visited = BitSet::new(function.value_capacity());

    // trace consumed block parameters back to function parameters
    while let Some(value) = pending.pop() {
        if !visited.insert(value.id() as usize) {
            continue;
        }

        // mark parameters and trace control-flow forwarding
        match definitions.definition(value) {
            Some(mir::ValueDefinition::FunctionParameter(index)) => {
                consumed.insert(index);
            }
            Some(mir::ValueDefinition::BlockParameter { .. }) => {
                pending.extend(definitions.block_parameter_values(value));
            }
            Some(mir::ValueDefinition::Instruction { .. }) => {}
            None => {
                return Err(ProviderError::internal(format!(
                    "consumed MIR value {value:?} has no definition"
                )));
            }
        }
    }

    Ok(consumed)
}

/// Return the aggregate consumed by one projected move.
fn projection_move_source(
    instruction: &mir::Instruction,
    function: &mir::Function,
    tree: &mir::Tree,
) -> Option<mir::Value> {
    let (source, destination) = match instruction {
        mir::Instruction::FieldGet {
            aggregate,
            destination,
            ..
        }
        | mir::Instruction::ElementGet {
            aggregate,
            destination,
            ..
        } => (*aggregate, *destination),
        mir::Instruction::VariantPayload {
            variant,
            destination,
            ..
        } => (*variant, *destination),
        _ => return None,
    };

    // require ownership for a move-only projection
    let destination_type = function.expect_value_type(destination);

    tree.get(destination_type)
        .copy(tree)
        .is_no()
        .then_some(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::TestSession;

    /// Report a move-only parameter used only through shared reads.
    #[test]
    fn test_reports_read_only_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

function packetCode(v0: Packet): int32 {
entry(v0: Packet):
    v1: int32 = field.get v0, 0
    return v1
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.mir:3:21
  │
1 │ type Packet { code: int32; }
2 │
3 │ function packetCode(v0: Packet): int32 {
  │                     ^^^^^^^^^^
4 │ entry(v0: Packet):
5 │     v1: int32 = field.get v0, 0
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Follow a parameter forwarded through a control-flow edge.
    #[test]
    fn test_reports_forwarded_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

function packetCode(v0: Packet): int32 {
entry(v0: Packet):
    jump read(v0)

read(v1: Packet):
    v2: int32 = field.get v1, 0
    return v2
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.mir:3:21
  │
1 │ type Packet { code: int32; }
2 │
3 │ function packetCode(v0: Packet): int32 {
  │                     ^^^^^^^^^^
4 │ entry(v0: Packet):
5 │     jump read(v0)
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Ignore ownership transfers in unreachable blocks.
    #[test]
    fn test_reports_parameter_consumed_only_in_unreachable_block() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

external function consume(Packet): void

function packetCode(v0: Packet): int32 {
entry(v0: Packet):
    v1: int32 = field.get v0, 0
    return v1

dead:
    call consume(v0): (Packet) => void
    v2: int32 = 0
    return v2
}
"#,
        );

        session.assert_diagnostics(
            r#"
warning[needless-pass-by-value]: move-only parameter is never consumed
 ──▶ main.mir:5:21
  │
3 │ external function consume(Packet): void
4 │
5 │ function packetCode(v0: Packet): int32 {
  │                     ^^^^^^^^^^
6 │ entry(v0: Packet):
7 │     v1: int32 = field.get v0, 0
  │

 = help: accept a readonly borrow unless the function must take ownership
"#,
        );
    }

    /// Accept a parameter returned from the function.
    #[test]
    fn test_accepts_consumed_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

function keep(v0: Packet): Packet {
entry(v0: Packet):
    return v0
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter whose move-only field is returned.
    #[test]
    fn test_accepts_consumed_field() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Data { value: int32; }
type Packet { data: Data; }

function takeData(v0: Packet): Data {
entry(v0: Packet):
    v1: Data = field.get v0, 0
    return v1
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter consumed while replacing one field.
    #[test]
    fn test_accepts_mutated_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

function setCode(v0: Packet, v1: int32): Packet {
entry(v0: Packet, v1: int32):
    v2: Packet = field.set v0, 0, v1
    return v2
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a parameter transferred into another call.
    #[test]
    fn test_accepts_call_argument() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

external function consume(Packet): void

function forward(v0: Packet): void {
entry(v0: Packet):
    call consume(v0): (Packet) => void
    return
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a borrowed parameter.
    #[test]
    fn test_accepts_borrowed_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
type Packet { code: int32; }

function packetCode(v0: ref<Packet, borrowed, readonly>): int32 {
entry(v0: ref<Packet, borrowed, readonly>):
    v1: ref<int32, borrowed, readonly> = field.address v0, 0
    v2: int32 = load v1
    return v2
}
"#,
        );

        session.assert_no_diagnostics();
    }

    /// Accept a freely copyable value parameter.
    #[test]
    fn test_accepts_copy_parameter() {
        let session = TestSession::mir(
            &NEEDLESS_PASS_BY_VALUE,
            r#"
function increment(v0: int32): int32 {
entry(v0: int32):
    v1: int32 = 1
    v2: int32 = add v0, v1
    return v2
}
"#,
        );

        session.assert_no_diagnostics();
    }
}
