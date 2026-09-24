use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_nominal_declarations_inside_a_function_body() {
    let session = TestSession::single(
        r#"
function make(): void {
    struct Vector { x: float64; }
    class Point {}
    enum Status { Idle }
    interface Greet {
        greet(): int32;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function make(): void {
    struct Vector {
        x: float64;
    }
    class Point {}
    enum Status {
        Idle,
    }
    interface Greet {
        greet(): int32;
    }
}

=== dir ===
function make(): void {
/// @type.symbol symbol=make type=() => void

    struct Vector { x: float64; }
    /// @type.symbol symbol=make.Vector source="struct Vector { x: float64; }" type=make.Vector

    class Point {}
    /// @type.symbol symbol=make.Point source="class Point {}" type=make.Point

    enum Status { Idle }
    /// @type.symbol symbol=make.Status source="enum Status { Idle }" type=make.Status

    interface Greet {
    /// @type.symbol symbol=make.Greet type=make.Greet

        greet(): int32;
    }
}
"#,
        r#"
/// @diagnostic.error id=declaration-not-nestable message="'struct' declarations cannot nest in a function body; declare them at module scope"
/// @diagnostic.label line=3 column=5 span="struct Vector { x: float64; }" line_source="struct Vector { x: float64; }"
/// @diagnostic.error id=declaration-not-nestable message="'class' declarations cannot nest in a function body; declare them at module scope"
/// @diagnostic.label line=4 column=5 span="class Point {}" line_source="class Point {}"
/// @diagnostic.error id=declaration-not-nestable message="'enum' declarations cannot nest in a function body; declare them at module scope"
/// @diagnostic.label line=5 column=5 span="enum Status { Idle }" line_source="enum Status { Idle }"
/// @diagnostic.error id=declaration-not-nestable message="'interface' declarations cannot nest in a function body; declare them at module scope"
/// @diagnostic.label line=6 column=5 span="interface Greet {\n        greet(): int32;\n    }" line_source="interface Greet {"
"#,
    );
}

#[test]
fn test_reject_an_extension_inside_a_function_body() {
    let session = TestSession::single(
        r#"
struct Vector {
    x: float64;
}

function extend(): void {
    extension of Vector {
        length(this): float64 {
            this.x
        }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Vector {
    x: float64;
}

function extend(): void {
    extension of Vector {
        length(this): float64 {
            this.x
        }
    }
}

=== dir ===
struct Vector {
/// @type.symbol symbol=Vector type=Vector
/// @definition.struct symbol=Vector
/// @definition.field symbol=Vector.x source="x: float64" key=x type=float64

    x: float64;
    /// @type.symbol symbol=Vector.x source="x: float64" type=float64

}

function extend(): void {
/// @type.symbol symbol=extend type=() => void

    extension of Vector {
        length(this): float64 {
            this.x
        }
    }
}
"#,
        r#"
/// @diagnostic.error id=declaration-not-nestable message="'extension' declarations cannot nest in a function body; declare them at module scope"
/// @diagnostic.label line=7 column=5 span="extension of Vector {\n        length(this): float64 {\n            this.x\n        }\n    }" line_source="extension of Vector {"
"#,
    );
}

#[test]
fn test_accept_nested_function_and_type_declarations() {
    let session = TestSession::single(
        r#"
function measure(): float64 {
    type Distance = float64;
    newtype Meters = float64;
    function convert(value: Distance): Meters {
        Meters(value)
    }

    convert(2.0) satisfies Meters;
    1.0
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function measure(): float64 {
    type Distance = float64;
    newtype Meters = float64;
    function convert(value: Distance): Meters {
        Meters(value)
    }

    convert(2.0) satisfies Meters;
    1.0
}

=== dir ===
function measure(): float64 {
/// @type.symbol symbol=measure type=() => float64

    type Distance = float64;
    /// @type.symbol symbol=measure.Distance source="type Distance = float64" type=float64

    newtype Meters = float64;
    /// @type.symbol symbol=measure.Meters source="newtype Meters = float64" type=measure.Meters

    function convert(value: Distance): Meters {
    /// @type.symbol symbol=measure.convert type=(measure.Distance) => measure.Meters
    /// @type.symbol symbol=measure.convert.value source="value: Distance" type=measure.Distance
    /// @resolution.name source=Distance target=measure.Distance
    /// @resolution.name source=Meters target=measure.Meters

        Meters(value)
        /// @resolution.name source=Meters target=measure.Meters
        /// @resolution.construct source=Meters(value) parameters=(float64) arguments=(provided(value) as float64) return=measure.Meters kind=newtype target=measure.Meters backing=float64
        /// @resolution.name source=value target=measure.convert.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=measure.convert.value

    }

    convert(2.0) satisfies Meters;
    /// @resolution.name source=convert target=measure.convert
    /// @resolution.call source=convert(2.0) parameters=(measure.Distance) arguments=(provided(2.0) as measure.Distance) return=measure.Meters kind=symbol target=measure.convert
    /// @resolution.name source=Meters target=measure.Meters

    1.0
}
"#,
        r#"
"#,
    );
}
