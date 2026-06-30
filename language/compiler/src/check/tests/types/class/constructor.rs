use crate::tests::{DirRows, TestSession};

#[test]
fn test_class_without_constructor_gets_implicit_empty_constructor() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32 = 0;
}

const counter = new Counter();
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;
}

const counter: Counter = new Counter();

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32 = 0" key=value type=int32

    value: int32 = 0;
    /// @type.symbol symbol=Counter.value source="value: int32 = 0" type=int32
    /// @type.node source=0 type=0

}

const counter = new Counter();
/// @type.symbol symbol=counter source=counter type=Counter
/// @type.node source="new Counter()" type=Counter
/// @resolution.construct source="new Counter()" parameters=() return=Counter kind=class target=Counter constructor=default
/// @resolution.name source=Counter target=Counter
"#,
    );
}

#[test]
fn test_derived_class_without_constructor_forwards_base_constructor() {
    let session = TestSession::single(
        r#"
class Base {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

class Derived extends Base {}

const derived = new Derived(1);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Base {
    value: int32;

    constructor(value: int32): Base {
        this.value = value;
    }
}

class Derived extends Base {}

const derived: Derived = new Derived(1);

=== checked ===
class Base {
/// @type.symbol symbol=Base type=Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.value source="value: int32" key=value type=int32
/// @definition.method symbol=Base.constructor slot=constructor role=constructor type=(int32) => Base

    value: int32;
    /// @type.symbol symbol=Base.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Base.constructor type=(int32) => Base
    /// @type.symbol symbol=Base.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Base
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Base type=Base
        /// @resolution.pattern.assign source=this.value kind=place place=field(Base.value) type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Base.constructor.value

    }
}

class Derived extends Base {}
/// @type.symbol symbol=Derived source="class Derived extends Base {}" type=Derived
/// @definition.class symbol=Derived source="class Derived extends Base {}"
/// @definition.extends symbol=Derived source=Base target=Base
/// @resolution.name source=Base target=Base

const derived = new Derived(1);
/// @type.symbol symbol=derived source=derived type=Derived
/// @type.node source="new Derived(1)" type=Derived
/// @resolution.construct source="new Derived(1)" parameters=(int32) arguments=(provided(1) as int32) return=Derived kind=class target=Derived constructor=forwarded:Base.constructor
/// @resolution.name source=Derived target=Derived
/// @type.node source=1 type=1
"#,
    );
}

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
=== annotated ===
class Counter {
    value: int32;

    constructor(value: int32): Counter {
        this.value = value;
    }
}

const counter: Counter = new Counter(1);

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(int32) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(int32) => Counter
    /// @type.symbol symbol=Counter.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Counter
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.pattern.assign source=this.value kind=place place=field(Counter.value) type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Counter.constructor.value

    }
}

const counter = new Counter(1);
/// @type.symbol symbol=counter source=counter type=Counter
/// @type.node source="new Counter(1)" type=Counter
/// @resolution.construct source="new Counter(1)" parameters=(int32) arguments=(provided(1) as int32) return=Counter kind=class target=Counter constructor=Counter.constructor
/// @resolution.name source=Counter target=Counter
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_constructor_call_selects_matching_overload() {
    let session = TestSession::single(
        r#"
class Box {
    value: string | int32;

    constructor(value: string) {
        this.value = value;
    }

    constructor(value: int32) {
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
=== annotated ===
class Box {
    value: string | int32;

    constructor(value: string): Box {
        this.value = value as string | int32;
    }

    constructor(value: int32): Box {
        this.value = value as string | int32;
    }
}

const text: Box = new Box("x");
const number: Box = new Box(1);

=== checked ===
class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(string) => Box
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=string | int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Box.constructor.value#2

    }
}

const text = new Box("x");
/// @type.symbol symbol=text source=text type=Box
/// @type.node source="new Box(\"x\")" type=Box
/// @resolution.construct source="new Box(\"x\")" parameters=(string) arguments=(provided("x") as string) return=Box kind=class target=Box constructor=Box.constructor#1
/// @resolution.name source=Box target=Box
/// @type.node source="\"x\"" type="x"

const number = new Box(1);
/// @type.symbol symbol=number source=number type=Box
/// @type.node source="new Box(1)" type=Box
/// @resolution.construct source="new Box(1)" parameters=(int32) arguments=(provided(1) as int32) return=Box kind=class target=Box constructor=Box.constructor#2
/// @resolution.name source=Box target=Box
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_constructor_call_without_matching_overload_reports_error() {
    let session = TestSession::single(
        r#"
class Box {
    value: string | int32;

    constructor(value: string) {
        this.value = value;
    }

    constructor(value: int32) {
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
=== annotated ===
class Box {
    value: string | int32;

    constructor(value: string): Box {
        this.value = value as string | int32;
    }

    constructor(value: int32): Box {
        this.value = value as string | int32;
    }
}

new Box(true);

=== checked ===
class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(string) => Box
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(string) => Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.pattern.assign source=this.value kind=place place=field(Box.value) type=string | int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Box.constructor.value#2

    }
}

new Box(true);
/// @resolution.name source=Box target=Box
/// @type.node source=true type=true

"#,
        r#"
/// @diagnostic.error code=EC302 message="no overload matches arguments ('true')"
/// @diagnostic.label line=14 column=1 span="new Box(true)" line_source="new Box(true);"
"#,
    );
}

#[test]
fn test_declare_constructor_declarations_define_overloads() {
    let session = TestSession::single(
        r#"
declare class Box {
    value: string | int32;

    constructor(value: string);
    constructor(value: int32);
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Box {
    value: string | int32;

    constructor(value: string);
    constructor(value: int32);
}

=== checked ===
declare class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 source="constructor(value: string)" slot=constructor role=constructor type=(string) => Box
/// @definition.method symbol=Box.constructor#2 source="constructor(value: int32)" slot=constructor role=constructor type=(int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 source="constructor(value: string)" type=(string) => Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 source="constructor(value: int32)" type=(int32) => Box
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

}
"#,
    );
}

#[test]
fn test_concrete_constructor_declaration_requires_body() {
    let session = TestSession::single(
        r#"
class Box {
    constructor(value: string);
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box {
    constructor(value: string);
}

=== checked ===
class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.method symbol=Box.constructor source="constructor(value: string)" slot=constructor role=constructor type=(string) => Box

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor source="constructor(value: string)" type=(string) => Box
    /// @type.symbol symbol=Box.constructor.value source="value: string" type=string

}
"#,
        r#"
/// @diagnostic.error code=EC611 message="declaration 'constructor' requires a body"
/// @diagnostic.label line=3 column=5 span="constructor(value: string)" line_source="constructor(value: string);"
"#,
    );
}
