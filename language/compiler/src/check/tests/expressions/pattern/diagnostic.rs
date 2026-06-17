use crate::tests::{DirRows, TestSession};

#[test]
fn test_object_pattern_reports_missing_field() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { y } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { y } = point;

=== checked ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { y } = point;
/// @type.symbol symbol=y source=y type=<error>
/// @resolution.pattern source="{ y }" kind=object fields=[y]
/// @resolution.pattern source=y kind=binding target=y
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC426 message="pattern field 'y' does not exist on type '{ x: int32 }'"
/// @diagnostic.label line=4 column=7 source=y
"#,
    );
}

#[test]
fn test_nominal_object_pattern_reports_non_field_member() {
    let session = TestSession::single(
        r#"
class User {
    name: string;
    displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { displayName } => displayName
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
    displayName(): string {
        return this.name;
    }
}

declare const user: User;

match (user) {
    User { displayName } => displayName
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string
/// @definition.method symbol=User.displayName source="displayName(): string {\n        return this.name;\n    }" slot=displayName type=(this: User) => string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

    displayName(): string {
    /// @type.symbol symbol=User.displayName source="displayName(): string {\n        return this.name;\n    }" type=(this: User) => string

        return this.name;
        /// @type.node source=this.name type=string
        /// @type.node source=this type=User
        /// @resolution.member source=this.name receiver=User kind=field key=name

    }
}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

match (user) {
/// @type.node source=user type=User
/// @resolution.name source=user target=user

    User { displayName } => displayName
    /// @type.symbol symbol=displayName source=displayName type=<error>
    /// @resolution.pattern source="User { displayName }" kind=nominal_object target=User fields=[displayName]
    /// @resolution.name source=User target=User
    /// @type.node source=displayName type=<error>
    /// @resolution.name source=displayName target=displayName

}
"#,
        r#"
/// @diagnostic.error code=EC427 message="member 'displayName' on type 'User' is not a field"
/// @diagnostic.label line=11 column=12 source=displayName
"#,
    );
}

#[test]
fn test_object_pattern_reports_duplicate_field() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32 };

let { x, x: other } = point;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32 };

let { x, x: other } = point;

=== checked ===
declare const point: { x: int32 };
/// @type.symbol symbol=point source=point type={ x: int32 }

let { x, x: other } = point;
/// @type.symbol symbol=x source=x type=int32
/// @type.symbol symbol=other source=other type=int32
/// @resolution.pattern source="{ x, x: other }" kind=object fields=[x, x: other]
/// @resolution.pattern source=x kind=binding target=x
/// @resolution.pattern source=other kind=binding target=other
/// @type.node source=point type={ x: int32 }
/// @resolution.name source=point target=point
"#,
        r#"
/// @diagnostic.error code=EC428 message="field 'x' appears more than once in pattern"
/// @diagnostic.label line=4 column=10 source=x
"#,
    );
}

#[test]
fn test_object_pattern_reports_duplicate_binding_name() {
    let session = TestSession::single(
        r#"
declare const pair: { left: int32; right: int32 };

let { left: value, right: value } = pair;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const pair: { left: int32; right: int32 };

let { left: value, right: value } = pair;

=== checked ===
declare const pair: { left: int32; right: int32 };
/// @type.symbol symbol=pair source=pair type={ left: int32; right: int32 }

let { left: value, right: value } = pair;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source="{ left: value, right: value }" kind=object fields=[left: value, right: value]
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=pair type={ left: int32; right: int32 }
/// @resolution.name source=pair target=pair
"#,
        r#"
/// @diagnostic.error code=EC429 message="binding 'value' appears more than once in pattern"
/// @diagnostic.label line=4 column=27 source=value
"#,
    );
}

#[test]
fn test_sequence_pattern_reports_multiple_rest_patterns() {
    let session = TestSession::single(
        r#"
let [head, ...middle, ...tail] = [1, 2, 3];
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
let [head, ...middle, ...tail] = [1, 2, 3];

=== checked ===
let [head, ...middle, ...tail] = [1, 2, 3];
/// @type.symbol symbol=head source=head type=float64
/// @type.symbol symbol=middle source=middle type=Array<float64>
/// @type.symbol symbol=tail source=tail type=Array<float64>
/// @resolution.pattern source="[head, ...middle, ...tail]" kind=sequence sequence=array fields=[0: head] rest=...middle
/// @resolution.pattern source=head kind=binding target=head
/// @resolution.pattern source=middle kind=binding target=middle
/// @resolution.pattern source=tail kind=binding target=tail
/// @type.node source=[1, 2, 3] type=Array<float64>
/// @type.node source=1 type=float64
/// @type.node source=2 type=float64
/// @type.node source=3 type=float64
"#,
        r#"
/// @diagnostic.error code=EC431 message="pattern can contain at most one rest field"
/// @diagnostic.label line=2 column=23 source="...tail"
"#,
    );
}

#[test]
fn test_union_pattern_reports_incompatible_bindings() {
    let session = TestSession::single(
        r#"
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left } | { right } => left
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left } | { right } => left
}

=== checked ===
declare const value: { left: int32 } | { right: int32 };
/// @type.symbol symbol=value source=value type={ left: int32 } | { right: int32 }

match (value) {
/// @type.node source=value type={ left: int32 } | { right: int32 }
/// @resolution.name source=value target=value

    { left } | { right } => left
    /// @type.symbol symbol=left source=left type=<error>
    /// @type.symbol symbol=right source=right type=<error>
    /// @resolution.pattern source="{ left } | { right }" kind=union alternatives=[pattern, pattern]
    /// @resolution.pattern source="{ left }" kind=object fields=[left]
    /// @resolution.pattern source="{ right }" kind=object fields=[right]
    /// @type.node source=left type=<error>
    /// @resolution.name source=left target=left

}
"#,
        r#"
/// @diagnostic.error code=EC435 message="union pattern alternatives must bind the same names with the same forms"
/// @diagnostic.label line=5 column=5 source="{ left } | { right }"
"#,
    );
}
