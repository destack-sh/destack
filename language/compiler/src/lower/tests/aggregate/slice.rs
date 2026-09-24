use crate::tests::TestSession;

/// Lower a borrowed slice parameter to a fat descriptor.
#[test]
fn test_lower_borrowed_slice_parameters_to_fat_descriptors() {
    let session = TestSession::single(
        r#"
function measure(values: &readonly [int32]): int32 {
    return 7;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.measure",
        r#"
function test.main.measure<'a>(v0: slice<int32, borrowed, 'a, readonly>): int32 {
    local l0: slice<int32, borrowed, 'a, readonly>

entry(v0: slice<int32, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = 7
    return v1
}
"#,
    );
}

/// Lower a string parameter through the String representation class.
#[test]
fn test_lower_string_parameters_through_the_representation_class() {
    let session = TestSession::single(
        r#"
function keep(name: string): string {
    return name;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.keep", r#"
@nocopy
@languageItem("string.String")
type String;

function test.main.keep(v0: ref<String, managed, mutable, local>): ref<String, managed, mutable, local> {
    local l0: ref<String, managed, mutable, local>

entry(v0: ref<String, managed, mutable, local>):
    store l0, v0
    v1: ref<String, managed, mutable, local> = load l0
    return v1
}
"#);
}

/// Lower a borrow of a slice newtype to a fat descriptor.
#[test]
fn test_lower_newtype_slice_borrows_to_fat_descriptors() {
    let session = TestSession::single(
        r#"
newtype Bytes = [uint8];

function measure(view: &readonly Bytes): int32 {
    return 7;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.measure",
        r#"
function test.main.measure<'a>(v0: slice<uint8, borrowed, 'a, readonly>): int32 {
    local l0: slice<uint8, borrowed, 'a, readonly>

entry(v0: slice<uint8, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = 7
    return v1
}
"#,
    );
}

/// Lower a borrow of a nested generic slice newtype at the outer argument's element.
#[test]
fn test_lower_a_nested_generic_slice_newtype_borrow_at_its_element() {
    let session = TestSession::single(
        r#"
newtype Run<U> = [U];
newtype Frame<T> = Run<T>;

function measure(view: &readonly Frame<uint8>): int32 {
    return 7;
}
"#,
    );

    session.assert_mir_function(
        "main.ds",
        "test.main.measure",
        r#"
function test.main.measure<'a>(v0: slice<uint8, borrowed, 'a, readonly>): int32 {
    local l0: slice<uint8, borrowed, 'a, readonly>

entry(v0: slice<uint8, borrowed, 'a, readonly>):
    store l0, v0
    v1: int32 = 7
    return v1
}
"#,
    );
}

/// A subscript write on a borrowed slice calls the selected index set.
#[test]
fn test_lower_a_subscript_write_through_the_index_set_call() {
    let session = TestSession::single(
        r#"
function fill(values: &[int32], value: int32): void {
    values[0] = value;
}
"#,
    );

    session.assert_mir_function("main.ds", "test.main.fill", r#"
function test.main.fill<'a>(v0: slice<int32, borrowed, 'a, mutable>, v1: int32): void {
    local l0: slice<int32, borrowed, 'a, mutable>
    local l1: int32

entry(v0: slice<int32, borrowed, 'a, mutable>, v1: int32):
    store l0, v0
    store l1, v1
    v2: slice<int32, borrowed, 'a, mutable> = load l0
    v3: isize = 0
    v4: int32 = load l1
    call Slice.IndexSet.indexSet<int32>(v2, v3, v4): (slice<int32, borrowed, 'a, mutable>, isize, int32) => void
    return
}
"#);
}
