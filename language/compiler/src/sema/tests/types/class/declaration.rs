use crate::tests::{DirRows, TestSession};

/// Provide checked DIR for a class member left incomplete mid-edit.
#[test]
fn test_check_provides_for_an_incomplete_class_member() {
    let session = TestSession::single(
        r#"
class Foo {
    like
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Foo {
    like;
}

=== checked ===
class Foo {
/// @type.symbol symbol=Foo type=Foo
/// @definition.class symbol=Foo
/// @definition.field symbol=Foo.like source=like key=like type=<error>

    like
    /// @type.symbol symbol=Foo.like source=like type=<error>

}
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=3 column=5 span="like" line_source="like"
"#,
    );
}

/// Check a class declared inside a function body.
#[test]
fn test_check_a_function_local_class() {
    let session = TestSession::single(
        r#"
function build(): int32 {
    class Registry {
        value: int32 = 1;
    }

    const registry = new Registry();
    registry.value
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function build(): int32 {
    class Registry {
        value: int32 = 1;
    }

    const registry: Registry = new Registry();
    registry.value
}

=== checked ===
function build(): int32 {
/// @type.symbol symbol=build type=() => int32

    class Registry {
    /// @type.symbol symbol=build.Registry type=build.Registry

        value: int32 = 1;
        /// @type.symbol symbol=build.Registry.value source="value: int32 = 1" type=int32

    }

    const registry = new Registry();
    /// @type.symbol symbol=build.registry source=registry type=build.Registry
    /// @resolution.pattern source=registry kind=binding target=build.registry
    /// @resolution.construct source="new Registry()" parameters=() return=build.Registry kind=class target=build.Registry constructor=default
    /// @resolution.name source=Registry target=build.Registry

    registry.value
    /// @resolution.name source=registry target=build.registry
    /// @resolution.member source=registry.value receiver=build.Registry type=int32 kind=field target_receiver=build.Registry key=value target=build.Registry.value target_type=int32
    /// @resolution.place source=registry placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=registry root=build.registry
    /// @resolution.place source=registry.value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=registry.value root=build.registry keys=[value]

}
"#,
        r#"
"#,
    );
}
