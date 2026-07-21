use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_assignment_pattern_resolves_targets() {
    let session = TestSession::single(
        r#"
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let x: int32 = 0;
let label: string = "";
declare const point: { x: int32; y: string };

({ x, y: label } = point);

=== checked ===
let x: int32 = 0;
/// @type.symbol symbol=x#1 source=x type=int32
/// @resolution.pattern source=x kind=binding target=x#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const point: { x: int32; y: string };
/// @type.symbol symbol=point source=point type={ x: int32; y: string }
/// @resolution.pattern source=point kind=binding target=point

({ x, y: label } = point);
/// @type.node source="{ x, y: label } = point" type={ x: int32; y: string }
/// @resolution.pattern.assign source={ x, y: label } kind=object fields={ x, y: label }
/// @type.node source=x type=int32
/// @resolution.pattern.assign source=x kind=place place=binding(x#1) type=int32
/// @type.node source=label type=string
/// @resolution.pattern.assign source=label kind=place place=binding(label) type=string
/// @type.node source=point type={ x: int32; y: string }
/// @resolution.name source=point target=point
"#,
    );
}

#[test]
fn test_sequence_assignment_pattern_resolves_targets() {
    let session = TestSession::single(
        r#"
let first: int32 = 0;
let last: int32 = 0;
declare const values: [int32; 3];

[first, , last] = values;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let first: int32 = 0;
let last: int32 = 0;
declare const values: [int32; 3];

[first, , last] = values;

=== checked ===
let first: int32 = 0;
/// @type.symbol symbol=first source=first type=int32
/// @resolution.pattern source=first kind=binding target=first
/// @type.node source=0 type=0

let last: int32 = 0;
/// @type.symbol symbol=last source=last type=int32
/// @resolution.pattern source=last kind=binding target=last
/// @type.node source=0 type=0

declare const values: [int32; 3];
/// @type.symbol symbol=values source=values type=FixedArray<int32, 3>
/// @resolution.pattern source=values kind=binding target=values

[first, , last] = values;
/// @type.node source="[first, , last] = values" type=FixedArray<int32, 3>
/// @resolution.pattern.assign source=[first, , last] kind=sequence element=int32 arity=3 fields=(first, last)
/// @type.node source=first type=int32
/// @resolution.pattern.assign source=first kind=place place=binding(first) type=int32
/// @type.node source=last type=int32
/// @resolution.pattern.assign source=last kind=place place=binding(last) type=int32
/// @type.node source=values type=FixedArray<int32, 3>
/// @resolution.name source=values target=values
"#,
    );
}

#[test]
fn test_object_assignment_pattern_resolves_rest_target() {
    let session = TestSession::single(
        r#"
let name: string = "";
let rest: { age: int32; active: boolean } = { age: 0, active: false };
declare const user: { name: string; age: int32; active: boolean };

({ name, ...rest } = user);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let name: string = "";
let rest: { age: int32; active: boolean } = { age: 0, active: false };
declare const user: { name: string; age: int32; active: boolean };

({ name, ...rest } = user);

=== checked ===
let name: string = "";
/// @type.symbol symbol=name#1 source=name type=string
/// @resolution.pattern source=name kind=binding target=name#1
/// @type.node source="\"\"" type=""

let rest: { age: int32; active: boolean } = { age: 0, active: false };
/// @type.symbol symbol=rest source=rest type={ age: int32; active: boolean }
/// @resolution.pattern source=rest kind=binding target=rest
/// @type.node source={ age: 0, active: false } type={ age: 0; active: false }
/// @type.node source=0 type=0
/// @type.node source=false type=false

declare const user: { name: string; age: int32; active: boolean };
/// @type.symbol symbol=user source=user type={ name: string; age: int32; active: boolean }
/// @resolution.pattern source=user kind=binding target=user

({ name, ...rest } = user);
/// @type.node source="{ name, ...rest } = user" type={ name: string; age: int32; active: boolean }
/// @resolution.pattern.assign source={ name, ...rest } kind=object fields={ name } rest=...rest
/// @type.node source=name type=string
/// @resolution.pattern.assign source=name kind=place place=binding(name#1) type=string
/// @type.node source=rest type={ age: int32; active: boolean }
/// @resolution.pattern.assign source=rest kind=place place=binding(rest) type={ age: int32; active: boolean }
/// @type.node source=user type={ name: string; age: int32; active: boolean }
/// @resolution.name source=user target=user
"#,
    );
}

#[test]
fn test_sequence_assignment_pattern_resolves_rest_target() {
    let session = TestSession::single(
        r#"
let head: int32 = 0;
let tail: int32[] = [];
declare const values: int32[];

[head, ...tail] = values;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let head: int32 = 0;
let tail: int32[] = [];
declare const values: int32[];

[head, ...tail] = values;

=== checked ===
let head: int32 = 0;
/// @type.symbol symbol=head source=head type=int32
/// @resolution.pattern source=head kind=binding target=head
/// @type.node source=0 type=0

let tail: int32[] = [];
/// @type.symbol symbol=tail source=tail type=Array<int32>
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=[] type=Array<int32>

declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>
/// @resolution.pattern source=values kind=binding target=values

[head, ...tail] = values;
/// @type.node source="[head, ...tail] = values" type=Array<int32>
/// @resolution.pattern.assign source=[head, ...tail] kind=sequence element=int32 arity=1.. fields=(head) rest=...tail
/// @type.node source=head type=int32
/// @resolution.pattern.assign source=head kind=place place=binding(head) type=int32
/// @type.node source=tail type=Array<int32>
/// @resolution.pattern.assign source=tail kind=place place=binding(tail) type=Array<int32>
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values
"#,
    );
}

#[test]
fn test_nested_assignment_pattern_resolves_leaf_targets() {
    let session = TestSession::single(
        r#"
let x: int32 = 0;
let label: string = "";
declare const packet: { point: { x: int32 }; meta: (string,) };

({
    point: { x },
    meta: (label,),
} = packet);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let x: int32 = 0;
let label: string = "";
declare const packet: { point: { x: int32 }; meta: (string,) };

({
    point: { x },
    meta: (label,),
} = packet);

=== checked ===
let x: int32 = 0;
/// @type.symbol symbol=x#1 source=x type=int32
/// @resolution.pattern source=x kind=binding target=x#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const packet: { point: { x: int32 }; meta: (string,) };
/// @type.symbol symbol=packet source=packet type={ point: { x: int32 }; meta: (string,) }
/// @resolution.pattern source=packet kind=binding target=packet

({
/// @type.node type={ point: { x: int32 }; meta: (string,) }
/// @resolution.pattern.assign kind=object fields={ point: assignment_pattern, meta: assignment_pattern }

    point: { x },
    /// @resolution.pattern.assign source={ x } kind=object fields={ x }
    /// @type.node source=x type=int32
    /// @resolution.pattern.assign source=x kind=place place=binding(x#1) type=int32

    meta: (label,),
    /// @resolution.pattern.assign source=(label,) kind=tuple fields=(label)
    /// @type.node source=label type=string
    /// @resolution.pattern.assign source=label kind=place place=binding(label) type=string

} = packet);
/// @type.node source=packet type={ point: { x: int32 }; meta: (string,) }
/// @resolution.name source=packet target=packet
"#,
    );
}

#[test]
fn test_assignment_pattern_default_resolves_inside_object_and_sequence() {
    let session = TestSession::single(
        r#"
let count: int32 = 0;
let label: string = "";
declare const packet: { count?: int32; labels: (string | undefined,) };

({
    count = 1,
    labels: (label = "missing",),
} = packet);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let count: int32 = 0;
let label: string = "";
declare const packet: { count?: int32; labels: (string | undefined,) };

({
    count = 1,
    labels: (label = "missing",),
} = packet);

=== checked ===
let count: int32 = 0;
/// @type.symbol symbol=count#1 source=count type=int32
/// @resolution.pattern source=count kind=binding target=count#1
/// @type.node source=0 type=0

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @resolution.pattern source=label kind=binding target=label
/// @type.node source="\"\"" type=""

declare const packet: { count?: int32; labels: (string | undefined,) };
/// @type.symbol symbol=packet source=packet type={ count?: int32; labels: (string | undefined,) }
/// @resolution.pattern source=packet kind=binding target=packet

({
/// @type.node type={ count?: int32; labels: (string | undefined,) }
/// @resolution.pattern.assign kind=object fields={ count, labels: assignment_pattern }

    count = 1,
    /// @type.node source=count type=int32
    /// @resolution.pattern.assign source="count = 1" kind=default pattern=count value=expression
    /// @resolution.pattern.assign source=count kind=place place=binding(count#1) type=int32
    /// @type.node source=1 type=1

    labels: (label = "missing",),
    /// @resolution.pattern.assign source=(label = "missing",) kind=tuple fields=(label)
    /// @type.node source=label type=string
    /// @resolution.pattern.assign source="label = \"missing\"" kind=default pattern=label value=expression
    /// @resolution.pattern.assign source=label kind=place place=binding(label) type=string
    /// @type.node source="\"missing\"" type="missing"

} = packet);
/// @type.node source=packet type={ count?: int32; labels: (string | undefined,) }
/// @resolution.name source=packet target=packet
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_accepts_static_key() {
    let session = TestSession::single(
        r#"
let value: int32 = 0;
declare const point: { x: int32 };

({ ["x"]: value } = point);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let value: int32 = 0;
declare const point: { x: int32 };

({ ["x"]: value } = point);

=== checked ===
let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point

({ ["x"]: value } = point);
/// @type.node source="{ [\"x\"]: value } = point" type={ x: int32 }
/// @resolution.pattern.assign source={ ["x"]: value } kind=object fields={ x: value }
/// @type.node source="\"x\"" type="x"
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_rejects_partial_index_signature_key() {
    let session = TestSession::single(
        r#"
declare const key: string;
let value: int32 = 0;
declare const bag: { [key: string]: int32 };

({ [key]: value } = bag);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
let value: int32 = 0;
declare const bag: { [key: string]: int32 };

({ [key]: value } = bag);

=== checked ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const bag: { [key: string]: int32 };
/// @type.symbol symbol=bag source=bag type={ [key: string]: int32 }
/// @resolution.pattern source=bag kind=binding target=bag

({ [key]: value } = bag);
/// @type.node source="{ [key]: value } = bag" type={ [key: string]: int32 }
/// @resolution.pattern.assign source={ [key]: value } kind=object fields={ key: value }
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @type.node source=value type=int32
/// @resolution.pattern.assign source=value kind=place place=binding(value) type=int32
/// @type.node source=bag type={ [key: string]: int32 }
/// @resolution.name source=bag target=bag
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'int32 | undefined' is not assignable to type 'int32'"
/// @diagnostic.label line=6 column=11 span="value" line_source="({ [key]: value } = bag);"
/// @diagnostic.note message="expected 'int32', found 'undefined'"
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_rejects_unbounded_key_for_finite_shape() {
    let session = TestSession::single(
        r#"
declare const key: string;
let value: int32 = 0;
declare const point: { x: int32 };

({ [key]: value } = point);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const key: string;
let value: int32 = 0;
declare const point: { x: int32 };

({ [key]: value } = point);

=== checked ===
declare const key: string;
/// @type.symbol symbol=key source=key type=string
/// @resolution.pattern source=key kind=binding target=key

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=0 type=0

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }
/// @resolution.pattern source=point kind=binding target=point

({ [key]: value } = point);
/// @type.node source="{ [key]: value } = point" type=<error>
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error id=computed-pattern-key-not-valid message="computed pattern key is not valid for the source type"
/// @diagnostic.label line=6 column=5 span="key" line_source="({ [key]: value } = point);"
"#,
    );
}
