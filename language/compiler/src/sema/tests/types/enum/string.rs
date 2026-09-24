use crate::tests::{DirRows, TestSession};

/// A string-backed enum rejects implicit raw assignment.
#[test]
fn test_string_enum_backing_rejects_implicit_raw_assignment() {
    let session = TestSession::single(
        r#"
enum Direction {
    Up = "UP",
    Down = "DOWN",
}

const raw: string = Direction.Up;
const direction: Direction = "UP";
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
enum Direction {
    Up = "UP",
    Down = "DOWN",
}

const raw: string = Direction.Up;
const direction: Direction = "UP";

=== dir ===
enum Direction {
/// @type.symbol symbol=Direction type=Direction
/// @definition.enum symbol=Direction backing=string
/// @definition.variant symbol=Direction.Down source="Down = \"DOWN\"" key=Down value="\"DOWN\""
/// @definition.variant symbol=Direction.Up source="Up = \"UP\"" key=Up value="\"UP\""

    Up = "UP",
    /// @type.symbol symbol=Direction.Up source="Up = \"UP\"" type=Direction.Up

    Down = "DOWN",
    /// @type.symbol symbol=Direction.Down source="Down = \"DOWN\"" type=Direction.Down

}

const raw: string = Direction.Up;
/// @type.symbol symbol=raw source=raw type=string
/// @resolution.pattern source=raw kind=binding target=raw
/// @resolution.name source=Direction target=Direction
/// @resolution.member source=Direction.Up receiver=Direction type=Direction.Up kind=symbol target_receiver=Direction target=Direction.Up

const direction: Direction = "UP";
/// @type.symbol symbol=direction source=direction type=Direction
/// @resolution.pattern source=direction kind=binding target=direction
/// @resolution.name source=Direction target=Direction
"#, r#"
/// @diagnostic.error id=not-assignable message="type 'Direction.Up' is not assignable to type 'string'"
/// @diagnostic.label line=7 column=21 span="Direction.Up" line_source="const raw: string = Direction.Up;"
/// @diagnostic.related line=7 column=12 span="string" line_source="const raw: string = Direction.Up;" message="expected due to this annotation"
/// @diagnostic.error id=not-assignable message="type '\"UP\"' is not assignable to type 'Direction'"
/// @diagnostic.label line=8 column=30 span="\"UP\"" line_source="const direction: Direction = \"UP\";"
/// @diagnostic.related line=8 column=18 span="Direction" line_source="const direction: Direction = \"UP\";" message="expected due to this annotation"
"#);
}

/// A string enum member rejects comparison with a raw string literal.
#[test]
fn test_string_enum_member_rejects_a_raw_literal_comparison() {
    let session = TestSession::single(
        r#"
enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const direction: Direction;
const matches = direction === "UP";
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
enum Direction {
    Up = "UP",
    Down = "DOWN",
}

declare const direction: Direction;
const matches: boolean = direction === "UP";

=== dir ===
enum Direction {
/// @type.symbol symbol=Direction type=Direction
/// @definition.enum symbol=Direction backing=string
/// @definition.variant symbol=Direction.Down source="Down = \"DOWN\"" key=Down value="\"DOWN\""
/// @definition.variant symbol=Direction.Up source="Up = \"UP\"" key=Up value="\"UP\""

    Up = "UP",
    /// @type.symbol symbol=Direction.Up source="Up = \"UP\"" type=Direction.Up

    Down = "DOWN",
    /// @type.symbol symbol=Direction.Down source="Down = \"DOWN\"" type=Direction.Down

}

declare const direction: Direction;
/// @type.symbol symbol=direction source=direction type=Direction
/// @resolution.pattern source=direction kind=binding target=direction
/// @resolution.name source=Direction target=Direction

const matches = direction === "UP";
/// @type.symbol symbol=matches source=matches type=boolean
/// @resolution.pattern source=matches kind=binding target=matches
/// @resolution.name source=direction target=direction
/// @resolution.operator source="direction === \"UP\"" type=boolean operator="===" kind=builtin operands=[direction as Direction families=(Direction), "UP" as "UP" families=(string)]
/// @resolution.place source=direction placement="local" lifetime="static" access="immutable"
/// @resolution.access source=direction root=direction
"#, r#"
/// @diagnostic.error id=invalid-strict-equality message="this comparison is unintentional: types 'Direction' and '\"UP\"' have no overlap"
/// @diagnostic.label line=8 column=27 span="===" line_source="const matches = direction === \"UP\";"
"#);
}
