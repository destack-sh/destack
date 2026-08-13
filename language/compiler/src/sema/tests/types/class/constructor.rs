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
/// @resolution.pattern source=counter kind=binding target=counter
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

    constructor(value: int32): this {
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
/// @definition.method symbol=Base.constructor slot=constructor role=constructor type=(int32) => this

    value: int32;
    /// @type.symbol symbol=Base.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Base.constructor type=(int32) => this
    /// @type.symbol symbol=Base.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Base
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Base type=Base
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Base, target=field(receiver=Base, target=Base.value, type=int32), type=int32" type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Base.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Base.constructor.value

    }
}

class Derived extends Base {}
/// @type.symbol symbol=Derived source="class Derived extends Base {}" type=Derived
/// @definition.class symbol=Derived source="class Derived extends Base {}"
/// @definition.extends symbol=Derived source=Base target=Base
/// @resolution.name source=Base target=Base

const derived = new Derived(1);
/// @type.symbol symbol=derived source=derived type=Derived
/// @resolution.pattern source=derived kind=binding target=derived
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

    constructor(value: int32): this {
        this.value = value;
    }
}

const counter: Counter = new Counter(1);

=== checked ===
class Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(int32) => this

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(int32) => this
    /// @type.symbol symbol=Counter.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Counter
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=Counter
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Counter, target=field(receiver=Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Counter.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Counter.constructor.value

    }
}

const counter = new Counter(1);
/// @type.symbol symbol=counter source=counter type=Counter
/// @resolution.pattern source=counter kind=binding target=counter
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

    constructor(value: string): this {
        this.value = value as string | int32;
    }

    constructor(value: int32): this {
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
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(string) => this
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(int32) => this

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(string) => this
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box, target=field(receiver=Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => this
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box, target=field(receiver=Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Box.constructor.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#2

    }
}

const text = new Box("x");
/// @type.symbol symbol=text source=text type=Box
/// @resolution.pattern source=text kind=binding target=text
/// @type.node source="new Box(\"x\")" type=Box
/// @resolution.construct source="new Box(\"x\")" parameters=(string) arguments=(provided("x") as string) return=Box kind=class target=Box constructor=Box.constructor#1
/// @resolution.name source=Box target=Box
/// @type.node source="\"x\"" type="x"

const number = new Box(1);
/// @type.symbol symbol=number source=number type=Box
/// @resolution.pattern source=number kind=binding target=number
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

    constructor(value: string): this {
        this.value = value as string | int32;
    }

    constructor(value: int32): this {
        this.value = value as string | int32;
    }
}

new Box(true);

=== checked ===
class Box {
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(string) => this
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(int32) => this

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(string) => this
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box, target=field(receiver=Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => this
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box, target=field(receiver=Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Box.constructor.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#2

    }
}

new Box(true);
/// @type.node source="new Box(true)" type=<error>
/// @resolution.rejected source="new Box(true)"
/// @resolution.name source=Box target=Box
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=no-matching-construct message="no constructor matches arguments ('true')"
/// @diagnostic.label line=14 column=1 span="new Box(true)" line_source="new Box(true);"
/// @diagnostic.note message="the candidate '(value: string) => this' rejects argument 0: 'true' is not assignable to 'string'"
/// @diagnostic.note message="the candidate '(value: int32) => this' rejects argument 0: 'true' is not assignable to 'int32'"
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
/// @definition.method symbol=Box.constructor#1 source="constructor(value: string)" slot=constructor role=constructor type=(string) => this
/// @definition.method symbol=Box.constructor#2 source="constructor(value: int32)" slot=constructor role=constructor type=(int32) => this

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 source="constructor(value: string)" type=(string) => this
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 source="constructor(value: int32)" type=(int32) => this
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
/// @definition.method symbol=Box.constructor source="constructor(value: string)" slot=constructor role=constructor type=(string) => this

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor source="constructor(value: string)" type=(string) => this
    /// @type.symbol symbol=Box.constructor.value source="value: string" type=string

}
"#,
        r#"
/// @diagnostic.error id=missing-declaration-body message="declaration 'constructor' requires a body"
/// @diagnostic.label line=3 column=5 span="constructor" line_source="constructor(value: string);"
"#,
    );
}

#[test]
fn test_call_the_base_constructor_through_super() {
    let session = TestSession::single(
        r#"
class Animal {
    name: string;

    constructor(name: string) {
        this.name = name;
    }
}

class Dog extends Animal {
    tricks: int32;

    constructor(name: string, tricks: int32) {
        super(name);
        this.tricks = tricks;
    }
}

const dog = new Dog("rex", 3);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
class Animal {
    name: string;

    constructor(name: string): this {
        this.name = name;
    }
}

class Dog extends Animal {
    tricks: int32;

    constructor(name: string, tricks: int32): this {
        super(name);
        this.tricks = tricks;
    }
}

const dog: Dog = new Dog("rex", 3);

=== checked ===
class Animal {
/// @type.symbol symbol=Animal type=Animal
/// @definition.class symbol=Animal
/// @definition.field symbol=Animal.name source="name: string" key=name type=string
/// @definition.method symbol=Animal.constructor slot=constructor role=constructor type=(string) => this

    name: string;
    /// @type.symbol symbol=Animal.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Animal.constructor type=(string) => this
    /// @type.symbol symbol=Animal.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Animal type=Animal
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=Animal, target=field(receiver=Animal, target=Animal.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Animal.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Animal.constructor.name

    }
}

class Dog extends Animal {
/// @type.symbol symbol=Dog type=Dog
/// @definition.class symbol=Dog
/// @definition.extends symbol=Dog source=Animal target=Animal
/// @definition.field symbol=Dog.tricks source="tricks: int32" key=tricks type=int32
/// @definition.method symbol=Dog.constructor slot=constructor role=constructor type=(string, int32) => this
/// @resolution.name source=Animal target=Animal

    tricks: int32;
    /// @type.symbol symbol=Dog.tricks source="tricks: int32" type=int32

    constructor(name: string, tricks: int32) {
    /// @type.symbol symbol=Dog.constructor type=(string, int32) => this
    /// @type.symbol symbol=Dog.constructor.name source="name: string" type=string
    /// @type.symbol symbol=Dog.constructor.tricks source="tricks: int32" type=int32

        super(name);
        /// @resolution.receiver source=super kind=super declaration=Dog type=Animal
        /// @resolution.place source=super placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=super root=this
        /// @resolution.construct source=super(name) parameters=(string) arguments=(provided(name) as string) return=void kind=class target=Animal constructor=Animal.constructor
        /// @resolution.name source=name target=Dog.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Dog.constructor.name

        this.tricks = tricks;
        /// @resolution.receiver source=this kind=this declaration=Dog type=Dog
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.tricks kind=place
        /// @resolution.access source=this.tricks root=this keys=[tricks]
        /// @resolution.assignment source=this.tricks write="receiver=Dog, target=field(receiver=Dog, target=Dog.tricks, type=int32), type=int32" type=int32
        /// @resolution.name source=tricks target=Dog.constructor.tricks
        /// @resolution.place source=tricks placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=tricks root=Dog.constructor.tricks

    }
}

const dog = new Dog("rex", 3);
/// @type.symbol symbol=dog source=dog type=Dog
/// @resolution.pattern source=dog kind=binding target=dog
/// @resolution.construct source="new Dog(\"rex\", 3)" parameters=(string, int32) arguments=(provided("rex") as string, provided(3) as int32) return=Dog kind=class target=Dog constructor=Dog.constructor
/// @resolution.name source=Dog target=Dog
"#,
    );
}

#[test]
fn test_forward_a_base_constructor_across_modules() {
    let session = TestSession::builder()
        .module(
            "base.ds",
            r#"
export class Animal<T> {
    tag: T;

    constructor(tag: T) {
        this.tag = tag;
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { Animal } from "./base.ds";

class Dog extends Animal<string> {}

const dog = new Dog("rex");
"#,
        )
        .build();

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Animal } from "./base.ds";

class Dog extends Animal<string> {}

const dog: Dog = new Dog("rex");

=== checked ===
import { Animal } from "./base.ds";

class Dog extends Animal<string> {}
/// @type.symbol symbol=Dog source="class Dog extends Animal<string> {}" type=Dog
/// @definition.class symbol=Dog source="class Dog extends Animal<string> {}"
/// @definition.extends symbol=Dog source=Animal<string> target=base.Animal<string>
/// @resolution.name source=Animal target=base.Animal

const dog = new Dog("rex");
/// @type.symbol symbol=dog source=dog type=Dog
/// @resolution.pattern source=dog kind=binding target=dog
/// @resolution.construct source="new Dog(\"rex\")" parameters=(string) arguments=(provided("rex") as string) return=Dog kind=class target=Dog constructor=forwarded:base.Animal.symbol5
/// @resolution.name source=Dog target=Dog
"#,
    );
}

#[test]
fn test_satisfy_a_construct_signature_from_the_class_declaration() {
    let session = TestSession::single(
        r#"
class Counter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

declare function build(value: int32): Counter;

const make: new (value: int32) => Counter = Counter;
const broken: new (value: int32) => Counter = build;
"#,
    );

    session.assert_dir_checked_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-assignable message="type '(value: int32) => Counter' is not assignable to type 'new (value: int32) => Counter'"
/// @diagnostic.label line=13 column=47 span="build" line_source="const broken: new (value: int32) => Counter = build;"
/// @diagnostic.related line=13 column=15 span="new (value: int32) => Counter" line_source="const broken: new (value: int32) => Counter = build;" message="expected due to this annotation"
"#,
    );
}
