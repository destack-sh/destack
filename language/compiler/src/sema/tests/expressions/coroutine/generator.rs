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
/// @resolution.call parameters=(^Function<(GeneratorProducer<Generator.create.Y, Generator.create.R, Generator.create.N>,), Generator.create.R, "once">) arguments=(write as ^Function<(GeneratorProducer<Generator.create.Y, Generator.create.R, Generator.create.N>,), Generator.create.R, "once">) return=Generator<Generator.create.Y, Generator.create.R, Generator.create.N> kind=symbol target=Generator.create instance="Generator.create<int32, void, void>"
/// @generic.instantiation id="Generator.create<int32, void, void>" template=Generator.create arguments=(int32, void, void)
/// @generic.instance id="Cell<Fiber | undefined>" template=Cell arguments=(Fiber | undefined)
/// @generic.instance id="Cell<GeneratorPhase<int32, void, void>>" template=Cell arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id="Function<(GeneratorProducer<int32, void, void>,), void, \"once\">" template=Function arguments=((GeneratorProducer<int32, void, void>,), void, "once")
/// @generic.instance id="Generator.create<int32, void, void>" template=Generator.create arguments=(int32, void, void)
/// @generic.instance id="Generator.symbol37<int32, void, void, \"local\">" template=Generator.symbol37 arguments=(int32, void, void, "local")
/// @generic.instance id="Generator<int32, void, void, \"local\">" template=Generator arguments=(int32, void, void, "local")
/// @generic.instance id="Generator<int32, void, void>" template=Generator arguments=(int32, void, void)
/// @generic.instance id="GeneratorPhase<int32, void, void>" template=GeneratorPhase arguments=(int32, void, void)
/// @generic.instance id="GeneratorRequested<void, void>" template=GeneratorRequested arguments=(void, void)
/// @generic.instance id="GeneratorResult<int32, void>" template=GeneratorResult arguments=(int32, void)
/// @generic.instance id="GeneratorState.advance<int32, void, void>" template=GeneratorState.advance arguments=(int32, void, void)
/// @generic.instance id="GeneratorState.attach<int32, void, void>" template=GeneratorState.attach arguments=(int32, void, void)
/// @generic.instance id="GeneratorState.begin<int32, void, void>" template=GeneratorState.begin arguments=(int32, void, void)
/// @generic.instance id="GeneratorState.complete<int32, void, void>" template=GeneratorState.complete arguments=(int32, void, void)
/// @generic.instance id="GeneratorState.symbol85<int32, void, void, \"local\">" template=GeneratorState.symbol85 arguments=(int32, void, void, "local")
/// @generic.instance id="GeneratorState.takeResult<int32, void, void>" template=GeneratorState.takeResult arguments=(int32, void, void)
/// @generic.instance id="GeneratorState<int32, void, void, \"local\">" template=GeneratorState arguments=(int32, void, void, "local")
/// @generic.instance id="GeneratorState<int32, void, void>" template=GeneratorState arguments=(int32, void, void)
/// @generic.instance id="IteratorResult<int32, void>" template=IteratorResult arguments=(int32, void)
/// @generic.instance id="UnsafeCell<Fiber | undefined>" template=UnsafeCell arguments=(Fiber | undefined)
/// @generic.instance id="UnsafeCell<GeneratorPhase<int32, void, void>>" template=UnsafeCell arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id="get#2<Fiber | undefined>" template=get#2 arguments=(Fiber | undefined)
/// @generic.instance id="new#1<Fiber | undefined>" template=new#1 arguments=(Fiber | undefined)
/// @generic.instance id="new#1<GeneratorPhase<int32, void, void>>" template=new#1 arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id="new#2<Fiber | undefined>" template=new#2 arguments=(Fiber | undefined)
/// @generic.instance id="new#2<GeneratorPhase<int32, void, void>>" template=new#2 arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id="replace<Fiber | undefined>" template=replace arguments=(Fiber | undefined)
/// @generic.instance id="replace<GeneratorPhase<int32, void, void>>" template=replace arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id="set<GeneratorPhase<int32, void, void>>" template=set arguments=(GeneratorPhase<int32, void, void>)
/// @generic.instance id=GeneratorReturned<void> template=GeneratorReturned arguments=(void)
/// @generic.instance id=GeneratorYielded<int32> template=GeneratorYielded arguments=(int32)
/// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
/// @generic.instance id=IteratorYield<int32> template=IteratorYield arguments=(int32)
/// @type.symbol symbol=count.limit source="limit: int32" type=int32
/// @resolution.name source=Generator target=Generator

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
        /// @resolution.call source="yield value" parameters=(int32) arguments=(provided(value) as int32) return=GeneratorRequest<void, void> kind=symbol target=GeneratorProducer.yield instance="GeneratorProducer.yield<\"local\">"
        /// @generic.instantiation id="GeneratorProducer.yield<\"local\", int32, void, void>" template=GeneratorProducer.yield arguments=("local", int32, void, void)
        /// @generic.instance id="GeneratorProducer.yield<int32, void, void>" template=GeneratorProducer.yield arguments=(int32, void, void)
        /// @generic.instance id="GeneratorProducer<int32, void, void>" template=GeneratorProducer arguments=(int32, void, void)
        /// @generic.instance id="GeneratorRequest<void, void>" template=GeneratorRequest arguments=(void, void)
        /// @generic.instance id="GeneratorState.publish<int32, void, void>" template=GeneratorState.publish arguments=(int32, void, void)
        /// @generic.instance id="GeneratorState.takeRequest<int32, void, void>" template=GeneratorState.takeRequest arguments=(int32, void, void)
        /// @generic.instance id=GeneratorNext<void> template=GeneratorNext arguments=(void)
        /// @generic.instance id=GeneratorReturn<void> template=GeneratorReturn arguments=(void)
        /// @resolution.name source=value target=count.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=count.value

    }
}
"#);
}
