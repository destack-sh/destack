use crate::tests::{DirRows, TestSession};

#[test]
fn test_fresh_object_literal_argument_selects_union_arm() {
    // a fresh object literal has no aliases, so it conforms covariantly
    // with strict excess keys and picks the shape arm of a union target
    let session = TestSession::single(
        r#"
type Base = {
    only?: boolean;
    skip?: boolean;
};

type Options = Base & {
    samples?: uint64;
};

type Argument = (() => void) | Options;

function skipAll(): void {
    run({ skip: true })
}

function run(argument?: Argument): void {}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
type Base = {
    only?: boolean;
    skip?: boolean;
};

type Options = Base & {
    samples?: uint64;
};

type Argument = (() => void) | Options;

function skipAll(): void {
    run({ skip: true } as Argument | undefined);
}

function run(argument?: Argument): void {}

=== checked ===
type Base = {
/// @type.symbol symbol=Base type={ only?: boolean; skip?: boolean }
/// @definition.type symbol=Base value={ only?: boolean; skip?: boolean }

    only?: boolean;
    skip?: boolean;
};

type Options = Base & {
/// @type.symbol symbol=Options type=Base & { samples?: uint64 } reduced={ only?: boolean; skip?: boolean; samples?: uint64 }
/// @definition.type symbol=Options value=Base & { samples?: uint64 } reduced={ only?: boolean; skip?: boolean; samples?: uint64 }
/// @resolution.name source=Base target=Base

    samples?: uint64;
};

type Argument = (() => void) | Options;
/// @type.symbol symbol=Argument source="type Argument = (() => void) | Options" type=Function<(), void> | Options
/// @definition.type symbol=Argument source="type Argument = (() => void) | Options" value=Function<(), void> | Options
/// @resolution.name source=Options target=Options

function skipAll(): void {
/// @type.symbol symbol=skipAll type=() => void

    run({ skip: true })
    /// @type.node source="run({ skip: true })" type=void
    /// @resolution.name source=run target=run
    /// @resolution.call source="run({ skip: true })" parameters=(Argument | undefined) arguments=(provided({ skip: true }) as Argument | undefined) return=void kind=symbol target=run
    /// @type.node source={ skip: true } type={ only?: boolean; skip?: boolean; samples?: uint64 }
    /// @type.node source=true type=true

}

function run(argument?: Argument): void {}
/// @type.symbol symbol=run source="function run(argument?: Argument): void {}" type=(Argument | undefined?) => void
/// @type.symbol symbol=run.argument source="argument?: Argument" type=Argument | undefined
/// @resolution.name source=Argument target=Argument
"#,
    );
}
