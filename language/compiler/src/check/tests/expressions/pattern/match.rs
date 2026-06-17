use crate::tests::{DirRows, TestSession};

#[test]
fn test_match_literal_union_is_exhaustive() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
    "error" => "stop"
};

label satisfies "go" | "stop";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "stop" = match (status) {
    "ready" => "go"
    "error" => "stop"
};

label satisfies "go" | "stop";

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "stop"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

    "error" => "stop"
    /// @type.node source="\"error\"" type="error"
    /// @resolution.pattern source="\"error\"" kind=literal value="error"
    /// @type.node source="\"stop\"" type="stop"

};

label satisfies "go" | "stop";
/// @type.node source="label satisfies \"go\" | \"stop\"" type="go" | "stop"
/// @type.node source=label type="go" | "stop"
/// @resolution.name source=label target=label
"#,
    );
}

#[test]
fn test_match_reports_missing_literal_union_member() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" = match (status) {
    "ready" => "go"
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"

const label = match (status) {
/// @type.symbol symbol=label source=label type="go"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

};
"#,
        r#"
/// @diagnostic.error code=EC403 message="match is not exhaustive: '\"error\"' is not covered"
/// @diagnostic.label line=4 column=15 source="match (status) {\n    \"ready\" => \"go\"\n}"
"#,
    );
}

#[test]
fn test_match_guard_does_not_prove_exhaustiveness() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" if (true) => "go"
    "error" => "stop"
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "stop" = match (status) {
    "ready" if (true) => "go"
    "error" => "stop"
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "stop"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status

    "ready" if (true) => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source=true type=true
    /// @type.node source="\"go\"" type="go"

    "error" => "stop"
    /// @type.node source="\"error\"" type="error"
    /// @resolution.pattern source="\"error\"" kind=literal value="error"
    /// @type.node source="\"stop\"" type="stop"

};
"#,
        r#"
/// @diagnostic.error code=EC403 message="match is not exhaustive: '\"ready\"' is not covered"
/// @diagnostic.label line=4 column=15 source="match (status) {\n    \"ready\" if (true) => \"go\"\n    \"error\" => \"stop\"\n}"
"#,
    );
}

#[test]
fn test_match_guard_can_use_pattern_bindings() {
    let session = TestSession::single(
        r#"
declare const point: { x: int32; y: int32 };

const result = match (point) {
    { x, y } if (x == x) => y
    _ => 0
};

result satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const point: { x: int32; y: int32 };

const result: int32 = match (point) {
    { x, y } if (x == x) => y
    _ => 0
};

result satisfies int32;

=== checked ===
declare const point: { x: int32; y: int32 };
/// @type.symbol symbol=point source=point type={ x: int32; y: int32 }

const result = match (point) {
/// @type.symbol symbol=result source=result type=int32
/// @type.node source=point type={ x: int32; y: int32 }
/// @resolution.name source=point target=point

    { x, y } if (x == x) => y
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @resolution.pattern source="{ x, y }" kind=object fields={ x, y }
    /// @type.node source="x == x" type=boolean
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @type.node source=x type=int32
    /// @resolution.name source=x target=x
    /// @type.node source=y type=int32
    /// @resolution.name source=y target=y

    _ => 0
    /// @resolution.pattern source=_ kind=wildcard
    /// @type.node source="0" type=int32
    /// @type.node source=0 type=0

};

result satisfies int32;
/// @type.node source="result satisfies int32" type=int32
/// @type.node source=result type=int32
/// @resolution.name source=result target=result
"#,
    );
}

#[test]
fn test_match_object_pattern_binds_fields() {
    let session = TestSession::single(
        r#"
declare const config: { enabled: boolean; retries: int32 };

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const config: { enabled: boolean; retries: int32 };

match (config) {
    { enabled, retries } => {
        enabled satisfies boolean;
        retries satisfies int32;
    }
}

=== checked ===
declare const config: { enabled: boolean; retries: int32 };
/// @type.symbol symbol=config source=config type={ enabled: boolean; retries: int32 }

match (config) {
/// @type.node source=config type={ enabled: boolean; retries: int32 }
/// @resolution.name source=config target=config

    { enabled, retries } => {
    /// @type.symbol symbol=enabled source=enabled type=boolean
    /// @type.symbol symbol=retries source=retries type=int32
    /// @resolution.pattern source="{ enabled, retries }" kind=object fields={ enabled, retries }

        enabled satisfies boolean;
        /// @type.node source="enabled satisfies boolean" type=boolean
        /// @type.node source=enabled type=boolean
        /// @resolution.name source=enabled target=enabled

        retries satisfies int32;
        /// @type.node source="retries satisfies int32" type=int32
        /// @type.node source=retries type=int32
        /// @resolution.name source=retries target=retries

    }
}
"#,
    );
}

#[test]
fn test_match_nested_patterns_bind_leaf_values() {
    let session = TestSession::single(
        r#"
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };

match (packet) {
    { point: { x, y }, labels: [first, second] } => {
        x satisfies int32;
        y satisfies int32;
        first satisfies string;
        second satisfies string;
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };

match (packet) {
    { point: { x, y }, labels: [first, second] } => {
        x satisfies int32;
        y satisfies int32;
        first satisfies string;
        second satisfies string;
    }
}

=== checked ===
declare const packet: { point: { x: int32; y: int32 }; labels: [string; 2] };
/// @type.symbol symbol=packet source=packet type={ point: { x: int32; y: int32 }; labels: [string; 2] }

match (packet) {
/// @type.node source=packet type={ point: { x: int32; y: int32 }; labels: [string; 2] }
/// @resolution.name source=packet target=packet

    { point: { x, y }, labels: [first, second] } => {
    /// @type.symbol symbol=x source=x type=int32
    /// @type.symbol symbol=y source=y type=int32
    /// @type.symbol symbol=first source=first type=string
    /// @type.symbol symbol=second source=second type=string
    /// @resolution.pattern source="{ point: { x, y }, labels: [first, second] }" kind=object fields={ point: pattern, labels: pattern }
    /// @resolution.pattern source="{ x, y }" kind=object fields={ x, y }
    /// @resolution.pattern source="[first, second]" kind=sequence sequence=fixed_array length=2 fields=[first, second]

        x satisfies int32;
        /// @type.node source="x satisfies int32" type=int32
        /// @type.node source=x type=int32
        /// @resolution.name source=x target=x

        y satisfies int32;
        /// @type.node source="y satisfies int32" type=int32
        /// @type.node source=y type=int32
        /// @resolution.name source=y target=y

        first satisfies string;
        /// @type.node source="first satisfies string" type=string
        /// @type.node source=first type=string
        /// @resolution.name source=first target=first

        second satisfies string;
        /// @type.node source="second satisfies string" type=string
        /// @type.node source=second type=string
        /// @resolution.name source=second target=second

    }
}
"#,
    );
}

#[test]
fn test_match_nominal_object_pattern_binds_class_fields() {
    let session = TestSession::single(
        r#"
class User {
    name: string;
}

declare const user: User;

match (user) {
    User { name } => name satisfies string
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string;
}

declare const user: User;

match (user) {
    User { name } => name satisfies string
}

=== checked ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.name source=User target=User

match (user) {
/// @type.node source=user type=User
/// @resolution.name source=user target=user

    User { name } => name satisfies string
    /// @type.symbol symbol=name source=name type=string
    /// @resolution.pattern source="User { name }" kind=nominal_object target=User fields={ name }
    /// @resolution.name source=User target=User
    /// @type.node source="name satisfies string" type=string
    /// @type.node source=name type=string
    /// @resolution.name source=name target=name

}
"#,
    );
}

#[test]
fn test_match_rest_pattern_binds_remaining_sequence() {
    let session = TestSession::single(
        r#"
declare const values: int32[];

match (values) {
    [head, ...tail] => {
        head satisfies int32;
        tail satisfies int32[];
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const values: int32[];

match (values) {
    [head, ...tail] => {
        head satisfies int32;
        tail satisfies int32[];
    }
}

=== checked ===
declare const values: int32[];
/// @type.symbol symbol=values source=values type=Array<int32>

match (values) {
/// @type.node source=values type=Array<int32>
/// @resolution.name source=values target=values

    [head, ...tail] => {
    /// @type.symbol symbol=head source=head type=int32
    /// @type.symbol symbol=tail source=tail type=Array<int32>
    /// @resolution.pattern source="[head, ...tail]" kind=sequence sequence=array fields=[head] rest=...tail

        head satisfies int32;
        /// @type.node source="head satisfies int32" type=int32
        /// @type.node source=head type=int32
        /// @resolution.name source=head target=head

        tail satisfies int32[];
        /// @type.node source="tail satisfies int32[]" type=Array<int32>
        /// @type.node source=tail type=Array<int32>
        /// @resolution.name source=tail target=tail

    }
}
"#,
    );
}

#[test]
fn test_match_union_pattern_requires_compatible_bindings() {
    let session = TestSession::single(
        r#"
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left: item } | { right: item } => item satisfies int32
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const value: { left: int32 } | { right: int32 };

match (value) {
    { left: item } | { right: item } => item satisfies int32
}

=== checked ===
declare const value: { left: int32 } | { right: int32 };
/// @type.symbol symbol=value source=value type={ left: int32 } | { right: int32 }

match (value) {
/// @type.node source=value type={ left: int32 } | { right: int32 }
/// @resolution.name source=value target=value

    { left: item } | { right: item } => item satisfies int32
    /// @type.symbol symbol=item source=item type=int32
    /// @resolution.pattern source="{ left: item } | { right: item }" kind=union alternatives=[pattern, pattern]
    /// @resolution.pattern source="{ left: item }" kind=object fields={ left: item }
    /// @resolution.pattern source="{ right: item }" kind=object fields={ right: item }
    /// @type.node source="item satisfies int32" type=int32
    /// @type.node source=item type=int32
    /// @resolution.name source=item target=item

}
"#,
    );
}

#[test]
fn test_match_wildcard_fallback_uses_remaining_branch_type() {
    let session = TestSession::single(
        r#"
declare const status: "ready" | "error";

const label = match (status) {
    "ready" => "go"
    _ => status
};
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const status: "ready" | "error";

const label: "go" | "error" = match (status) {
    "ready" => "go"
    _ => status
};

=== checked ===
declare const status: "ready" | "error";
/// @type.symbol symbol=status source=status type="ready" | "error"

const label = match (status) {
/// @type.symbol symbol=label source=label type="go" | "error"
/// @type.node source=status type="ready" | "error"
/// @resolution.name source=status target=status

    "ready" => "go"
    /// @type.node source="\"ready\"" type="ready"
    /// @resolution.pattern source="\"ready\"" kind=literal value="ready"
    /// @type.node source="\"go\"" type="go"

    _ => status
    /// @resolution.pattern source=_ kind=wildcard
    /// @type.node source=status type="error"
    /// @resolution.name source=status target=status

};
"#,
    );
}
