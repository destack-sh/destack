use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_generator_functions_and_yields() {
    let session = TestSession::single(
        r#"
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
function* count(limit: int32): Generator<int32, void, void> {
    for (let value: int32 = 0; value < limit; value += 1) {
        yield value;
    }
}

=== dir ===
function* count(limit: int32): Generator<int32, void, void> {
/// @type.symbol symbol=count type=(int32) => *Generator<int32, void, void>
/// @generic.instance id="Generator<int32, void, void>" template=async.generator.Generator arguments=(int32, void, void)
/// @generic.instance id="async.generator.GeneratorPhase<int32, void, void>" template=async.generator.GeneratorPhase arguments=(int32, void, void)
/// @generic.instance id="async.generator.GeneratorRequest<void, void>" template=async.generator.GeneratorRequest arguments=(void, void)
/// @generic.instance id="async.generator.GeneratorRequested<void, void>" template=async.generator.GeneratorRequested arguments=(void, void)
/// @generic.instance id="async.generator.GeneratorResult<int32, void>" template=async.generator.GeneratorResult arguments=(int32, void)
/// @generic.instance id="async.generator.GeneratorState<int32, void, void>" template=async.generator.GeneratorState arguments=(int32, void, void)
/// @generic.instance id="iter.iterator.IteratorResult<int32, void>" template=iter.iterator.IteratorResult arguments=(int32, void)
/// @generic.instance id="memory.cell.cell.Cell<async.generator.GeneratorPhase<int32, void, void>>" template=memory.cell.cell.Cell arguments=(async.generator.GeneratorPhase<int32, void, void>)
/// @generic.instance id="memory.cell.cell.UnsafeCell<async.generator.GeneratorPhase<int32, void, void>>" template=memory.cell.cell.UnsafeCell arguments=(async.generator.GeneratorPhase<int32, void, void>)
/// @generic.instance id=async.generator.GeneratorNext<void> template=async.generator.GeneratorNext arguments=(void)
/// @generic.instance id=async.generator.GeneratorReturn<void> template=async.generator.GeneratorReturn arguments=(void)
/// @generic.instance id=async.generator.GeneratorReturned<void> template=async.generator.GeneratorReturned arguments=(void)
/// @generic.instance id=async.generator.GeneratorYielded<int32> template=async.generator.GeneratorYielded arguments=(int32)
/// @generic.instance id=iter.iterator.IteratorReturn<void> template=iter.iterator.IteratorReturn arguments=(void)
/// @generic.instance id=iter.iterator.IteratorYield<int32> template=iter.iterator.IteratorYield arguments=(int32)
/// @type.symbol symbol=count.limit source="limit: int32" type=int32
/// @resolution.name source=Generator target=async.generator.Generator

    for (let value: int32 = 0; value < limit; value += 1) {
    /// @type.symbol symbol=count.value source=value type=int32
    /// @resolution.pattern source=value kind=binding target=count.value
    /// @resolution.name source=value target=count.value
    /// @resolution.operator source="value < limit" type=boolean operator="<" kind=builtin operands=[value as int32 families=(integer), limit as int32 families=(integer)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=count.value
    /// @resolution.name source=limit target=count.limit
    /// @resolution.place source=limit placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=limit root=count.limit
    /// @resolution.name source=value target=count.value
    /// @resolution.operator source="value += 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.assignment source=value read=binding(count.value) write=binding(count.value) type=int32
    /// @resolution.access source=value root=count.value

        yield value;
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=count.value

    }
}
"#);
}
