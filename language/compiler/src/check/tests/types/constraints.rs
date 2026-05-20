use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_interface_constraint_parameters() {
    let session = TestSession::single(
        r#"
interface Drawable {
    draw(): void;
}

function paint(item: Drawable): void {
    item.draw();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
interface Drawable {
/// @type.symbol symbol=Drawable type=Drawable

    draw(): void;
    /// @type.symbol symbol=Drawable.draw type=(this: Drawable) => void
}

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=<paint.T0: Drawable>(paint.T0) => void
/// @generic.slot symbol=paint.T0 index=0 kind=type constraint=Drawable

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=direct target=Drawable.draw
    /// @resolution.call source="item.draw()" parameters=[] return=void kind=direct target=Drawable.draw receiver=paint.T0
}
"#);
}

#[test]
fn test_check_records_structural_constraint_parameters() {
    let session = TestSession::single(
        r#"
type Drawable = {
    draw(): void;
};

function paint(item: Drawable): void {
    item.draw();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
type Drawable = {
/// @type.symbol symbol=Drawable type={ draw(): void }

    draw(): void;
};

function paint(item: Drawable): void {
/// @type.symbol symbol=paint type=<paint.T0: Drawable>(paint.T0) => void
/// @generic.slot symbol=paint.T0 index=0 kind=type constraint=Drawable

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=direct target=Drawable.draw
    /// @resolution.call source="item.draw()" parameters=[] return=void kind=direct target=Drawable.draw receiver=paint.T0
}
"#);
}

#[test]
fn test_check_records_conditional_constraint_parameters() {
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
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write type=(this: TextSink, string) => void
}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write type=(this: NumberSink, int32) => void
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.slot symbol=SinkFor.T index=0 kind=type
/// @type.symbol symbol=SinkFor type=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.slot symbol=write.T index=0 kind=type
/// @type.symbol symbol=write type=<T, write.T0: SinkFor<T>>(T, write.T0) => void
/// @generic.slot symbol=write.T0 index=1 kind=type constraint=SinkFor<T>

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=[T] return=void kind=conditional receiver=write.T0 branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const text: TextSink;
/// @type.symbol symbol=text type=TextSink

declare const number: NumberSink;
/// @type.symbol symbol=number type=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=[string, TextSink] return=void kind=direct target=write instance="write<string, TextSink>"
/// @instance.application source="write(\"message\", text)" id="write<string, TextSink>"

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=[int32, NumberSink] return=void kind=direct target=write instance="write<int32, NumberSink>"
/// @instance.application source="write(1, number)" id="write<int32, NumberSink>"

/// @instance.entry id="write<string, TextSink>" symbol=write arguments=[string, TextSink]
/// @instance.entry id="write<int32, NumberSink>" symbol=write arguments=[int32, NumberSink]
"#);
}

#[test]
fn test_check_reports_conditional_constraint_branch_mismatches() {
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
        DirRows::checked(),
        r#"
interface TextSink {
/// @type.symbol symbol=TextSink type=TextSink

    write(value: string): void;
    /// @type.symbol symbol=TextSink.write type=(this: TextSink, string) => void
}

interface NumberSink {
/// @type.symbol symbol=NumberSink type=NumberSink

    write(value: int32): void;
    /// @type.symbol symbol=NumberSink.write type=(this: NumberSink, int32) => void
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.slot symbol=SinkFor.T index=0 kind=type
/// @type.symbol symbol=SinkFor type=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.slot symbol=write.T index=0 kind=type
/// @type.symbol symbol=write type=<T, write.T0: SinkFor<T>>(T, write.T0) => void
/// @generic.slot symbol=write.T0 index=1 kind=type constraint=SinkFor<T>

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=[T] return=void kind=conditional receiver=write.T0 branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const number: NumberSink;
/// @type.symbol symbol=number type=NumberSink

write("message", number);
/// @resolution.name source=write target=write
/// @type.node source="write(\"message\", number)" type=void
"#,
        r#"
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=17 column=18 source="write(\"message\", number);"
"#,
    );
}
