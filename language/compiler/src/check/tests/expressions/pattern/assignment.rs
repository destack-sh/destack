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
/// @type.symbol symbol=x source=x type=int32
/// @type.node source=0 type=int32

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @type.node source="\"\"" type=string

declare const point: { x: int32; y: string };
/// @type.symbol symbol=point source=point type={ x: int32; y: string }

({ x, y: label } = point);
/// @type.node source="({ x, y: label } = point)" type={ x: int32; y: string }
/// @resolution.pattern.assign source="{ x, y: label }" kind=object fields={ x, y: label }
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
/// @type.node source=0 type=int32

let last: int32 = 0;
/// @type.symbol symbol=last source=last type=int32
/// @type.node source=0 type=int32

declare const values: [int32; 3];
/// @type.symbol symbol=values source=values type=[int32; 3]

[first, , last] = values;
/// @type.node source="[first, , last] = values" type=[int32; 3]
/// @resolution.pattern.assign source="[first, , last]" kind=sequence fields=(first, last)
/// @resolution.pattern.assign source=first kind=place target=first
/// @type.node source=first type=int32
/// @resolution.name source=first target=first
/// @resolution.pattern.assign source=last kind=place target=last
/// @type.node source=last type=int32
/// @resolution.name source=last target=last
/// @type.node source=values type=[int32; 3]
/// @resolution.name source=values target=values
"#,
    );
}

#[test]
fn test_destructuring_assignment_requires_plain_assignment() {
    let session = TestSession::single(
        r#"
let first: int32 = 0;
declare const values: [int32; 1];

[first] += values;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let first: int32 = 0;
declare const values: [int32; 1];

[first] += values;

=== checked ===
let first: int32 = 0;
/// @type.symbol symbol=first source=first type=int32
/// @type.node source=0 type=int32

declare const values: [int32; 1];
/// @type.symbol symbol=values source=values type=[int32; 1]

[first] += values;
/// @type.node source="[first] += values" type=<error>
/// @resolution.pattern.assign source="[first]" kind=sequence fields=(first)
/// @resolution.pattern.assign source=first kind=place target=first
/// @type.node source=first type=int32
/// @resolution.name source=first target=first
/// @type.node source=values type=[int32; 1]
/// @resolution.name source=values target=values
"#,
        r#"
/// @diagnostic.error code=EC436 message="destructuring assignment only supports plain '='"
/// @diagnostic.label line=5 column=1 source="[first] += values;"
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
/// @type.symbol symbol=name source=name type=string
/// @type.node source="\"\"" type=string

let rest: { age: int32; active: boolean } = { age: 0, active: false };
/// @type.symbol symbol=rest source=rest type={ age: int32; active: boolean }
/// @type.node source="{ age: 0, active: false }" type={ age: int32; active: boolean }
/// @type.node source=0 type=int32
/// @type.node source=false type=boolean

declare const user: { name: string; age: int32; active: boolean };
/// @type.symbol symbol=user source=user type={ name: string; age: int32; active: boolean }

({ name, ...rest } = user);
/// @type.node source="({ name, ...rest } = user)" type={ name: string; age: int32; active: boolean }
/// @resolution.pattern.assign source="{ name, ...rest }" kind=object fields={ name } rest=...rest
/// @resolution.pattern.assign source=name kind=place target=name
/// @resolution.pattern.assign source=rest kind=place target=rest
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
/// @type.node source=0 type=int32

let tail: int32[] = [];
/// @type.symbol symbol=tail source=tail type=Array<int32>
/// @type.node source=[] type=Array<never>

declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

[head, ...tail] = values;
/// @type.node source="[head, ...tail] = values" type=Array<int32>
/// @resolution.pattern.assign source="[head, ...tail]" kind=sequence sequence=array fields=(head) rest=...tail
/// @resolution.pattern.assign source=head kind=place target=head
/// @resolution.pattern.assign source=tail kind=place target=tail
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

({ point: { x }, meta: (label) } = packet);
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

({ point: { x }, meta: (label) } = packet);

=== checked ===
let x: int32 = 0;
/// @type.symbol symbol=x source=x type=int32
/// @type.node source=0 type=int32

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @type.node source="\"\"" type=string

declare const packet: { point: { x: int32 }; meta: (string,) };
/// @type.symbol symbol=packet source=packet type={ point: { x: int32 }; meta: (string,) }

({ point: { x }, meta: (label) } = packet);
/// @type.node source="({ point: { x }, meta: (label) } = packet)" type={ point: { x: int32 }; meta: (string,) }
/// @resolution.pattern.assign source="{ point: { x }, meta: (label) }" kind=object fields={ point: pattern, meta: pattern }
/// @resolution.pattern.assign source="{ x }" kind=object fields={ x }
/// @resolution.pattern.assign source=x kind=place target=x
/// @resolution.pattern.assign source="(label)" kind=tuple fields=(label)
/// @resolution.pattern.assign source=label kind=place target=label
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

({ count = 1, labels: (label = "missing") } = packet);
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

({ count = 1, labels: (label = "missing") } = packet);

=== checked ===
let count: int32 = 0;
/// @type.symbol symbol=count source=count type=int32
/// @type.node source=0 type=int32

let label: string = "";
/// @type.symbol symbol=label source=label type=string
/// @type.node source="\"\"" type=string

declare const packet: { count?: int32; labels: (string | undefined,) };
/// @type.symbol symbol=packet source=packet type={ count?: int32; labels: (string | undefined,) }

({ count = 1, labels: (label = "missing") } = packet);
/// @type.node source="({ count = 1, labels: (label = \"missing\") } = packet)" type={ count?: int32; labels: (string | undefined,) }
/// @resolution.pattern.assign source="{ count = 1, labels: (label = \"missing\") }" kind=object fields={ count, labels: pattern }
/// @resolution.pattern.assign source="count = 1" kind=default pattern=count value=1
/// @resolution.pattern.assign source=count kind=place target=count
/// @type.node source=1 type=int32
/// @resolution.pattern.assign source="(label = \"missing\")" kind=tuple fields=(label)
/// @resolution.pattern.assign source="label = \"missing\"" kind=default pattern=label value="missing"
/// @resolution.pattern.assign source=label kind=place target=label
/// @type.node source="\"missing\"" type=string
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
/// @type.node source=0 type=int32

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

({ ["x"]: value } = point);
/// @type.node source="({ [\"x\"]: value } = point)" type={ x: int32 }
/// @resolution.pattern.assign source="{ [\"x\"]: value }" kind=object fields={ x: value }
/// @type.node source="\"x\"" type="x"
/// @resolution.pattern.assign source=value kind=place target=value
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
    );
}

#[test]
fn test_computed_assignment_pattern_accepts_index_signature_key() {
    let session = TestSession::single(
        r#"
declare const key: string;
let value: int32 = 0;
declare const bag: { [key: string]: int32 };

({ [key]: value } = bag);
"#,
    );

    session.assert_dir_checked(
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

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=0 type=int32

declare const bag: { [key: string]: int32 };
/// @type.symbol symbol=bag source=bag type={ [key: string]: int32 }

({ [key]: value } = bag);
/// @type.node source="({ [key]: value } = bag)" type={ [key: string]: int32 }
/// @resolution.pattern.assign source="{ [key]: value }" kind=object fields={ key: value }
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.pattern.assign source=value kind=place target=value
/// @type.node source=bag type={ [key: string]: int32 }
/// @resolution.name source=bag target=bag
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

let value: int32 = 0;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=0 type=int32

declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

({ [key]: value } = point);
/// @type.node source="({ [key]: value } = point)" type=<error>
/// @resolution.pattern.assign source="{ [key]: value }" kind=object fields={}
/// @type.node source=key type=string
/// @resolution.name source=key target=key
/// @resolution.pattern.assign source=value kind=place target=value
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC432 message="computed pattern key is not valid for the source type"
/// @diagnostic.label line=6 column=5 source=key
"#,
    );
}
