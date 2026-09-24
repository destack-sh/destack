use crate::tests::{DirRows, TestSession};

/// A deferred conditional member rejects in the body and reduces at every call site.
#[test]
fn test_conditional_parameter_reduces_at_call_sites() {
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

    session.assert_dir_and_diagnostics(
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

write<string>("message", text);
write<int64>(1, number);

=== dir ===
interface TextSink {
/// @generic.template symbol=TextSink parameters=(this: TextSink)
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink template=(this: TextSink)
/// @definition.where symbol=TextSink relation=satisfies left=this right=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(string) => void
    /// @type.symbol symbol=TextSink.write.value source="value: string" type=string

}

interface NumberSink {
/// @generic.template symbol=NumberSink parameters=(this: NumberSink)
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink template=(this: NumberSink)
/// @definition.where symbol=NumberSink relation=satisfies left=this right=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(int32) => void
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
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=write.value

}

declare const text: TextSink;
/// @type.symbol symbol=text source=text type=TextSink
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=TextSink target=TextSink

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=NumberSink
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=NumberSink target=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=(string, SinkFor<string>) arguments=(provided("message") as string, provided(text) as SinkFor<string>) return=void kind=symbol target=write instance=write<string>
/// @generic.instantiation id=write<string> template=write arguments=(string)
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="immutable"
/// @resolution.access source=text root=text

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=(int64, SinkFor<int64>) arguments=(provided(1) as int64, provided(number) as SinkFor<int64>) return=void kind=symbol target=write instance=write<int64>
/// @generic.instantiation id=write<int64> template=write arguments=(int64)
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="local" lifetime="static" access="immutable"
/// @resolution.access source=number root=number
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

    session.assert_dir_and_diagnostics(
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
write<string>("message", number);

=== dir ===
interface TextSink {
/// @generic.template symbol=TextSink parameters=(this: TextSink)
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink template=(this: TextSink)
/// @definition.where symbol=TextSink relation=satisfies left=this right=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(string) => void
    /// @type.symbol symbol=TextSink.write.value source="value: string" type=string

}

interface NumberSink {
/// @generic.template symbol=NumberSink parameters=(this: NumberSink)
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink template=(this: NumberSink)
/// @definition.where symbol=NumberSink relation=satisfies left=this right=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(int32) => void
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
    /// @type.node source=value type=T#2
    /// @resolution.name source=value target=write.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=write.value

}

declare const number: NumberSink;
/// @type.symbol symbol=number source=number type=NumberSink
/// @resolution.pattern source=number kind=binding target=number
/// @resolution.name source=NumberSink target=NumberSink

write("message", number);
/// @type.node source="write(\"message\", number)" type=void
/// @type.node source=write type=(string, SinkFor<string>) => void
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", number)" parameters=(string, SinkFor<string>) arguments=(provided("message") as string, provided(number) as SinkFor<string>) return=void kind=symbol target=write instance=write<string>
/// @generic.instantiation id=write<string> template=write arguments=(string)
/// @type.node source="\"message\"" type="message"
/// @type.node source=number type=NumberSink
/// @resolution.name source=number target=number
/// @resolution.place source=number placement="local" lifetime="static" access="immutable"
/// @resolution.access source=number root=number
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'write' does not exist on type 'SinkFor<T>'"
/// @diagnostic.label line=13 column=10 span="write" line_source="sink.write(value);"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'NumberSink' is not assignable to parameter of type 'SinkFor<string>'"
/// @diagnostic.label line=17 column=18 span="number" line_source="write(\"message\", number);"
/// @diagnostic.related line=17 column=1 span="write(\"message\", number)" line_source="write(\"message\", number);" message="in this call"
/// @diagnostic.note message="the mismatch is in parameter 0 of field 'write': expected 'int32', found 'string'"
"#,
    );
}
