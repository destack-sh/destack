use crate::tests::{DirRows, TestSession};

#[test]
fn test_string_literal_annotation_preserves_literal_type() {
    let session = TestSession::single(
        r#"
const value: "ready" = "ready";
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: "ready" = "ready";

=== dir ===
const value: "ready" = "ready";
/// @type.symbol symbol=value source=value type="ready"
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="\"ready\"" type="ready"
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: true = true;

=== dir ===
const value: true = true;
/// @type.symbol symbol=value source=value type=true
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=true type=true
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: null = null;

=== dir ===
const value = null;
/// @type.symbol symbol=value source=value type=null
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=null type=null
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: undefined = undefined;

=== dir ===
const value = undefined;
/// @type.symbol symbol=value source=value type=undefined
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=undefined type=undefined
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: 42 = 42 satisfies int32;

=== dir ===
const value = 42 satisfies int32;
/// @type.symbol symbol=value source=value type=42
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="42 satisfies int32" type=42
/// @type.node source=42 type=42
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const config: { version: int64 } = { version: 1 };
const version: int64 = config.version;

=== dir ===
const config = { version: 1 };
/// @type.symbol symbol=config source=config type={ version: int64 }
/// @resolution.pattern source=config kind=binding target=config
/// @type.node source={ version: 1 } type={ version: int64 }
/// @type.node source=1 type=1

const version = config.version;
/// @type.symbol symbol=version source=version type=int64
/// @resolution.pattern source=version kind=binding target=version
/// @type.node source=config type={ version: int64 }
/// @type.node source=config.version type=int64
/// @resolution.name source=config target=config
/// @resolution.member source=config.version receiver={ version: int64 } type=int64 kind=field target_receiver={ version: int64 } key=version target_type=int64
/// @resolution.place source=config placement="local" lifetime="static" access="immutable"
/// @resolution.access source=config root=config
/// @resolution.access source=config.version root=config keys=[version]
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const config: { readonly nested: { readonly mode: "dev" } } = { nested: { mode: "dev" } } as const;
const mode: "dev" = config.nested.mode;

=== dir ===
const config = { nested: { mode: "dev" } } as const;
/// @type.symbol symbol=config source=config type={ readonly nested: { readonly mode: "dev" } }
/// @resolution.pattern source=config kind=binding target=config
/// @type.node source="{ nested: { mode: \"dev\" } } as const" type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source={ nested: { mode: "dev" } } type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source={ mode: "dev" } type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = config.nested.mode;
/// @type.symbol symbol=mode source=mode type="dev"
/// @resolution.pattern source=mode kind=binding target=mode
/// @type.node source=config type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source=config.nested type={ readonly mode: "dev" }
/// @type.node source=config.nested.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.nested receiver={ readonly nested: { readonly mode: "dev" } } type={ readonly mode: "dev" } kind=field target_receiver={ readonly nested: { readonly mode: "dev" } } key=nested target_type={ readonly mode: "dev" }
/// @resolution.member source=config.nested.mode receiver={ readonly mode: "dev" } type="dev" kind=field target_receiver={ readonly mode: "dev" } key=mode target_type="dev"
/// @resolution.place source=config placement="local" lifetime="static" access="immutable"
/// @resolution.access source=config root=config
/// @resolution.place source=config.nested placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=config.nested root=config keys=[nested]
/// @resolution.access source=config.nested.mode root=config keys=[nested, mode]
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
const value: { readonly env: { readonly mode: "dev" } } = {
    env: { mode: "dev" },
} as const satisfies { env: { mode: string } };
const mode: "dev" = value.env.mode;

=== dir ===
const value = { env: { mode: "dev" } } as const satisfies { env: { mode: string } };
/// @type.symbol symbol=value source=value type={ readonly env: { readonly mode: "dev" } }
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="{ env: { mode: \"dev\" } } as const" type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ env: { mode: "dev" } } as const satisfies { env: { mode: string } } type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ env: { mode: "dev" } } type={ readonly env: { readonly mode: "dev" } }
/// @type.node source={ mode: "dev" } type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"
/// @type.symbol symbol=env source="env: { mode: string }" type={ mode: string }
/// @type.symbol symbol=mode#1 source="mode: string" type=string

const mode = value.env.mode;
/// @type.symbol symbol=mode#2 source=mode type="dev"
/// @resolution.pattern source=mode kind=binding target=mode#2
/// @type.node source=value type={ readonly env: { readonly mode: "dev" } }
/// @type.node source=value.env type={ readonly mode: "dev" }
/// @type.node source=value.env.mode type="dev"
/// @resolution.name source=value target=value
/// @resolution.member source=value.env receiver={ readonly env: { readonly mode: "dev" } } type={ readonly mode: "dev" } kind=field target_receiver={ readonly env: { readonly mode: "dev" } } key=env target_type={ readonly mode: "dev" }
/// @resolution.member source=value.env.mode receiver={ readonly mode: "dev" } type="dev" kind=field target_receiver={ readonly mode: "dev" } key=mode target_type="dev"
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
/// @resolution.access source=value root=value
/// @resolution.place source=value.env placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=value.env root=value keys=[env]
/// @resolution.access source=value.env.mode root=value keys=[env, mode]
"#,
    );
}

#[test]
fn test_exported_const_literal_keeps_precision_across_import() {
    let session = TestSession::builder()
        .module(
            "values.tspp",
            r#"
export const version = 1;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { version } from "./values.tspp";

const copy = version;
"#,
        )
        .build();

    session.assert_dir_many(
        &["values.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== values.tspp ===

=== annotated ===
export const version: 1 = 1;

=== dir ===
export const version = 1;
/// @type.symbol symbol=version source=version type=1
/// @resolution.pattern source=version kind=binding target=version
/// @type.node source=1 type=1

=== main.tspp ===

=== annotated ===
import { version } from "./values.tspp";

const copy: 1 = version;

=== dir ===
import { version } from "./values.tspp";

const copy = version;
/// @type.symbol symbol=copy source=copy type=1
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=version type=1
/// @resolution.name source=version target=values.version
/// @resolution.place source=version placement="local" lifetime="static" access="immutable"
/// @resolution.access source=version root=values.version
"#,
    );
}

#[test]
fn test_exported_let_literal_widens_across_import() {
    let session = TestSession::builder()
        .module(
            "values.tspp",
            r#"
export let counter = 1;
"#,
        )
        .module(
            "main.tspp",
            r#"
import { counter } from "./values.tspp";

const copy = counter;
"#,
        )
        .build();

    session.assert_dir_many(
        &["values.tspp", "main.tspp"],
        DirRows::checked().with_reference_types(),
        r#"
=== values.tspp ===

=== annotated ===
export let counter: int64 = 1;

=== dir ===
export let counter = 1;
/// @type.symbol symbol=counter source=counter type=int64
/// @resolution.pattern source=counter kind=binding target=counter
/// @type.node source=1 type=1

=== main.tspp ===

=== annotated ===
import { counter } from "./values.tspp";

const copy: int64 = counter;

=== dir ===
import { counter } from "./values.tspp";

const copy = counter;
/// @type.symbol symbol=copy source=copy type=int64
/// @resolution.pattern source=copy kind=binding target=copy
/// @type.node source=counter type=int64
/// @resolution.name source=counter target=values.counter
/// @resolution.access source=counter root=values.counter
"#,
    );
}
