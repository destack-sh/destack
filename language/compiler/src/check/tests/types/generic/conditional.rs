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

declare const text: Dynamic<TextSink>;
declare const number: Dynamic<NumberSink>;

write<string>("message", text);
write<float64>(1, number);

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
/// @type.symbol symbol=write.sink source="sink: SinkFor<T>" type=SinkFor<T#2>
/// @resolution.name source=SinkFor target=SinkFor
/// @resolution.name source=T target=write.T

    sink.write(value);
    /// @resolution.name source=sink target=write.sink
    /// @resolution.place source=sink placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sink root=write.sink
    /// @resolution.rejected source=sink.write
    /// @resolution.rejected source=sink.write(value)
    /// @resolution.name source=value target=write.value

}

declare const text: TextSink;
/// @type.symbol symbol=text source=text type=Dynamic<TextSink>
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=TextSink target=TextSink

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=Dynamic<NumberSink>
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=NumberSink target=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=(string, TextSink) arguments=(provided("message") as string, provided(text) as TextSink) return=void kind=symbol target=write instance=write<string>
/// @generic.instance source="write(\"message\", text)" id=write<string>
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=text root=text

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=(float64, NumberSink) arguments=(provided(1) as float64, provided(number) as NumberSink) return=void kind=symbol target=write instance=write<float64>
/// @generic.instance source="write(1, number)" id=write<float64>
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=number root=number

/// @generic.instance id=SinkFor<T#2> template=SinkFor arguments=(T#2)
/// @generic.instance id=write<float64> template=write arguments=(float64)
/// @generic.instance id=write<string> template=write arguments=(string)
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'write' does not exist on type 'SinkFor<T>'"
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

declare const number: Dynamic<NumberSink>;
write<string>("message", number);

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
/// @type.symbol symbol=write.sink source="sink: SinkFor<T>" type=SinkFor<T#2>
/// @resolution.name source=SinkFor target=SinkFor
/// @resolution.name source=T target=write.T

    sink.write(value);
    /// @type.node source=sink type=SinkFor<T#2>
    /// @type.node source=sink.write type=<error>
    /// @type.node source=sink.write(value) type=<error>
    /// @resolution.name source=sink target=write.sink
    /// @resolution.place source=sink placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=sink root=write.sink
    /// @resolution.rejected source=sink.write
    /// @resolution.rejected source=sink.write(value)
    /// @generic.instance source=sink id=SinkFor<T#2>
    /// @resolution.name source=value target=write.value

}

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=Dynamic<NumberSink>
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=NumberSink target=NumberSink

write("message", number);
/// @type.node source="write(\"message\", number)" type=void
/// @type.node source=write type=(string, TextSink) => void
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", number)" parameters=(string, TextSink) arguments=(provided("message") as string, provided(number) as TextSink) return=void kind=symbol target=write instance=write<string>
/// @generic.instance source="write(\"message\", number)" id=write<string>
/// @type.node source="\"message\"" type="message"
/// @type.node source=number type=Dynamic<NumberSink>
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=number root=number

/// @generic.instance id=SinkFor<T#2> template=SinkFor arguments=(T#2)
/// @generic.instance id=write<string> template=write arguments=(string)
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'write' does not exist on type 'SinkFor<T>'"
/// @diagnostic.label line=13 column=10 span="write" line_source="sink.write(value);"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Dynamic<NumberSink>' is not assignable to parameter of type 'SinkFor<string>'"
/// @diagnostic.label line=17 column=18 span="number" line_source="write(\"message\", number);"
/// @diagnostic.related line=17 column=1 span="write(\"message\", number)" line_source="write(\"message\", number);" message="in this call"
/// @diagnostic.note message="'SinkFor<string>' reduces to 'TextSink'"
"#,
    );
}
