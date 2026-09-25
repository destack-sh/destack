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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Handler = { run: (value: float64) => number };

const handler: { run: (value: float64) => float64 } = {
    run: (value: float64): float64 => value + 1,
} satisfies Handler;

handler.run(1) satisfies number;

=== dir ===
type Handler = { run: (value: number) => number };
/// @type.symbol symbol=Handler source="type Handler = { run: (value: number) => number }" type={ run: (float64) => float64 }
/// @definition.type symbol=Handler source="type Handler = { run: (value: number) => number }" value={ run: (float64) => float64 }
/// @type.symbol symbol=Handler.run source="run: (value: number) => number" type=(float64) => float64
/// @type.symbol symbol=Handler.value source="value: number" type=float64

const handler = {
/// @type.symbol symbol=handler source=handler type={ run: Function<(float64,), float64, "readonly"> }
/// @resolution.pattern source=handler kind=binding target=handler

    run: (value) => value + 1,
    /// @type.symbol symbol=symbol5 source="(value) => value + 1" type=Function<(float64,), float64, "readonly">
    /// @type.symbol symbol=symbol5.value source=value type=float64
    /// @resolution.name source=value target=symbol5.value
    /// @resolution.operator source="value + 1" type=float64 operator="+" kind=builtin operands=[value as float64 families=(float), 1 as float64 families=(float)]
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol5.value

} satisfies Handler;
/// @resolution.name source=Handler target=Handler

handler.run(1) satisfies number;
/// @resolution.name source=handler target=handler
/// @resolution.member source=handler.run receiver={ run: Function<(float64,), float64, "readonly"> } type=Function<(float64,), float64, "readonly"> kind=field target_receiver={ run: Function<(float64,), float64, "readonly"> } key=run target_type=Function<(float64,), float64, "readonly">
/// @resolution.call source=handler.run(1) parameters=(float64) arguments=(provided(1) as float64) return=float64 kind=expression target=expression
/// @resolution.place source=handler placement="local" lifetime="static" access="immutable"
/// @resolution.access source=handler root=handler
/// @resolution.place source=handler.run placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=handler.run root=handler keys=[run]
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Mode = "dev" | "prod";

const config: { mode: "dev" } = { mode: "dev" } satisfies { mode: Mode };

config.mode satisfies "dev";

=== dir ===
type Mode = "dev" | "prod";
/// @type.symbol symbol=Mode source="type Mode = \"dev\" | \"prod\"" type="dev" | "prod"
/// @definition.type symbol=Mode source="type Mode = \"dev\" | \"prod\"" value="dev" | "prod"

const config = { mode: "dev" } satisfies { mode: Mode };
/// @type.symbol symbol=config source=config type={ mode: "dev" }
/// @resolution.pattern source=config kind=binding target=config
/// @type.symbol symbol=mode source="mode: Mode" type=Mode
/// @resolution.name source=Mode target=Mode

config.mode satisfies "dev";
/// @resolution.name source=config target=config
/// @resolution.member source=config.mode receiver={ mode: "dev" } type="dev" kind=field target_receiver={ mode: "dev" } key=mode target_type="dev"
/// @resolution.place source=config placement="local" lifetime="static" access="immutable"
/// @resolution.access source=config root=config
/// @resolution.place source=config.mode placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=config.mode root=config keys=[mode]
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
type Shape = { a: number };

const value: { a: float64; b: int64 } = { a: 1, b: 2 } satisfies Shape;

=== dir ===
type Shape = { a: number };
/// @type.symbol symbol=Shape source="type Shape = { a: number }" type={ a: float64 }
/// @definition.type symbol=Shape source="type Shape = { a: number }" value={ a: float64 }
/// @type.symbol symbol=Shape.a source="a: number" type=float64

const value = { a: 1, b: 2 } satisfies Shape;
/// @type.symbol symbol=value source=value type={ a: float64; b: int64 }
/// @resolution.pattern source=value kind=binding target=value
/// @resolution.name source=Shape target=Shape
"#,
        r#"
/// @diagnostic.error id=excess-property message="unknown property 'b' in object literal for type 'Shape'"
/// @diagnostic.label line=4 column=15 span="{ a: 1, b: 2 }" line_source="const value = { a: 1, b: 2 } satisfies Shape;"
/// @diagnostic.note message="object literals may only specify known properties"
"#,
    );
}

#[test]
fn test_satisfies_accepts_scalar_domains_and_representations() {
    let session = TestSession::single(
        r#"
import { Float, FloatDomain, Integer, IntegerDomain, NumericDomain } from "tspp:math";

1 satisfies IntegerDomain;
1.5 satisfies FloatDomain;
1 satisfies NumericDomain;
1.5 satisfies NumericDomain;
1 as int32 satisfies IntegerDomain;
1.5 as float32 satisfies FloatDomain;
1 as int32 satisfies Integer;
1.5 as float32 satisfies Float;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Float, FloatDomain, Integer, IntegerDomain, NumericDomain } from "tspp:math";

1 satisfies IntegerDomain;
1.5 satisfies FloatDomain;
1 satisfies NumericDomain;
1.5 satisfies NumericDomain;
1 as int32 satisfies IntegerDomain;
1.5 as float32 satisfies FloatDomain;
1 as int32 satisfies Integer;
1.5 as float32 satisfies Float;

=== dir ===
import { Float, FloatDomain, Integer, IntegerDomain, NumericDomain } from "tspp:math";

1 satisfies IntegerDomain;
/// @resolution.name source=IntegerDomain target=IntegerDomain

1.5 satisfies FloatDomain;
/// @resolution.name source=FloatDomain target=FloatDomain

1 satisfies NumericDomain;
/// @resolution.name source=NumericDomain target=NumericDomain

1.5 satisfies NumericDomain;
/// @resolution.name source=NumericDomain target=NumericDomain

1 as int32 satisfies IntegerDomain;
/// @resolution.name source=IntegerDomain target=IntegerDomain

1.5 as float32 satisfies FloatDomain;
/// @resolution.name source=FloatDomain target=FloatDomain

1 as int32 satisfies Integer;
/// @resolution.name source=Integer target=Integer

1.5 as float32 satisfies Float;
/// @resolution.name source=Float target=Float
"#,
    );
}

#[test]
fn test_satisfies_rejects_scalar_literals_as_representations() {
    let session = TestSession::single(
        r#"
import { Float, Integer } from "tspp:math";

1 satisfies Integer;
1.5 satisfies Float;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Float, Integer } from "tspp:math";

1 satisfies Integer;
1.5 satisfies Float;

=== dir ===
import { Float, Integer } from "tspp:math";

1 satisfies Integer;
/// @resolution.name source=Integer target=Integer

1.5 satisfies Float;
/// @resolution.name source=Float target=Float
"#,
        r#"
/// @diagnostic.error id=constraint-not-satisfied message="type '1' does not satisfy 'Integer'"
/// @diagnostic.label line=4 column=1 span="1" line_source="1 satisfies Integer;"
/// @diagnostic.error id=constraint-not-satisfied message="type '1.5' does not satisfy 'Float'"
/// @diagnostic.label line=5 column=1 span="1.5" line_source="1.5 satisfies Float;"
"#,
    );
}
