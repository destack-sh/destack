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
=== annotated ===
const config: { version: float64 } = { version: 1 };
const version: float64 = config.version;

=== checked ===
const config = { version: 1 };
/// @type.symbol symbol=config source=config type=Managed<{ version: float64 }>
/// @type.node source="{ version: 1 }" type=Managed<{ version: 1 }>
/// @type.node source=1 type=1

const version = config.version;
/// @type.symbol symbol=version source=version type=float64
/// @type.node source=config type=Managed<{ version: float64 }>
/// @type.node source=config.version type=float64
/// @resolution.name source=config target=config
/// @resolution.member source=config.version receiver=Managed<{ version: float64 }> kind=field key=version

/// @check.stats.solve variables=2 types=10 constraints=2 obligations=0 solutions=2 bounds=2 decisions=2
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
const config: { nested: { mode: "dev" } } = { nested: { mode: "dev" } } as const;
const mode: "dev" = config.nested.mode;

=== checked ===
const config = { nested: { mode: "dev" } } as const;
/// @type.symbol symbol=config source=config type=Managed<{ nested: Managed<{ mode: "dev" }> }>
/// @type.node source="{ nested: { mode: \"dev\" } } as const" type=Managed<{ nested: Managed<{ mode: "dev" }> }>
/// @type.node source="{ nested: { mode: \"dev\" } }" type=Managed<{ nested: Managed<{ mode: "dev" }> }>
/// @type.node source="{ mode: \"dev\" }" type=Managed<{ mode: "dev" }>
/// @type.node source="\"dev\"" type="dev"

const mode = config.nested.mode;
/// @type.symbol symbol=mode source=mode type="dev"
/// @type.node source=config type=Managed<{ nested: Managed<{ mode: "dev" }> }>
/// @type.node source=config.nested type=Managed<{ mode: "dev" }>
/// @type.node source=config.nested.mode type="dev"
/// @resolution.name source=config target=config
/// @resolution.member source=config.nested receiver=Managed<{ nested: Managed<{ mode: "dev" }> }> kind=field key=nested
/// @resolution.member source=config.nested.mode receiver=Managed<{ mode: "dev" }> kind=field key=mode

/// @check.stats.solve variables=3 types=10 constraints=1 obligations=0 solutions=3 bounds=3 decisions=3
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
const value: { env: { mode: "dev" } } = { env: { mode: "dev" } } as const satisfies {
    env: { mode: string };
};
const mode: "dev" = value.env.mode;

=== checked ===
const value = { env: { mode: "dev" } } as const satisfies { env: { mode: string } };
/// @type.symbol symbol=value source=value type=Managed<{ env: Managed<{ mode: "dev" }> }>
/// @type.node source="{ env: { mode: \"dev\" } } as const satisfies { env: { mode: string } }" type=Managed<{ env: Managed<{ mode: "dev" }> }>
/// @type.node source="{ env: { mode: \"dev\" } } as const" type=Managed<{ env: Managed<{ mode: "dev" }> }>
/// @type.node source="{ env: { mode: \"dev\" } }" type=Managed<{ env: Managed<{ mode: "dev" }> }>
/// @type.node source="{ mode: \"dev\" }" type=Managed<{ mode: "dev" }>
/// @type.node source="\"dev\"" type="dev"

const mode = value.env.mode;
/// @type.symbol symbol=mode#2 source=mode type="dev"
/// @type.node source=value type=Managed<{ env: Managed<{ mode: "dev" }> }>
/// @type.node source=value.env type=Managed<{ mode: "dev" }>
/// @type.node source=value.env.mode type="dev"
/// @resolution.name source=value target=value
/// @resolution.member source=value.env receiver=Managed<{ env: Managed<{ mode: "dev" }> }> kind=field key=env
/// @resolution.member source=value.env.mode receiver=Managed<{ mode: "dev" }> kind=field key=mode

/// @check.stats.solve variables=3 types=13 constraints=2 obligations=0 solutions=3 bounds=3 decisions=3
"#,
    );
}
