use crate::tests::{DirRows, TestSession};

#[test]
fn test_conditional_parameter_reduces_at_call_sites() {
    // deferred conditional members reject in the body: no induced generics,
    // and tsc requires narrowing; call sites reduce at instantiation
    let session = TestSession::single(
        r#"
interface TextSink {
    write(value: string): void;
}

interface NumberSink {
    write(value: int32): void;
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;

function write<T>(value: T, sink: SinkFor<T>): void {
    sink.write(value);
}

declare const text: TextSink;
declare const number: NumberSink;

write("message", text);
write(1, number);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface TextSink {
    write(value: string): void;
}

interface NumberSink {
    write(value: int32): void;
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;

function write<T>(value: T, sink: SinkFor<T>): void {
    sink.write(value);
}

declare const text: TextSink;
declare const number: NumberSink;

write<"message">("message", text);
write<1>(1, number);

=== checked ===
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(this: this, string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(this: this, string) => void
    /// @type.symbol symbol=TextSink.write.value source="value: string" type=string

}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(this: this, int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(this: this, int32) => void
    /// @type.symbol symbol=NumberSink.write.value source="value: int32" type=int32

}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.template symbol=SinkFor parameters=(T#1)
/// @type.symbol symbol=SinkFor source="type SinkFor<T> = T extends string ? TextSink : NumberSink" type=T#1 extends string ? TextSink : NumberSink
/// @definition.type symbol=SinkFor source="type SinkFor<T> = T extends string ? TextSink : NumberSink" template=(T#1) value=T#1 extends string ? TextSink : NumberSink
/// @type.symbol symbol=SinkFor.T source=T type=T#1
/// @resolution.name source=T target=SinkFor.T
/// @resolution.name source=TextSink target=TextSink
/// @resolution.name source=NumberSink target=NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.template symbol=write parameters=(T#2)
/// @type.symbol symbol=write type=<T#2>(T#2, SinkFor<T#2>) => void
/// @type.symbol symbol=write.T source=T type=T#2
/// @type.symbol symbol=write.value source="value: T" type=T#2
/// @resolution.name source=T target=write.T
/// @type.symbol symbol=write.sink source="sink: SinkFor<T>" type=SinkFor<T#2> reduced=T#2 extends string ? TextSink : NumberSink
/// @resolution.name source=SinkFor target=SinkFor
/// @resolution.name source=T target=write.T

    sink.write(value);
    /// @resolution.name source=sink target=write.sink
    /// @resolution.name source=value target=write.value

}

declare const text: TextSink;
/// @type.symbol symbol=text source=text type=TextSink
/// @resolution.name source=TextSink target=TextSink

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=NumberSink
/// @resolution.name source=NumberSink target=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=("message", SinkFor<"message">) arguments=(provided("message") as "message", provided(text) as SinkFor<"message">) return=void kind=symbol target=write instance="write<\"message\">"
/// @generic.instance source="write(\"message\", text)" id="write<\"message\">"
/// @resolution.name source=text target=text

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=(1, SinkFor<1>) arguments=(provided(1) as 1, provided(number) as SinkFor<1>) return=void kind=symbol target=write instance=write<1>
/// @generic.instance source="write(1, number)" id=write<1>
/// @resolution.name source=number target=number

/// @generic.instance id="write<\"message\">" template=write arguments=("message")
/// @generic.instance id=SinkFor<T#2> template=SinkFor arguments=(T#2)
/// @generic.instance id=write<1> template=write arguments=(1)
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'write' does not exist on type 'SinkFor<T>'"
/// @diagnostic.label line=13 column=10 span="write" line_source="sink.write(value);"
"#,
    );
}

#[test]
fn test_conditional_constraint_invalid_branch_reports_error() {
    let session = TestSession::single(
        r#"
interface TextSink {
    write(value: string): void;
}

interface NumberSink {
    write(value: int32): void;
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;

function write<T>(value: T, sink: SinkFor<T>): void {
    sink.write(value);
}

declare const number: NumberSink;
write("message", number);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface TextSink {
    write(value: string): void;
}

interface NumberSink {
    write(value: int32): void;
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;

function write<T>(value: T, sink: SinkFor<T>): void {
    sink.write(value);
}

declare const number: NumberSink;
write<"message">("message", number);

=== checked ===
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(this: this, string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(this: this, string) => void
    /// @type.symbol symbol=TextSink.write.value source="value: string" type=string

}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(this: this, int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(this: this, int32) => void
    /// @type.symbol symbol=NumberSink.write.value source="value: int32" type=int32

}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.template symbol=SinkFor parameters=(T#1)
/// @type.symbol symbol=SinkFor source="type SinkFor<T> = T extends string ? TextSink : NumberSink" type=T#1 extends string ? TextSink : NumberSink
/// @definition.type symbol=SinkFor source="type SinkFor<T> = T extends string ? TextSink : NumberSink" template=(T#1) value=T#1 extends string ? TextSink : NumberSink
/// @type.symbol symbol=SinkFor.T source=T type=T#1
/// @resolution.name source=T target=SinkFor.T
/// @resolution.name source=TextSink target=TextSink
/// @resolution.name source=NumberSink target=NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.template symbol=write parameters=(T#2)
/// @type.symbol symbol=write type=<T#2>(T#2, SinkFor<T#2>) => void
/// @type.symbol symbol=write.T source=T type=T#2
/// @type.symbol symbol=write.value source="value: T" type=T#2
/// @resolution.name source=T target=write.T
/// @type.symbol symbol=write.sink source="sink: SinkFor<T>" type=SinkFor<T#2> reduced=T#2 extends string ? TextSink : NumberSink
/// @resolution.name source=SinkFor target=SinkFor
/// @resolution.name source=T target=write.T

    sink.write(value);
    /// @type.node source=sink type=SinkFor<T#2> reduced=T#2 extends string ? TextSink : NumberSink
    /// @type.node source=sink.write type=<error>
    /// @type.node source=sink.write(value) type=<error>
    /// @resolution.name source=sink target=write.sink
    /// @generic.instance source=sink id=SinkFor<T#2>
    /// @resolution.name source=value target=write.value

}

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=NumberSink
/// @resolution.name source=NumberSink target=NumberSink

write("message", number);
/// @type.node source="write(\"message\", number)" type=void
/// @type.node source=write type=("message", SinkFor<"message">) => void
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", number)" parameters=("message", SinkFor<"message">) arguments=(provided("message") as "message", provided(number) as SinkFor<"message">) return=void kind=symbol target=write instance="write<\"message\">"
/// @generic.instance source="write(\"message\", number)" id="write<\"message\">"
/// @generic.instance source=write id="SinkFor<\"message\">"
/// @type.node source="\"message\"" type="message"
/// @type.node source=number type=NumberSink
/// @resolution.name source=number target=number

/// @generic.instance id="SinkFor<\"message\">" template=SinkFor arguments=("message")
/// @generic.instance id="write<\"message\">" template=write arguments=("message")
/// @generic.instance id=SinkFor<T#2> template=SinkFor arguments=(T#2)
"#,
        r#"
/// @diagnostic.error code=EC300 message="member 'write' does not exist on type 'SinkFor<T>'"
/// @diagnostic.label line=13 column=10 span="write" line_source="sink.write(value);"
/// @diagnostic.error code=EC209 message="argument of type 'NumberSink' is not assignable to parameter of type 'SinkFor<\"message\">'"
/// @diagnostic.label line=17 column=18 span="number" line_source="write(\"message\", number);"
/// @diagnostic.related line=17 column=1 span="write(\"message\", number)" line_source="write(\"message\", number);" message="in this call"
/// @diagnostic.note message="'SinkFor<\"message\">' reduces to 'TextSink'"
"#,
    );
}
