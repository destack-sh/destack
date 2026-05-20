use crate::tests::{DirRows, TestSession};

#[test]
fn test_check_records_constructor_call_resolution() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

const counter = new Counter(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
class Counter {
/// @type.symbol symbol=Counter type=Counter

    value: int32;
    /// @type.symbol symbol=Counter.value type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(int32) => Counter

        this.value = value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Counter kind=direct target=Counter.value
        /// @resolution.name source=value target=value
    }
}

const counter = new Counter(1);
/// @resolution.name source=Counter target=Counter
/// @resolution.call source="new Counter(1)" parameters=[int32] return=Counter kind=construct target=Counter.constructor
/// @type.symbol symbol=counter type=Counter
"#,
    );
}

#[test]
fn test_check_records_constructor_overload_resolution() {
    let session = TestSession::single(
        r#"
class Box {
    value: string | int32;

    constructor(value: string);
    constructor(value: int32);
    constructor(value: string | int32) {
        this.value = value;
    }
}

const text = new Box("x");
const number = new Box(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
class Box {
/// @type.symbol symbol=Box type=Box

    value: string | int32;
    /// @type.symbol symbol=Box.value type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box

    constructor(value: string | int32) {
    /// @type.symbol symbol=Box.constructor#3 type=(string | int32) => Box

        this.value = value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box kind=direct target=Box.value
        /// @resolution.name source=value target=value
    }
}

const text = new Box("x");
/// @resolution.name source=Box target=Box
/// @resolution.call source="new Box(\"x\")" parameters=[string] return=Box kind=construct target=Box.constructor#1
/// @type.symbol symbol=text type=Box

const number = new Box(1);
/// @resolution.name source=Box target=Box
/// @resolution.call source="new Box(1)" parameters=[int32] return=Box kind=construct target=Box.constructor#2
/// @type.symbol symbol=number type=Box
"#,
    );
}

#[test]
fn test_check_reports_constructor_call_overload_mismatches() {
    let session = TestSession::single(
        r#"
class Box {
    value: string | int32;

    constructor(value: string);
    constructor(value: int32);
    constructor(value: string | int32) {
        this.value = value;
    }
}

new Box(true);
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
class Box {
/// @type.symbol symbol=Box type=Box

    value: string | int32;
    /// @type.symbol symbol=Box.value type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box

    constructor(value: string | int32) {
    /// @type.symbol symbol=Box.constructor#3 type=(string | int32) => Box

        this.value = value;
        /// @resolution.name source=this target=this
        /// @resolution.member source=this.value receiver=Box kind=direct target=Box.value
        /// @resolution.name source=value target=value
    }
}

new Box(true);
/// @resolution.name source=Box target=Box

"#,
        r#"
/// @diagnostic.error code=EC302 message="no matching call overload"
/// @diagnostic.label line=12 column=8 source="new Box(true);"
"#,
    );
}
