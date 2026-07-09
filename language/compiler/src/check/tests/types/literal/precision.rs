use crate::tests::{DirRows, TestSession};

#[test]
fn test_const_number_keeps_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 42 = 42;

=== checked ===
const value = 42;
/// @type.symbol symbol=value source=value type=42
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_let_number_widens_binding_to_numeric_type() {
    let session = TestSession::single(
        r#"
let value = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
let value: float64 = 42;

=== checked ===
let value = 42;
/// @type.symbol symbol=value source=value type=float64
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_type_annotation_sets_scalar_binding_type() {
    let session = TestSession::single(
        r#"
const value: int32 = 42;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: int32 = 42;

=== checked ===
const value: int32 = 42;
/// @type.symbol symbol=value source=value type=int32
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_string_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: "ready" = "ready";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: "ready" = "ready";

=== checked ===
const value: "ready" = "ready";
/// @type.symbol symbol=value source=value type="ready"
/// @type.node source="\"ready\"" type="ready"

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_boolean_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: true = true;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: true = true;

=== checked ===
const value: true = true;
/// @type.symbol symbol=value source=value type=true
/// @type.node source=true type=true

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_null_literal_keeps_null_type() {
    let session = TestSession::single(
        r#"
const value = null;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: null = null;

=== checked ===
const value = null;
/// @type.symbol symbol=value source=value type=null
/// @type.node source=null type=null

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_undefined_literal_keeps_undefined_type() {
    let session = TestSession::single(
        r#"
const value = undefined;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: undefined = undefined;

=== checked ===
const value = undefined;
/// @type.symbol symbol=value source=value type=undefined
/// @type.node source=undefined type=undefined

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_satisfies_preserves_scalar_literal_type() {
    let session = TestSession::single(
        r#"
const value = 42 satisfies int32;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: 42 = 42 satisfies int32;

=== checked ===
const value = 42 satisfies int32;
/// @type.symbol symbol=value source=value type=42
/// @type.node source="42 satisfies int32" type=42
/// @type.node source=42 type=42

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0
"#,
    );
}

#[test]
fn test_const_object_member_widens_without_const_assertion() {
    let session = TestSession::single(
        r#"
const config = { version: 1 };
const version = config.version;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const config: { version: float64 } = { version: 1 };
const version: float64 = config.version;

=== checked ===
const config = { version: 1 };
/// @type.symbol symbol=config source=config type={ version: float64 }
/// @type.node source={ version: 1 } type={ version: 1 }
/// @type.node source=1 type=1

const version = config.version;
/// @type.symbol symbol=version source=version type=float64
/// @type.node source=config type={ version: float64 }
/// @type.node source=config.version type=float64
/// @resolution.name source=config target=config
/// @resolution.member source=config.version receiver={ version: float64 } kind=field key=version

/// @check.stats.solve variables=0 types=5 constraints=0 obligations=0 solutions=0 bounds=0 decisions=2
"#,
    );
}

#[test]
fn test_const_assertion_preserves_nested_object_literals() {
    let session = TestSession::single(
        r#"
const config = { nested: { mode: "dev" } } as const;
const mode = config.nested.mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const config: { readonly nested: { readonly mode: "dev" } } = { nested: { mode: "dev" } } as const;
const mode: "dev" = config.nested.mode;

=== checked ===
const config = { nested: { mode: "dev" } } as const;
/// @type.symbol symbol=config source=config type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source="{ nested: { mode: \"dev\" } } as const" type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source={ nested: { mode: "dev" } } type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source={ mode: "dev" } type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = config.nested.mode;
/// @type.symbol symbol=mode source=mode type="dev"
/// @type.node source=config type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source=config.nested type={ readonly mode: "dev" }
/// @type.node source=config.nested.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.nested receiver={ readonly nested: { readonly mode: "dev" } } kind=field key=nested
/// @resolution.member source=config.nested.mode receiver={ readonly mode: "dev" } kind=field key=mode

/// @check.stats.solve variables=0 types=4 constraints=0 obligations=0 solutions=0 bounds=0 decisions=3
"#,
    );
}

#[test]
fn test_const_assertion_through_satisfies_preserves_nested_literals() {
    let session = TestSession::single(
        r#"
const value = { env: { mode: "dev" } } as const satisfies { env: { mode: string } };
const mode = value.env.mode;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== annotated ===
const value: { readonly env: { readonly mode: "dev" } } = {
    env: { mode: "dev" },
} as const satisfies { env: { mode: string } };
const mode: "dev" = value.env.mode;

=== checked ===
const value = { env: { mode: "dev" } } as const satisfies { env: { mode: string } };
/// @type.symbol symbol=value source=value type={ readonly env: { readonly mode: "dev" } }
/// @type.node source="{ env: { mode: \"dev\" } } as const" type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ env: { mode: "dev" } } as const satisfies { env: { mode: string } } type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ env: { mode: "dev" } } type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ mode: "dev" } type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = value.env.mode;
/// @type.symbol symbol=mode#2 source=mode type="dev"
/// @type.node source=value type={ readonly env: { readonly mode: "dev" } }
/// @type.node source=value.env type={ readonly mode: "dev" }
/// @type.node source=value.env.mode type="dev"
/// @resolution.name source=value target=value
/// @resolution.member source=value.env receiver={ readonly env: { readonly mode: "dev" } } kind=field key=env
/// @resolution.member source=value.env.mode receiver={ readonly mode: "dev" } kind=field key=mode

/// @check.stats.solve variables=0 types=7 constraints=0 obligations=0 solutions=0 bounds=0 decisions=3
"#,
    );
}

#[test]
fn test_exported_const_literal_keeps_precision_across_import() {
    let session = TestSession::builder()
        .module(
            "values.ds",
            r#"
export const version = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import { version } from "./values.ds";

const copy = version;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["values.ds", "main.ds"],
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== values.ds ===

=== annotated ===
export const version: 1 = 1;

=== checked ===
export const version = 1;
/// @type.symbol symbol=version source=version type=1
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=2 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0

=== main.ds ===

=== annotated ===
import { version } from "./values.ds";

const copy: 1 = version;

=== checked ===
import { version } from "./values.ds";

const copy = version;
/// @type.symbol symbol=copy source=copy type=1
/// @type.node source=version type=1
/// @resolution.name source=version target=values.version

/// @check.stats.solve variables=0 types=1 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}

#[test]
fn test_exported_let_literal_widens_across_import() {
    let session = TestSession::builder()
        .module(
            "values.ds",
            r#"
export let counter = 1;
"#,
        )
        .module(
            "main.ds",
            r#"
import { counter } from "./values.ds";

const copy = counter;
"#,
        )
        .build();

    session.assert_dir_checked_many(
        &["values.ds", "main.ds"],
        DirRows::checked().with_reference_types().with_check_stats(),
        r#"
=== values.ds ===

=== annotated ===
export let counter: float64 = 1;

=== checked ===
export let counter = 1;
/// @type.symbol symbol=counter source=counter type=float64
/// @type.node source=1 type=1

/// @check.stats.solve variables=0 types=3 constraints=0 obligations=0 solutions=0 bounds=0 decisions=0

=== main.ds ===

=== annotated ===
import { counter } from "./values.ds";

const copy: float64 = counter;

=== checked ===
import { counter } from "./values.ds";

const copy = counter;
/// @type.symbol symbol=copy source=copy type=float64
/// @type.node source=counter type=float64
/// @resolution.name source=counter target=values.counter

/// @check.stats.solve variables=0 types=1 constraints=0 obligations=0 solutions=0 bounds=0 decisions=1
"#,
    );
}
