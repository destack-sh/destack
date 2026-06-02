use crate::tests::{DirRows, TestSession};

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
const config = { version: 1 };
/// @type.symbol symbol=config type={ version: int32 }
/// @type.node source="{ version: 1 }" type={ version: 1 }
/// @type.node source=1 type=1

const version = config.version;
/// @type.symbol symbol=version type=int32
/// @type.node source=config type={ version: int32 }
/// @type.node source=config.version type=int32
/// @resolution.name source=config target=config
/// @resolution.member source=config.version receiver={ version: int32 } kind=field key=version
/// @check.stats.solve variables=2 terms=14 constraints=2 obligations=0 solutions=2 bounds=4 decisions=1
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
        DirRows::checked().with_reference_types(),
        r#"
const config = { nested: { mode: "dev" } } as const;
/// @type.symbol symbol=config type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source="{ nested: { mode: \"dev\" } } as const" type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source="{ nested: { mode: \"dev\" } }" type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source="{ mode: \"dev\" }" type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"

const mode = config.nested.mode;
/// @type.symbol symbol=mode type="dev"
/// @type.node source=config type={ readonly nested: { readonly mode: "dev" } }
/// @type.node source=config.nested type={ readonly mode: "dev" }
/// @type.node source=config.nested.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.nested receiver={ readonly nested: { readonly mode: "dev" } } kind=field key=nested
/// @resolution.member source=config.nested.mode receiver={ readonly mode: "dev" } kind=field key=mode
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
        DirRows::checked().with_reference_types(),
        r#"
const value = { env: { mode: "dev" } } as const satisfies { env: { mode: string } };
/// @type.symbol symbol=value type={ readonly env: { readonly mode: "dev" } }
/// @type.node source="{ env: { mode: \"dev\" } } as const satisfies { env: { mode: string } }" type={ readonly env: { readonly mode: "dev" } }
/// @type.node source="{ env: { mode: \"dev\" } } as const" type={ readonly env: { readonly mode: "dev" } }
/// @type.node source="{ env: { mode: \"dev\" } }" type={ readonly env: { readonly mode: "dev" } }
/// @type.node source="{ mode: \"dev\" }" type={ readonly mode: "dev" }
/// @type.node source="\"dev\"" type="dev"
/// @type.symbol symbol=env type={ mode: string }
/// @type.symbol symbol=mode#1 type=string

const mode = value.env.mode;
/// @type.symbol symbol=mode#2 type="dev"
/// @type.node source=value type={ readonly env: { readonly mode: "dev" } }
/// @type.node source=value.env type={ readonly mode: "dev" }
/// @type.node source=value.env.mode type="dev"
/// @resolution.name source=value target=value
/// @resolution.member source=value.env receiver={ readonly env: { readonly mode: "dev" } } kind=field key=env
/// @resolution.member source=value.env.mode receiver={ readonly mode: "dev" } kind=field key=mode
"#,
    );
}
