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
        "main.tspp",
        "test.main.read",
        r#"
function test.main.read(v0: ref<{ x: int32 }, managed, mutable, local>): int32 {
    local l0: ref<{ x: int32 }, managed, mutable, local>

entry(v0: ref<{ x: int32 }, managed, mutable, local>):
    store l0, v0
    v1: ref<{ x: int32 }, managed, mutable, local> = load l0
    v2: int32 = load (*v1).0
    return v2
}

/// @layout.struct name=type@1 size=4 align=4
/// @layout.field owner=type@1 index=0 name=x offset=0 size=4 align=4
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.build",
        r#"
function test.main.build(): int32 {
    local l0: ref<{ x: int32 }, managed, mutable, local>

entry:
    v0: int32 = 7
    v1: { x: int32 } = aggregate (v0)
    v2: ref<{ x: int32 }, managed, mutable, local> = new.complete v1
    store l0, v2
    v3: ref<{ x: int32 }, managed, mutable, local> = load l0
    v4: int32 = call test.main.read(v3): (ref<{ x: int32 }, managed, mutable, local>) => int32
    return v4
}

/// @layout.struct name=type@1 size=4 align=4
/// @layout.field owner=type@1 index=0 name=x offset=0 size=4 align=4
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
        "main.tspp",
        "test.main.pick",
        r#"
function test.main.pick(v0: ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = type@4; 1uint1 = void; } }, managed, mutable, local>): int32 {
    local l0: ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = type@4; 1uint1 = void; } }, managed, mutable, local>

entry(v0: ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = type@4; 1uint1 = void; } }, managed, mutable, local>):
    store l0, v0
    v1: int32 = 0
    return v1
}
"#,
    );

    session.assert_mir_function(
        "main.tspp",
        "test.main.build",
        r#"
function test.main.build(): int32 {
entry:
    v0: int32 = 3
    v1: variant<uint1> { 0uint1 = int32; 1uint1 = void; } = variant.new 0, v0
    v2: variant<uint1> { 0uint1 = ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: type@5 }, managed, mutable, local>; 1uint1 = void; } = variant.new 1
    v3: { depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = ref<type@6, managed, mutable, local>; 1uint1 = void; } } = aggregate (v1, v2)
    v4: ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = type@4; 1uint1 = void; } }, managed, mutable, local> = new.complete v3
    v5: int32 = call test.main.pick(v4): (ref<{ depth: variant<uint1> { 0uint1 = int32; 1uint1 = void; }, nested: variant<uint1> { 0uint1 = type@4; 1uint1 = void; } }, managed, mutable, local>) => int32
    return v5
}

/// @layout.variant name=type@3 size=8 align=4
/// @layout.discriminant owner=type@3 kind=direct offset=0 byte_len=1 bit_offset=0 bit_len=8
/// @layout.case owner=type@3 index=0 discriminant=0 payload_offset=4
/// @layout.case owner=type@3 index=1 discriminant=1 payload_offset=4
/// @layout.variant name=type@5 size=8 align=8
/// @layout.discriminant owner=type@5 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@5 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@5 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@6 size=16 align=8
/// @layout.field owner=type@6 index=0 name=depth offset=8 size=8 align=4
/// @layout.field owner=type@6 index=1 name=nested offset=0 size=8 align=8
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
        "main.tspp",
        "test.main.build",
        r#"
function test.main.build(): int32 {
    local l0: { x: int32 }

entry:
    v0: int32 = 7
    v1: { x: int32 } = aggregate (v0)
    store l0, v1
    v2: int32 = load (l0).0
    return v2
}

/// @layout.struct name=type@1 size=4 align=4
/// @layout.field owner=type@1 index=0 name=x offset=0 size=4 align=4
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
        "main.tspp",
        "test.main.trace",
        r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.trace<'a>(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly>
    local l1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    v2: ref<String, borrowed, 'a, readonly> = load l0
    v3: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; } = load l1
    v4: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } = variant.new 1
    v5: { fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } } = aggregate (v3, v4)
    v6: ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local> = new.complete v5
    v7: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v6
    call test.main.log(v2, v7): (ref<String, borrowed, 'a, readonly>, variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }) => void
    return
}

/// @layout.struct name=type@6 size=0 align=1
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@18 size=8 align=8
/// @layout.discriminant owner=type@18 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@18 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@18 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@19 size=24 align=8
/// @layout.field owner=type@19 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@19 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
"#,
    );
    session.assert_mir_function("main.tspp", "test.main.log", r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.log<'a, 'b>(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly>
    local l1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    return
}

/// @layout.struct name=type@6 size=0 align=1
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@12 size=8 align=8
/// @layout.discriminant owner=type@12 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@12 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@12 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@13 size=24 align=8
/// @layout.field owner=type@13 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@13 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@15 size=8 align=8
/// @layout.discriminant owner=type@15 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@15 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@15 index=1 discriminant=1 payload_offset=0
"#);

    session.assert_mir_function("main.tspp", "test.main.trace", r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.trace<'a>(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: ref<String, borrowed, 'a, readonly>
    local l1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }

entry(v0: ref<String, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }):
    store l0, v0
    store l1, v1
    v2: ref<String, borrowed, 'a, readonly> = load l0
    v3: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; } = load l1
    v4: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } = variant.new 1
    v5: { fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } } = aggregate (v3, v4)
    v6: ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local> = new.complete v5
    v7: variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; } = variant.new 0, v6
    call test.main.log(v2, v7): (ref<String, borrowed, 'a, readonly>, variant<uint1> { 0uint1 = ref<{ fields: variant<uint1> { 0uint1 = dynamic<{  }, managed, mutable, local>; 1uint1 = void; }, message: variant<uint1> { 0uint1 = ref<String, borrowed, 'frame, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }) => void
    return
}

/// @layout.struct name=type@6 size=0 align=1
/// @layout.variant name=type@10 size=16 align=8
/// @layout.discriminant owner=type@10 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@10 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@10 index=1 discriminant=1 payload_offset=0
/// @layout.variant name=type@18 size=8 align=8
/// @layout.discriminant owner=type@18 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@18 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@18 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@19 size=24 align=8
/// @layout.field owner=type@19 index=0 name=fields offset=0 size=16 align=8
/// @layout.field owner=type@19 index=1 name=message offset=16 size=8 align=8
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
"#);
}

#[test]
fn test_lower_an_optional_chain_borrow_into_a_struct_field() {
    let session = TestSession::single(
        r#"
type LogOptions<'a> = {
    message?: &'a readonly string | undefined;
};

struct LogEntry<'a, 'b> {
    logger: &'a readonly string;
    message: &'b readonly string | undefined;
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
        "main.tspp",
        "test.main.Logger.log",
        r#"
@nocopy
type test.main.Logger {
    name: ref<String, managed, mutable, local>;
}

@nocopy
@languageItem("string.String")
type String;

type test.main.LogEntry<'a, 'b> {
    logger: ref<String, borrowed, 'a, readonly>;
    message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; };
}

function test.main.Logger.log<'a, 'b>(v0: ref<test.main.Logger, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = ref<{ message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }): void {
    local l0: variant<uint1> { 0uint1 = ref<{ message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }
    local l1: ref<test.main.Logger, borrowed, 'a, readonly>
    local l2: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; }
    local l3: test.main.LogEntry<'a, 'b>

entry(v0: ref<test.main.Logger, borrowed, 'a, readonly>, v1: variant<uint1> { 0uint1 = ref<{ message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, managed, mutable, local>; 1uint1 = void; }):
    store l0, v1
    store l1, v0
    v2: ref<test.main.Logger, borrowed, 'a, readonly> = load l1
    v3: ref<ref<String, managed, mutable, local>, borrowed, 'a, readonly> = address (*v2).0
    v4: ref<String, managed, mutable, local> = load (*v3)
    v5: ref<String, borrowed, 'a, readonly> = cast.bit v4 -> ref<String, borrowed, 'a, readonly>
    v6: ref<{ message: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } }, borrowed, 'managed, mutable> = address (*(l0 as 0))
    v7: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } = load (*v6).0
    store l2, v7
    jump b1

b1:
    v8: variant<uint1> { 0uint1 = ref<String, borrowed, 'b, readonly>; 1uint1 = void; } = load l2
    v9: test.main.LogEntry<'a, 'b> = aggregate (v5, v8)
    store l3, v9
    v10: ref<test.main.LogEntry<'a, 'b>, borrowed, 'frame, readonly> = address l3
    call test.main.write(v10): (ref<test.main.LogEntry<'a, 'b>, borrowed, 'frame, readonly>) => void
    return
}

/// @layout.struct name=test.main.Logger size=8 align=8
/// @layout.field owner=test.main.Logger index=0 name=name offset=0 size=8 align=8
/// @layout.struct name=test.main.LogEntry<'a, 'b> size=16 align=8
/// @layout.field owner=test.main.LogEntry<'a, 'b> index=0 name=logger offset=0 size=8 align=8
/// @layout.field owner=test.main.LogEntry<'a, 'b> index=1 name=message offset=8 size=8 align=8
/// @layout.variant name=type@21 size=8 align=8
/// @layout.discriminant owner=type@21 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@21 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@21 index=1 discriminant=1 payload_offset=0
/// @layout.struct name=type@22 size=8 align=8
/// @layout.field owner=type@22 index=0 name=message offset=0 size=8 align=8
/// @layout.variant name=type@24 size=8 align=8
/// @layout.discriminant owner=type@24 kind=niche offset=0 byte_len=8 bit_offset=0 bit_len=64 untagged=0 niche_start=0
/// @layout.case owner=type@24 index=0 discriminant=0 payload_offset=0
/// @layout.case owner=type@24 index=1 discriminant=1 payload_offset=0
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

    session.assert_mir_function("main.tspp", "test.main.Counter.constructor", r#"
@nocopy
type test.main.Counter {
    count: int32;
}

constructor test.main.Counter.constructor(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32): void {
    local l0: int32
    local l1: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>
    local l2: function<() => int32, repeatable, managed, mutable, local>

entry(v0: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable>, v1: int32):
    store l0, v1
    store l1, v0
    v2: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v3: int32 = load l0
    store (*v2).0, v3
    v4: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v5: ref<test.main.Counter, borrowed, 'managed, mutable> = cast.bit v4 -> ref<test.main.Counter, borrowed, 'managed, mutable>
    v6: ref<test.main.Counter, managed, mutable, local> = cast.bit v5 -> ref<test.main.Counter, managed, mutable, local>
    call test.main.Counter.bump(v6): (ref<test.main.Counter, managed, mutable, local>) => void
    v7: ref<uninit<test.main.Counter>, borrowed, 'managed, mutable> = load l1
    v8: ref<test.main.Counter, borrowed, 'managed, mutable> = cast.bit v7 -> ref<test.main.Counter, borrowed, 'managed, mutable>
    v9: { ref<test.main.Counter, borrowed, 'managed, mutable> } = aggregate (v8)
    v10: ref<{ ref<test.main.Counter, borrowed, 'managed, mutable> }, managed, mutable, local> = new.complete v9
    v11: function<() => int32, repeatable, managed, mutable, local> = function.bind test.main.Counter.constructor.closure#0, v10
    store l2, v11
    v12: function<() => int32, repeatable, managed, mutable, local> = load l2
    v13: function<() => int32, repeatable, borrowed, 'managed, readonly> = cast.bit v12 -> function<() => int32, repeatable, borrowed, 'managed, readonly>
    v14: int32 = call.indirect v13(): () => int32
    return
}

/// @layout.struct name=test.main.Counter size=4 align=4
/// @layout.field owner=test.main.Counter index=0 name=count offset=0 size=4 align=4
/// @layout.struct name=type@9 size=8 align=8
/// @layout.field owner=type@9 index=0 offset=0 size=8 align=8
"#);
}
