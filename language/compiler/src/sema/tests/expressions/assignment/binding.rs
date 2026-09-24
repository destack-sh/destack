use crate::tests::{DirRows, TestSession};

/// A binding declared inside a loop body starts each iteration uninitialized.
#[test]
fn test_read_unassigned_loop_local() {
    let session = TestSession::single(
        r#"
export function ok(): void {
    loop {
        const x = 1;
    }
}

export function fail(): void {
    loop {
        let x: int32;
        const y = x + 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
export function ok(): void {
    loop {
        const x: 1 = 1;
    }
}

export function fail(): void {
    loop {
        let x: int32;
        const y: int32 = x + 1;
    }
}

=== dir ===
export function ok(): void {
/// @type.symbol symbol=ok type=() => void

    loop {
        const x = 1;
        /// @type.symbol symbol=ok.x source=x type=1
        /// @resolution.pattern source=x kind=binding target=ok.x

    }
}

export function fail(): void {
/// @type.symbol symbol=fail type=() => void

    loop {
        let x: int32;
        /// @type.symbol symbol=fail.x source=x type=int32
        /// @resolution.pattern source=x kind=binding target=fail.x

        const y = x + 1;
        /// @type.symbol symbol=fail.y source=y type=int32
        /// @resolution.pattern source=y kind=binding target=fail.y
        /// @resolution.name source=x target=fail.x
        /// @resolution.operator source="x + 1" type=int32 operator="+" kind=builtin operands=[x as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=fail.x

    }
}
"#, r#"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=11 column=19 span="x" line_source="const y = x + 1;"
/// @diagnostic.related line=10 column=13 span="x" line_source="let x: int32;" message="declared here"
"#);
}

/// A binding read before every path assigns it is uninitialized.
#[test]
fn test_read_partially_assigned_bindings() {
    let session = TestSession::single(
        r#"
function foo(x: int32): void {}

export function uninit(): void {
    let x: int32;
    foo(x);
}

export function ifNoElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    }
    foo(x);
}

export function ifWithElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    } else {
        x = 20;
    }
    foo(x);
}

export function whileCond(): void {
    let x: boolean;
    while (x) {}
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
function foo(x: int32): void {}

export function uninit(): void {
    let x: int32;
    foo(x);
}

export function ifNoElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    }
    foo(x);
}

export function ifWithElse(flag: boolean): void {
    let x: int32;
    if (flag) {
        x = 10;
    } else {
        x = 20;
    }
    foo(x);
}

export function whileCond(): void {
    let x: boolean;
    while (x) {}
}

=== dir ===
function foo(x: int32): void {}
/// @type.symbol symbol=foo source="function foo(x: int32): void {}" type=(int32) => void
/// @type.symbol symbol=foo.x source="x: int32" type=int32

export function uninit(): void {
/// @type.symbol symbol=uninit type=() => void

    let x: int32;
    /// @type.symbol symbol=uninit.x source=x type=int32
    /// @resolution.pattern source=x kind=binding target=uninit.x

    foo(x);
    /// @resolution.name source=foo target=foo
    /// @resolution.call source=foo(x) parameters=(int32) arguments=(provided(x) as int32) return=void kind=symbol target=foo
    /// @resolution.name source=x target=uninit.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=uninit.x

}

export function ifNoElse(flag: boolean): void {
/// @type.symbol symbol=ifNoElse type=(boolean) => void
/// @type.symbol symbol=ifNoElse.flag source="flag: boolean" type=boolean

    let x: int32;
    /// @type.symbol symbol=ifNoElse.x source=x type=int32
    /// @resolution.pattern source=x kind=binding target=ifNoElse.x

    if (flag) {
    /// @resolution.name source=flag target=ifNoElse.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=ifNoElse.flag

        x = 10;
        /// @resolution.name source=x target=ifNoElse.x
        /// @resolution.pattern.assign source=x kind=place
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=ifNoElse.x
        /// @resolution.assignment source=x write=binding(ifNoElse.x) type=int32

    }
    foo(x);
    /// @resolution.name source=foo target=foo
    /// @resolution.call source=foo(x) parameters=(int32) arguments=(provided(x) as int32) return=void kind=symbol target=foo
    /// @resolution.name source=x target=ifNoElse.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=ifNoElse.x

}

export function ifWithElse(flag: boolean): void {
/// @type.symbol symbol=ifWithElse type=(boolean) => void
/// @type.symbol symbol=ifWithElse.flag source="flag: boolean" type=boolean

    let x: int32;
    /// @type.symbol symbol=ifWithElse.x source=x type=int32
    /// @resolution.pattern source=x kind=binding target=ifWithElse.x

    if (flag) {
    /// @resolution.name source=flag target=ifWithElse.flag
    /// @resolution.place source=flag placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=flag root=ifWithElse.flag

        x = 10;
        /// @resolution.name source=x target=ifWithElse.x
        /// @resolution.pattern.assign source=x kind=place
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=ifWithElse.x
        /// @resolution.assignment source=x write=binding(ifWithElse.x) type=int32

    } else {
        x = 20;
        /// @resolution.name source=x target=ifWithElse.x
        /// @resolution.pattern.assign source=x kind=place
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=ifWithElse.x
        /// @resolution.assignment source=x write=binding(ifWithElse.x) type=int32

    }
    foo(x);
    /// @resolution.name source=foo target=foo
    /// @resolution.call source=foo(x) parameters=(int32) arguments=(provided(x) as int32) return=void kind=symbol target=foo
    /// @resolution.name source=x target=ifWithElse.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=ifWithElse.x

}

export function whileCond(): void {
/// @type.symbol symbol=whileCond type=() => void

    let x: boolean;
    /// @type.symbol symbol=whileCond.x source=x type=boolean
    /// @resolution.pattern source=x kind=binding target=whileCond.x

    while (x) {}
    /// @resolution.name source=x target=whileCond.x
    /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=x root=whileCond.x

}
"#, r#"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=6 column=9 span="x" line_source="foo(x);"
/// @diagnostic.related line=5 column=9 span="x" line_source="let x: int32;" message="declared here"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=14 column=9 span="x" line_source="foo(x);"
/// @diagnostic.related line=10 column=9 span="x" line_source="let x: int32;" message="declared here"
/// @diagnostic.error id=use-before-assigned message="'x' is used before being assigned"
/// @diagnostic.label line=29 column=12 span="x" line_source="while (x) {}"
/// @diagnostic.related line=28 column=9 span="x" line_source="let x: boolean;" message="declared here"
"#);
}

#[test]
fn test_initializer_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
const value: int32 = "text";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: int32 = "text";

=== dir ===
const value: int32 = "text";
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=2 column=22 span="\"text\"" line_source="const value: int32 = \"text\";"
/// @diagnostic.related line=2 column=14 span="int32" line_source="const value: int32 = \"text\";" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_mutable_binding_accepts_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = 2;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 1;
value = 2;

=== dir ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_assignment_rejects_incompatible_value() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value = "text";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 1;
value = "text";

=== dir ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type="text"
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"text\"' is not assignable to type 'int32'"
/// @diagnostic.label line=3 column=9 span="\"text\"" line_source="value = \"text\";"
/// @diagnostic.related line=3 column=1 span="value" line_source="value = \"text\";" message="expected due to the type of this target"
"#,
    );
}

#[test]
fn test_mutable_binding_accepts_compound_assignment() {
    let session = TestSession::single(
        r#"
let value: int32 = 1;
value += 2;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 1;
value += 2;

=== dir ===
let value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value += 2;
/// @type.node source="value += 2" type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.operator source="value += 2" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 2 as int32 families=(integer)]
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.assignment source=value read=binding(value) write=binding(value) type=int32
/// @resolution.access source=value root=value
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_mutable_binding_uses_widened_initializer_type() {
    let session = TestSession::single(
        r#"
let value = 1;
value = 2;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int64 = 1;
value = 2;

=== dir ===
let value = 1;
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = 2;
/// @type.node source="value = 2" type=2
/// @type.node source=value type=int64
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int64
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_mutable_binding_rejects_assignment_outside_widened_type() {
    let session = TestSession::single(
        r#"
let value = 1;
value = "text";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int64 = 1;
value = "text";

=== dir ===
let value = 1;
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

value = "text";
/// @type.node source="value = \"text\"" type="text"
/// @type.node source=value type=int64
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int64
/// @type.node source="\"text\"" type="text"
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"text\"' is not assignable to type 'int64'"
/// @diagnostic.label line=3 column=9 span="\"text\"" line_source="value = \"text\";"
/// @diagnostic.related line=3 column=1 span="value" line_source="value = \"text\";" message="expected due to the type of this target"
"#,
    );
}

#[test]
fn test_assignment_definitely_assigns_annotated_binding() {
    let session = TestSession::single(
        r#"
let value: int32;
value = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32;
value = 1;

=== dir ===
let value: int32;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value

value = 1;
/// @type.node source="value = 1" type=1
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=int32
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_array_assignment_accepts_literal_elements() {
    let session = TestSession::single(
        r#"
let values: int32[];
values = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[];
values = [1, 2];

=== dir ===
let values: int32[];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

values = [1, 2];
/// @type.node source="values = [1, 2]" type=int32[]
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.pattern.assign source=values kind=place
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.assignment source=values write=binding(values) type=int32[]
/// @type.node source=[1, 2] type=int32[]
/// @resolution.call source=[1, 2] parameters=(^Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_empty_array_assignment_uses_target_type() {
    let session = TestSession::single(
        r#"
let values: int32[];
values = [];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: int32[];
values = [];

=== dir ===
let values: int32[];
/// @type.symbol symbol=values source=values type=int32[]
/// @resolution.pattern source=values kind=binding target=values
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

values = [];
/// @type.node source="values = []" type=int32[]
/// @type.node source=values type=int32[]
/// @resolution.name source=values target=values
/// @resolution.pattern.assign source=values kind=place
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.assignment source=values write=binding(values) type=int32[]
/// @type.node source=[] type=int32[]
/// @resolution.call source=[] parameters=(^Slice<int32>) arguments=(rest() as int32) return=int32[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<int32>
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
"#,
    );
}

#[test]
fn test_fixed_array_assignment_accepts_matching_length() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2];
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: [int32; 2];
values = [1, 2];

=== dir ===
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>
/// @resolution.pattern source=values kind=binding target=values

values = [1, 2];
/// @type.node source="values = [1, 2]" type=FixedArray<int32, 2>
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.name source=values target=values
/// @resolution.pattern.assign source=values kind=place
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.assignment source=values write=binding(values) type=FixedArray<int32, 2>
/// @type.node source=[1, 2] type=FixedArray<int32, 2>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

#[test]
fn test_fixed_array_assignment_rejects_wrong_length() {
    let session = TestSession::single(
        r#"
let values: [int32; 2];
values = [1, 2, 3];
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let values: [int32; 2];
values = [1, 2, 3];

=== dir ===
let values: [int32; 2];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 2>
/// @resolution.pattern source=values kind=binding target=values

values = [1, 2, 3];
/// @type.node source="values = [1, 2, 3]" type=FixedArray<int32, 3>
/// @type.node source=values type=FixedArray<int32, 2>
/// @resolution.name source=values target=values
/// @resolution.pattern.assign source=values kind=place
/// @resolution.place source=values placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=values root=values
/// @resolution.assignment source=values write=binding(values) type=FixedArray<int32, 2>
/// @type.node source=[1, 2, 3] type=FixedArray<int32, 3>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'FixedArray<int32, 3>' is not assignable to type 'FixedArray<int32, 2>'"
/// @diagnostic.label line=3 column=10 span="[1, 2, 3]" line_source="values = [1, 2, 3];"
/// @diagnostic.related line=3 column=1 span="values" line_source="values = [1, 2, 3];" message="expected due to the type of this target"
/// @diagnostic.note message="the mismatch is in the length: expected '2', found '3'"
"#,
    );
}

#[test]
fn test_write_definitely_assigns_binding() {
    let session = TestSession::single(
        r#"
let value: string;
value = "ready";
const copy = value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: string;
value = "ready";
const copy: string = value;

=== dir ===
let value: string;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

value = "ready";
/// @type.node source="value = \"ready\"" type="ready"
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.pattern.assign source=value kind=place
/// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=value root=value
/// @resolution.assignment source=value write=binding(value) type=string
/// @type.node source="\"ready\"" type="ready"

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_read_before_definite_assignment_reports_error() {
    let session = TestSession::single(
        r#"
let value: string;
const copy = value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: string;
const copy: string = value;

=== dir ===
let value: string;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=use-before-assigned message="'value' is used before being assigned"
/// @diagnostic.label line=3 column=14 span="value" line_source="const copy = value;"
/// @diagnostic.related line=2 column=5 span="value" line_source="let value: string;" message="declared here"
"#,
    );
}

#[test]
fn test_branch_assignment_requires_all_paths() {
    let session = TestSession::single(
        r#"
declare const condition: boolean;

let value: string;
if (condition) {
    value = "ready";
}
const copy = value;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const condition: boolean;

let value: string;
if (condition) {
    value = "ready";
}
const copy: string = value;

=== dir ===
declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean
/// @resolution.pattern source=condition kind=binding target=condition

let value: string;
/// @type.symbol symbol=value source=value type=string
/// @resolution.pattern source=value kind=binding target=value

if (condition) {
/// @type.node source=condition type=boolean
/// @resolution.name source=condition target=condition
/// @resolution.place source=condition placement="local" lifetime="static" access="immutable"
/// @resolution.access source=condition root=condition

    value = "ready";
    /// @type.node source="value = \"ready\"" type="ready"
    /// @type.node source=value type=string
    /// @resolution.name source=value target=value
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=value root=value
    /// @resolution.assignment source=value write=binding(value) type=string
    /// @type.node source="\"ready\"" type="ready"

}
const copy = value;
/// @type.symbol symbol=copy source=copy type=string
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.access source=value root=value
"#,
        r#"
/// @diagnostic.error id=use-before-assigned message="'value' is used before being assigned"
/// @diagnostic.label line=8 column=14 span="value" line_source="const copy = value;"
/// @diagnostic.related line=4 column=5 span="value" line_source="let value: string;" message="declared here"
"#,
    );
}

#[test]
fn test_undefined_initializer_models_optional_state() {
    let session = TestSession::single(
        r#"
declare const condition: boolean;

let value: string | undefined = undefined;
if (condition) {
    value = "ready";
}
const copy = value;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const condition: boolean;

let value: string | undefined = undefined as string | undefined;
if (condition) {
    value = "ready" as string | undefined;
}
const copy: string | undefined = value;

=== dir ===
declare const condition: boolean;
/// @type.symbol symbol=condition source=condition type=boolean
/// @resolution.pattern source=condition kind=binding target=condition

let value: string | undefined = undefined;
/// @type.symbol symbol=value source=value type=string | undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=undefined type=undefined

if (condition) {
/// @type.node source=condition type=boolean
/// @resolution.name source=condition target=condition
/// @resolution.place source=condition placement="local" lifetime="static" access="immutable"
/// @resolution.access source=condition root=condition

    value = "ready";
    /// @type.node source="value = \"ready\"" type="ready"
    /// @type.node source=value type=string | undefined
    /// @resolution.name source=value target=value
    /// @resolution.pattern.assign source=value kind=place
    /// @resolution.place source=value placement="local" lifetime="static" access="exclusive"
    /// @resolution.access source=value root=value
    /// @resolution.assignment source=value write=binding(value) type=string | undefined
    /// @type.node source="\"ready\"" type="ready"

}
const copy = value;
/// @type.symbol symbol=copy source=copy type=string | undefined
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=value type=string | undefined
/// @resolution.name source=value target=value
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_module_binding_read_before_declaration_reports_error() {
    let session = TestSession::single(
        r#"
const value = answer;
const answer = 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const value: 1 = answer;
const answer: 1 = 1;

=== dir ===
const value = answer;
/// @type.symbol symbol=value source=value type=1
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=answer target=answer
/// @resolution.access source=answer root=answer

const answer = 1;
/// @type.symbol symbol=answer source=answer type=1
/// @resolution.pattern source=answer kind=binding target=answer
"#,
        r#"
/// @diagnostic.error id=use-before-assigned message="'answer' is used before being assigned"
/// @diagnostic.label line=2 column=15 span="answer" line_source="const value = answer;"
/// @diagnostic.related line=3 column=7 span="answer" line_source="const answer = 1;" message="declared here"
"#,
    );
}

#[test]
fn test_module_binding_cycle_reports_use_before_assigned() {
    let session = TestSession::single(
        r#"
const a = b;
const b = a;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
const a = b;
const b = a;

=== dir ===
const a = b;
/// @type.symbol symbol=a source=a type=<error>
/// @resolution.pattern source=a kind=binding target=a
/// @resolution.name source=b target=b
/// @resolution.access source=b root=b

const b = a;
/// @type.symbol symbol=b source=b type=<error>
/// @resolution.pattern source=b kind=binding target=b
/// @resolution.name source=a target=a
/// @resolution.access source=a root=a
"#,
        r#"
/// @diagnostic.error id=use-before-assigned message="'b' is used before being assigned"
/// @diagnostic.label line=2 column=11 span="b" line_source="const a = b;"
/// @diagnostic.related line=3 column=7 span="b" line_source="const b = a;" message="declared here"
"#,
    );
}

/// Reject a type literal name as an assignment target.
#[test]
fn test_type_literal_rejects_assignment() {
    let session = TestSession::single(
        r#"
int = 5;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
int = 5;

=== dir ===
int = 5;
/// @type.node source="int = 5" type=<error>
/// @type.node source=int type=<error>
/// @resolution.rejected source=int
"#,
        r#"
/// @diagnostic.error id=non-writable-assignment-target message="assignment target is not a writable place"
/// @diagnostic.label line=2 column=1 span="int" line_source="int = 5;"
"#,
    );
}
