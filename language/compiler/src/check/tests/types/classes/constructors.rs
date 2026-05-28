use crate::tests::{DirRows, TestSession};

#[test]
fn test_constructor_call_selects_class_constructor() {
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
        DirRows::checked().with_reference_types(),
        r#"
class Counter {
/// @type.symbol symbol=Counter type=Counter

    value: int32;
    /// @type.symbol symbol=Counter.value type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(int32) => Counter
    /// @type.symbol symbol=value type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Counter
        /// @type.node source=this.value type=int32
        /// @resolution.member source=this.value receiver=Counter kind=symbol target=Counter.value
        /// @resolution.receiver source=this kind=this owner=Counter type=Counter
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=value

    }
}

const counter = new Counter(1);
/// @type.symbol symbol=counter type=Counter
/// @type.node source="new Counter(1)" type=Counter
/// @resolution.call source="new Counter(1)" parameters=(int32) return=Counter kind=construct target=Counter.constructor
/// @type.node source=Counter type=Counter
/// @resolution.name source=Counter target=Counter
/// @type.node source=1 type=int32
"#,
    );
}

#[test]
fn test_constructor_call_selects_matching_overload() {
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
        DirRows::checked().with_reference_types(),
        r#"
class Box {
/// @type.symbol symbol=Box type=Box

    value: string | int32;
    /// @type.symbol symbol=Box.value type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box
    /// @type.symbol symbol=value#1 type=string

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box
    /// @type.symbol symbol=value#2 type=int32

    constructor(value: string | int32) {
    /// @type.symbol symbol=Box.constructor#3 type=(string | int32) => Box
    /// @type.symbol symbol=value#3 type=string | int32

        this.value = value;
        /// @type.node source="this.value = value" type=string | int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.member source=this.value receiver=Box kind=symbol target=Box.value
        /// @resolution.receiver source=this kind=this owner=Box type=Box
        /// @type.node source=value type=string | int32
        /// @resolution.name source=value target=value#3

    }
}

const text = new Box("x");
/// @type.symbol symbol=text type=Box
/// @type.node source="new Box(\"x\")" type=Box
/// @resolution.call source="new Box(\"x\")" parameters=(string) return=Box kind=construct target=Box.constructor#1
/// @type.node source=Box type=Box
/// @resolution.name source=Box target=Box
/// @type.node source="\"x\"" type=string

const number = new Box(1);
/// @type.symbol symbol=number type=Box
/// @type.node source="new Box(1)" type=Box
/// @resolution.call source="new Box(1)" parameters=(int32) return=Box kind=construct target=Box.constructor#2
/// @type.node source=Box type=Box
/// @resolution.name source=Box target=Box
/// @type.node source=1 type=int32
"#,
    );
}

#[test]
fn test_constructor_call_without_matching_overload_reports_error() {
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
        DirRows::checked().with_reference_types(),
        r#"
class Box {
/// @type.symbol symbol=Box type=Box

    value: string | int32;
    /// @type.symbol symbol=Box.value type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box
    /// @type.symbol symbol=value#1 type=string

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box
    /// @type.symbol symbol=value#2 type=int32

    constructor(value: string | int32) {
    /// @type.symbol symbol=Box.constructor#3 type=(string | int32) => Box
    /// @type.symbol symbol=value#3 type=string | int32

        this.value = value;
        /// @type.node source="this.value = value" type=string | int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.member source=this.value receiver=Box kind=symbol target=Box.value
        /// @resolution.receiver source=this kind=this owner=Box type=Box
        /// @type.node source=value type=string | int32
        /// @resolution.name source=value target=value#3

    }
}

new Box(true);
/// @type.node source=Box type=Box
/// @resolution.name source=Box target=Box
/// @type.node source=true type=boolean

"#,
        r#"
/// @diagnostic.error code=EC302 message="no matching call overload"
/// @diagnostic.label line=12 column=1 source="new Box(true);"
"#,
    );
}
