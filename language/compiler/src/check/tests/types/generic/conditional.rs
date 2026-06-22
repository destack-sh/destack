use crate::tests::{DirRows, TestSession};

#[test]
fn test_conditional_constraint_selects_valid_branch() {
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

    session.assert_dir_checked(
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

function write<T, T0: SinkFor<T>>(value: T, sink: T0): void {
    sink.write(value);
}

declare const text: TextSink;
declare const number: NumberSink;

write("message", text);
write(1, number);

=== checked ===
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(this: TextSink, string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(this: TextSink, string) => void
    /// @type.symbol symbol=value#1 source="value: string" type=string

}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(this: NumberSink, int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(this: NumberSink, int32) => void
    /// @type.symbol symbol=value#2 source="value: int32" type=int32

}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.template symbol=SinkFor parameters=(T)
/// @type.symbol symbol=SinkFor type=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.template symbol=write parameters=(T, T0: SinkFor<T>)
/// @type.symbol symbol=write type=<T, write.T0: SinkFor<T>>(T, write.T0) => void

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=(T) return=void kind=conditional receiver=write.T0 branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const text: TextSink;
/// @type.symbol symbol=text type=TextSink

declare const number: NumberSink;
/// @type.symbol symbol=number type=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=(string, TextSink) return=void kind=symbol target=write instance="write<string, TextSink>"
/// @generic.instance source="write(\"message\", text)" id="write<string, TextSink>"

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=(int32, NumberSink) return=void kind=symbol target=write instance="write<int32, NumberSink>"
/// @generic.instance source="write(1, number)" id="write<int32, NumberSink>"

/// @generic.instance id="write<string, TextSink>" template=write arguments=(string, TextSink)
/// @generic.instance id="write<int32, NumberSink>" template=write arguments=(int32, NumberSink)
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

function write<T, T0: SinkFor<T>>(value: T, sink: T0): void {
    sink.write(value);
}

declare const number: NumberSink;
write("message", number);

=== checked ===
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink
/// @definition.interface symbol=TextSink
/// @definition.method symbol=TextSink.write source="write(value: string): void" slot=write type=(this: TextSink, string) => void

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write source="write(value: string): void" type=(this: TextSink, string) => void
    /// @type.symbol symbol=value#1 source="value: string" type=string

}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink
/// @definition.interface symbol=NumberSink
/// @definition.method symbol=NumberSink.write source="write(value: int32): void" slot=write type=(this: NumberSink, int32) => void

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write source="write(value: int32): void" type=(this: NumberSink, int32) => void
    /// @type.symbol symbol=value#2 source="value: int32" type=int32

}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.template symbol=SinkFor parameters=(T)
/// @type.symbol symbol=SinkFor type=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.template symbol=write parameters=(T, T0: SinkFor<T>)
/// @type.symbol symbol=write type=<T, write.T0: SinkFor<T>>(T, write.T0) => void

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=(T) return=void kind=conditional receiver=write.T0 branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const number: NumberSink;
/// @type.symbol symbol=number type=NumberSink

write("message", number);
/// @resolution.name source=write target=write
/// @type.node source="write(\"message\", number)" type=void
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'NumberSink' is not assignable to type 'TextSink'"
/// @diagnostic.label line=17 column=18 source="write(\"message\", number);"
"#,
    );
}
