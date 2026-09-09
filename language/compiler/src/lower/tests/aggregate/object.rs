use crate::tests::TestSession;

/// Lower an object literal as an instance of its declared class.
#[test]
fn test_lower_an_object_literal_to_its_declared_class() {
    let session = TestSession::single(
        r#"
type Point = { x: int32 };

function read(point: Point): int32 {
    return point.x;
}

function build(): int32 {
    const point: Point = { x: 7 };
    return read(point);
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.read",
        r#"
type test.main.Point {
    x: int32;
}

function test.main.read(v0: ref<test.main.Point, managed, mutable, local>): int32 {
    local l0: ref<test.main.Point, managed, mutable, local>

entry(v0: ref<test.main.Point, managed, mutable, local>):
    local.set l0, v0
    v1: ref<test.main.Point, managed, mutable, local> = local.get l0
    v2: ref<int32, borrowed, 'managed, readonly, local> = field.project v1, 0
    v3: int32 = load v2
    return v3
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
type test.main.Point {
    x: int32;
}

function test.main.build(): int32 {
    local l0: ref<test.main.Point, managed, mutable, local>

entry:
    v0: int32 = 7
    v1: test.main.Point = aggregate (v0)
    v2: ref<test.main.Point, managed, mutable, local> = new.complete v1
    local.set l0, v2
    v3: ref<test.main.Point, managed, mutable, local> = local.get l0
    v4: int32 = call test.main.read(v3): (ref<test.main.Point, managed, mutable, local>) => int32
    return v4
}

/// @layout.struct name=test.main.Point size=4 align=4
/// @layout.field owner=test.main.Point index=0 name=x offset=0 size=4 align=4
"#,
    );
}

/// Lower a recursive object alias to its named managed reference.
#[test]
fn test_lower_a_recursive_object_alias_to_a_named_reference() {
    let session = TestSession::single(
        r#"
type Selector = {
    depth?: int32;
    nested?: Selector;
};

function pick(selector: Selector): int32 {
    return 0;
}

function build(): int32 {
    return pick({ depth: 3 });
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.pick",
        r#"
type test.main.Selector {
    depth: variant<uint1> { 0uint1 = void; 1uint1 = int32; };
    nested: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Selector, managed, mutable, local>; };
}

function test.main.pick(v0: ref<test.main.Selector, managed, mutable, local>): int32 {
    local l0: ref<test.main.Selector, managed, mutable, local>

entry(v0: ref<test.main.Selector, managed, mutable, local>):
    local.set l0, v0
    v1: int32 = 0
    return v1
}

/// @layout.struct name=test.main.Selector size=16 align=8
/// @layout.field owner=test.main.Selector index=0 name=depth offset=8 size=8 align=4
/// @layout.field owner=test.main.Selector index=1 name=nested offset=0 size=8 align=8
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
type test.main.Selector {
    depth: variant<uint1> { 0uint1 = void; 1uint1 = int32; };
    nested: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Selector, managed, mutable, local>; };
}

function test.main.build(): int32 {
entry:
    v0: int32 = 3
    v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<test.main.Selector, managed, mutable, local>; } = variant.new 0
    v2: test.main.Selector = aggregate (v0, v1)
    v3: ref<test.main.Selector, managed, mutable, local> = new.complete v2
    v4: int32 = call test.main.pick(v3): (ref<test.main.Selector, managed, mutable, local>) => int32
    return v4
}

/// @layout.struct name=test.main.Selector size=16 align=8
/// @layout.field owner=test.main.Selector index=0 name=depth offset=8 size=8 align=4
/// @layout.field owner=test.main.Selector index=1 name=nested offset=0 size=8 align=8
/// @layout.variant name=type@7 size=8 align=8
/// @layout.discriminant owner=type@7 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@7 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@7 index=1 discriminant=1 payload_offset=0
"#,
    );
}

/// Construct an owned object literal in place without an allocation.
#[test]
fn test_construct_an_owned_object_literal_in_place() {
    let session = TestSession::single(
        r#"
function build(): int32 {
    const point: ^{ x: int32 } = { x: 7 };
    return point.x;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.build",
        r#"
function test.main.build(): int32 {
    local l0: { x: int32 }

entry:
    v0: int32 = 7
    v1: { x: int32 } = aggregate (v0)
    local.set l0, v1
    v2: ref<{ x: int32 }, borrowed, 'frame, readonly, frame> = local.project l0
    v3: ref<int32, borrowed, 'frame, readonly, frame> = field.project v2, 0
    v4: int32 = load v3
    return v4
}

/// @layout.struct name=type@5 size=4 align=4
/// @layout.field owner=type@5 index=0 name=x offset=0 size=4 align=4
"#,
    );
}

#[test]
fn test_lower_an_object_literal_with_an_absent_borrowed_field() {
    let session = TestSession::single(
        r#"
type Fields = { readonly [key: string]: string };

type LogOptions<'a> = {
    fields?: Fields | undefined;
    message?: &'a readonly string | undefined;
};

function log(name: &readonly string, options?: LogOptions): void {}

export function trace(name: &readonly string, fields?: Fields): void {
    log(name, { fields });
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.trace",
        r#"
@languageItem("string.String")
type String;

type test.main.Fields = dynamic<{  }, managed, mutable, local>;

type test.main.LogOptions<'a> {
    fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; };
    message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.trace<'a>(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }):
    local.set l0, v0
    local.set l1, v1
    v2: ref<String, borrowed, 'a, readonly, local> = local.get l0
    v3: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; } = local.get l1
    v4: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'frame, readonly, local>; } = variant.new 0
    v5: test.main.LogOptions<'frame & local> = aggregate (v3, v4)
    v6: ref<test.main.LogOptions<'frame & local>, managed, mutable, local> = new.complete v5
    v7: variant<uint1> { 0uint1 = ref<test.main.LogOptions<'frame & local>, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v6
    call test.main.log(v2, v7): <'a, 'b>(ref<String, borrowed, 'a, readonly, local>, variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }, message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; 1uint1 = void; }) => void
    return
}

/// @layout.variant name=type@13 size=16 align=8
/// @layout.discriminant owner=type@13 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@16 size=8 align=8
/// @layout.discriminant owner=type@16 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@16 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@16 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@18 size=24 align=8
/// @layout.field owner=type@18 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@18 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@20 size=8 align=8
/// @layout.discriminant owner=type@20 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@20 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@20 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=test.main.LogOptions<'frame & local> size=24 align=8
/// @layout.field owner=test.main.LogOptions<'frame & local> index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=test.main.LogOptions<'frame & local> index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@46 size=8 align=8
/// @layout.discriminant owner=type@46 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@46 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@46 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@54 size=8 align=8
/// @layout.discriminant owner=type@54 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@54 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@54 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function("main.ds", "test.main.log", r#"
@languageItem("string.String")
type String;

type test.main.Fields = dynamic<{  }, managed, mutable, local>;

function test.main.log<'a, 'b>(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }, message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }, message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }, message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; 1uint1 = void; }):
    local.set l0, v0
    local.set l1, v1
    return
}

/// @layout.variant name=type@13 size=16 align=8
/// @layout.discriminant owner=type@13 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@16 size=8 align=8
/// @layout.discriminant owner=type@16 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@16 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@16 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@18 size=24 align=8
/// @layout.field owner=type@18 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@18 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@20 size=8 align=8
/// @layout.discriminant owner=type@20 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@20 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@20 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.ds", "test.main.trace", r#"
@languageItem("string.String")
type String;

type test.main.Fields = dynamic<{  }, managed, mutable, local>;

type test.main.LogOptions<'a> {
    fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; };
    message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.trace<'a>(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly, local>
    local l1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }):
    local.set l0, v0
    local.set l1, v1
    v2: ref<String, borrowed, 'a, readonly, local> = local.get l0
    v3: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; } = local.get l1
    v4: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'frame, readonly, local>; } = variant.new 0
    v5: test.main.LogOptions<'frame & local> = aggregate (v3, v4)
    v6: ref<test.main.LogOptions<'frame & local>, managed, mutable, local> = new.complete v5
    v7: variant<uint1> { 0uint1 = ref<test.main.LogOptions<'frame & local>, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v6
    call test.main.log(v2, v7): <'a, 'b>(ref<String, borrowed, 'a, readonly, local>, variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = test.main.Fields; 1uint1 = void; }, message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; 1uint1 = void; }) => void
    return
}

/// @layout.variant name=type@13 size=16 align=8
/// @layout.discriminant owner=type@13 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@13 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@13 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@16 size=8 align=8
/// @layout.discriminant owner=type@16 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@16 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@16 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@18 size=24 align=8
/// @layout.field owner=type@18 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@18 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@20 size=8 align=8
/// @layout.discriminant owner=type@20 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@20 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@20 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=test.main.LogOptions<'frame & local> size=24 align=8
/// @layout.field owner=test.main.LogOptions<'frame & local> index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=test.main.LogOptions<'frame & local> index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@46 size=8 align=8
/// @layout.discriminant owner=type@46 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@46 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@46 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@54 size=8 align=8
/// @layout.discriminant owner=type@54 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@54 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@54 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_an_optional_chain_borrow_into_a_struct_field() {
    let session = TestSession::single(
        r#"
type LogOptions<'a> = {
    message?: &'a readonly string | undefined;
};

struct LogEntry<'a> {
    logger: &'a readonly string;
    message: &'a readonly string | undefined;
}

function write(entry: &readonly LogEntry): void {}

class Logger {
    readonly name: string;

    constructor(name: string) {
        this.name = name;
    }

    log(&readonly this, options?: LogOptions): void {
        const entry = LogEntry {
            logger: &readonly this.name,
            message: options?.message,
        };
        write(&readonly entry);
    }
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.Logger.log",
        r#"
@languageItem("string.String")
type String;

type test.main.Logger {
    name: ref<String, managed, mutable, local>;
}

@copy
type test.main.LogEntry<'a> {
    logger: ref<String, borrowed, 'a, readonly>;
    message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'a, readonly>; };
}

function test.main.Logger.log<'a, 'b>(v0: ref<test.main.Logger, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<{ message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; }): void {
    local l0: variant<uint1> { 0uint1 = void; 1uint1 = ref<{ message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; }
    local l1: ref<test.main.Logger, borrowed, 'a, readonly, local>
    local l2: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; }
    local l3: test.main.LogEntry<'a | 'b & local>

entry(v0: ref<test.main.Logger, borrowed, 'a, readonly, local>, v1: variant<uint1> { 0uint1 = void; 1uint1 = ref<{ message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; }):
    local.set l0, v1
    local.set l1, v0
    v2: ref<test.main.Logger, borrowed, 'a, readonly, local> = local.get l1
    v3: ref<ref<String, managed, readonly, local>, borrowed, 'a, readonly, local> = field.project v2, 0
    v4: ref<String, managed, readonly, local> = load v3
    v5: ref<String, borrowed, 'a, readonly, local> = cast.bit v4 -> ref<String, borrowed, 'a, readonly, local>
    v6: variant<uint1> { 0uint1 = void; 1uint1 = ref<{ message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local>; } = local.get l0
    v7: ref<{ message: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } }, managed, mutable, local> = variant.payload v6, 1
    v8: ref<variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; }, borrowed, 'managed, readonly, local> = field.project v7, 0
    v9: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } = load v8
    local.set l2, v9
    jump b1

b1:
    v10: variant<uint1> { 0uint1 = void; 1uint1 = ref<String, borrowed, 'b, readonly, local>; } = local.get l2
    v11: test.main.LogEntry<'a | 'b & local> = aggregate (v5, v10)
    local.set l3, v11
    v12: ref<test.main.LogEntry<'a | 'b & local>, borrowed, 'frame, readonly, local> = local.address l3
    call test.main.write(v12): <'a, 'b>(ref<test.main.LogEntry<'a & local>, borrowed, 'b, readonly, local>) => void
    return
}

/// @layout.struct name=test.main.Logger size=8 align=8
/// @layout.field owner=test.main.Logger index=0 name=name offset=0 size=8 align=8
/// @layout.struct name=test.main.LogEntry<'a & local> size=16 align=8
/// @layout.field owner=test.main.LogEntry<'a & local> index=0 name=logger offset=0 size=8 align=8
/// @layout.field owner=test.main.LogEntry<'a & local> index=1 name=message offset=8 size=8 align=8
/// @layout.variant name=type@31 size=8 align=8
/// @layout.discriminant owner=type@31 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@31 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@31 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@33 size=8 align=8
/// @layout.field owner=type@33 index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@35 size=8 align=8
/// @layout.discriminant owner=type@35 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=1 niche_start=0
/// @layout.case owner=type@35 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@35 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=test.main.LogEntry<'a | 'b & local> size=16 align=8
/// @layout.field owner=test.main.LogEntry<'a | 'b & local> index=0 name=logger offset=0 size=8 align=8
/// @layout.field owner=test.main.LogEntry<'a | 'b & local> index=1 name=message offset=8 size=8 align=8
"#,
    );
}

#[test]
fn test_lower_a_constructor_reading_this_after_its_fields_initialize() {
    let session = TestSession::single(
        r#"
class Counter {
    count: int32;

    constructor(start: int32) {
        this.count = start;
        this.bump();
        const read = () => this.count;
        read();
    }

    bump(this): void {
        this.count += 1;
    }
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.Counter.constructor", r#"
type test.main.Counter {
    count: int32;
}

function test.main.Counter.constructor<'a>(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>
    local l2: function<<'a>() => int32, repeatable, managed, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local>, v1: int32):
    local.set l0, v1
    local.set l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v3: int32 = local.get l0
    v4: ref<uninit<int32>, borrowed, 'a, mutable, local> = field.project v2, 0
    store v4, v3
    v5: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v6: ref<test.main.Counter, managed, mutable, local> = cast.bit v5 -> ref<test.main.Counter, managed, mutable, local>
    call test.main.Counter.bump(v6): (ref<test.main.Counter, managed, mutable, local>) => void
    v7: ref<uninit<test.main.Counter>, borrowed, 'a, mutable, local> = local.get l1
    v8: ref<test.main.Counter, managed, mutable, local> = cast.bit v7 -> ref<test.main.Counter, managed, mutable, local>
    v9: { ref<test.main.Counter, managed, mutable, local> } = aggregate (v8)
    v10: ref<{ ref<test.main.Counter, managed, mutable, local> }, managed, mutable, local> = new.complete v9
    v11: function<<'a>() => int32, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.closure#0, v10
    local.set l2, v11
    v12: function<<'a>() => int32, repeatable, managed, mutable, local> = local.get l2
    v13: function<<'a>() => int32, repeatable, borrowed, 'managed, readonly, local> = cast.bit v12 -> function<<'a>() => int32, repeatable, borrowed, 'managed, readonly, local>
    v14: int32 = call.indirect v13(): <'a>() => int32
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@29 size=8 align=8
/// @layout.field owner=type@29 index=0 offset=0 size=8 align=8
"#);
}
