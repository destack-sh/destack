use crate::tests::{DirRows, TestSession};

#[test]
fn test_satisfies_contextualizes_closure_parameters() {
    let session = TestSession::single(
        r#"
type Handler = { run: (value: number) => number };

const handler = {
    run: (value) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Handler = { run: (value: number) => number };

const handler: { run: (number) => number } = {
    run: (value: number) => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;

=== checked ===
type Handler = { run: (value: number) => number };
/// @type.symbol symbol=Handler source="type Handler = { run: (value: number) => number }" type={ run: (number) => number }
/// @definition.type symbol=Handler source="type Handler = { run: (value: number) => number }" value={ run: (number) => number }

const handler = {
    run: (value) => value + 1,
} satisfies Handler;
/// @type.symbol symbol=handler source=handler type={ run: (number) => number }
/// @resolution.name source=Handler target=Handler

handler.run(1) satisfies number;
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver={ run: (number) => number } kind=field key=run
/// @resolution.call source=handler.run(1) parameters=(number) return=number kind=symbol target=handler.run
"#,
    );
}

#[test]
fn test_satisfies_preserves_literal_members() {
    let session = TestSession::single(
        r#"
type Mode = "dev" | "prod";

const config = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Mode = "dev" | "prod";

const config: { mode: "dev" } = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";

=== checked ===
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode source="type Mode = \"dev\" | \"prod\"" type="dev" | "prod"
/// @definition.type symbol=Mode source="type Mode = \"dev\" | \"prod\"" value="dev" | "prod"

const config = { mode: "dev" } satisfies { mode: Mode };
/// @type.symbol symbol=config source=config type={ mode: "dev" }
/// @resolution.name source=Mode target=Mode

config.mode satisfies "dev";
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } kind=field key=mode
"#,
    );
}

#[test]
fn test_satisfies_enforces_excess_property_checks() {
    let session = TestSession::single(
        r#"
type Shape = { a: number };

const value = { a: 1, b: 2 } satisfies Shape;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Shape = { a: number };

const value = { a: 1, b: 2 } satisfies Shape;

=== checked ===
type Shape = { a: number };
/// @type.symbol symbol=Shape source="type Shape = { a: number }" type={ a: number }
/// @definition.type symbol=Shape source="type Shape = { a: number }" value={ a: number }

const value = { a: 1, b: 2 } satisfies Shape;
/// @type.symbol symbol=value source=value type={ a: 1; b: 2 }
/// @resolution.name source=Shape target=Shape
"#,
        r#"
/// @diagnostic.error code=EC205 message="unknown property 'b' in object literal for type '{ a: number }'"
/// @diagnostic.label line=4 column=15 source="const value = { a: 1, b: 2 } satisfies Shape;"
"#,
    );
}
