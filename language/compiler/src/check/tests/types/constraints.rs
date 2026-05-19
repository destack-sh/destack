use super::super::snapshot::{assert_check_snapshot, assert_check_snapshot_with_diagnostics};

#[test]
fn test_check_records_interface_constraint_parameters() {
    assert_check_snapshot(
        r#"
interface Drawable {
    draw(): void;
}

function paint(item: Drawable): void {
    item.draw();
}
"#,
        r#"
interface Drawable {
/// @type.symbol key=Drawable value=Drawable

    draw(): void;
    /// @type.symbol key=Drawable.draw value=(this: Drawable) => void
}

function paint(item: Drawable): void {
/// @generic.parameters key=paint parameters=[paint.T0]
/// @type.symbol key=paint value=<paint.T0: Drawable>(paint.T0) => void
/// @generic.parameter key=paint.T0 space=type constraint=Drawable

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=direct target=Drawable.draw
    /// @resolution.call source="item.draw()" parameters=[] return=void kind=direct target=Drawable.draw
}

/// @type.summary types=3 nodes=1 symbols=4
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=1 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_records_structural_constraint_parameters() {
    assert_check_snapshot(
        r#"
type Drawable = {
    draw(): void;
};

function paint(item: Drawable): void {
    item.draw();
}
"#,
        r#"
type Drawable = {
/// @type.symbol key=Drawable value={ draw(): void }

    draw(): void;
};

function paint(item: Drawable): void {
/// @generic.parameters key=paint parameters=[paint.T0]
/// @type.symbol key=paint value=<paint.T0: Drawable>(paint.T0) => void
/// @generic.parameter key=paint.T0 space=type constraint=Drawable

    item.draw();
    /// @resolution.name source=item target=item
    /// @resolution.member source=item.draw receiver=paint.T0 kind=direct target=Drawable.draw
    /// @resolution.call source="item.draw()" parameters=[] return=void kind=direct target=Drawable.draw
}

/// @type.summary types=4 nodes=1 symbols=4
/// @generic.summary parameters=1 lists=1
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=1 labels=0 members=1 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_records_conditional_constraint_parameters() {
    assert_check_snapshot(
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
        r#"
interface TextSink {
/// @type.symbol key=TextSink value=TextSink

    write(value: string): void;
    /// @type.symbol key=TextSink.write value=(this: TextSink, string) => void
}

interface NumberSink {
/// @type.symbol key=NumberSink value=NumberSink

    write(value: int32): void;
    /// @type.symbol key=NumberSink.write value=(this: NumberSink, int32) => void
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.parameters key=SinkFor parameters=[SinkFor.T]
/// @generic.parameter key=SinkFor.T space=type
/// @type.symbol key=SinkFor value=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.parameters key=write parameters=[write.T, write.T0]
/// @generic.parameter key=write.T space=type
/// @type.symbol key=write value=<T, write.T0: SinkFor<T>>(T, write.T0) => void
/// @generic.parameter key=write.T0 space=type constraint=SinkFor<T>

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=[T] return=void kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const text: TextSink;
/// @type.symbol key=text value=TextSink

declare const number: NumberSink;
/// @type.symbol key=number value=NumberSink

write("message", text);
/// @resolution.name source=write target=write
/// @resolution.call source="write(\"message\", text)" parameters=[string, TextSink] return=void kind=direct target=write instance=instance0
/// @instance.node source="write(\"message\", text)" instance=instance0

write(1, number);
/// @resolution.name source=write target=write
/// @resolution.call source="write(1, number)" parameters=[int32, NumberSink] return=void kind=direct target=write instance=instance1
/// @instance.node source="write(1, number)" instance=instance1

/// @instance.entry instance=instance0 key=write arguments=[string, TextSink]
/// @instance.entry instance=instance1 key=write arguments=[int32, NumberSink]
/// @type.summary types=8 nodes=2 symbols=8
/// @generic.summary parameters=3 lists=2
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=3 labels=0 members=1 calls=3
/// @instance.summary instances=2 nodes=2
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0
"#,
    );
}

#[test]
fn test_check_reports_conditional_constraint_branch_mismatches() {
    assert_check_snapshot_with_diagnostics(
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
        r#"
=== dir ===
interface TextSink {
/// @type.symbol key=TextSink value=TextSink

    write(value: string): void;
    /// @type.symbol key=TextSink.write value=(this: TextSink, string) => void
}

interface NumberSink {
/// @type.symbol key=NumberSink value=NumberSink

    write(value: int32): void;
    /// @type.symbol key=NumberSink.write value=(this: NumberSink, int32) => void
}

type SinkFor<T> = T extends string ? TextSink : NumberSink;
/// @generic.parameters key=SinkFor parameters=[SinkFor.T]
/// @generic.parameter key=SinkFor.T space=type
/// @type.symbol key=SinkFor value=T extends string ? TextSink : NumberSink

function write<T>(value: T, sink: SinkFor<T>): void {
/// @generic.parameters key=write parameters=[write.T, write.T0]
/// @generic.parameter key=write.T space=type
/// @type.symbol key=write value=<T, write.T0: SinkFor<T>>(T, write.T0) => void
/// @generic.parameter key=write.T0 space=type constraint=SinkFor<T>

    sink.write(value);
    /// @resolution.name source=sink target=sink
    /// @resolution.member source=sink.write receiver=write.T0 kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
    /// @resolution.call source="sink.write(value)" parameters=[T] return=void kind=conditional branches=[T extends string => TextSink.write, else => NumberSink.write]
}

declare const number: NumberSink;
/// @type.symbol key=number value=NumberSink

write("message", number);
/// @resolution.name source=write target=write
/// @type.node source="write(\"message\", number)" value=void

/// @type.summary types=8 nodes=2 symbols=7
/// @generic.summary parameters=3 lists=2
/// @relation.summary extends=0 implements=0
/// @extension.summary extensions=0
/// @resolution.summary names=2 labels=0 members=1 calls=1
/// @instance.summary instances=0 nodes=0
/// @capture.summary functions=0 bindings=0 directives=0 rules=0
/// @layout.summary layouts=0 types=0

=== diagnostics ===
/// @diagnostic.error code=EC200 message="type is not assignable"
/// @diagnostic.label line=17 column=18 source="write(\"message\", number);"
"#,
    );
}
