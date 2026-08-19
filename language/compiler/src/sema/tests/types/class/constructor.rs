use crate::tests::{DirRows, TestSession};

#[test]
fn test_reject_constructor_result_annotation() {
    let session = TestSession::single(
        r#"
class User {
    constructor(): this {}
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor(): this {}
}

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(): this {}" slot=constructor role=constructor type=() => this

    constructor(): this {}
    /// @type.symbol symbol=User.constructor source="constructor(): this {}" type=() => this

}
"#,
        r#"
/// @diagnostic.error id=constructor-result-annotation message="constructor cannot declare a result type"
/// @diagnostic.label line=3 column=20 span="this" line_source="constructor(): this {}"
"#,
    );
}

#[test]
fn test_reject_constructor_return_value() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {
        return this;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor(): this {
        return this;
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=() => this

    constructor() {
    /// @type.symbol symbol=User.constructor type=() => this

        return this;
        /// @type.node source=this type=User
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this

    }
}
"#,
        r#"
/// @diagnostic.error id=constructor-return-value message="constructor cannot return a value"
/// @diagnostic.label line=4 column=9 span="return this" line_source="return this;"
"#,
    );
}

#[test]
fn test_accept_constructor_bare_return() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {
        return;
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor(): this {
        return;
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=() => User

    constructor() {
    /// @type.symbol symbol=User.constructor type=() => User

        return;
    }
}
"#,
    );
}

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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;
}

const counter: Counter = new Counter();

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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

    session.assert_dir(
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

=== dir ===
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
    /// @type.symbol symbol=Box.constructor#2 type=(int32) => Box
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

    session.assert_dir_and_diagnostics(
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

=== dir ===
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare class Box {
    value: string | int32;

    constructor(value: string);
    constructor(value: int32);
}

=== dir ===
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

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box {
    constructor(value: string);
}

=== dir ===
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

    session.assert_dir(
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

=== dir ===
class Animal {
/// @type.symbol symbol=Animal type=Animal
/// @definition.class symbol=Animal
/// @definition.field symbol=Animal.name source="name: string" key=name type=string
/// @definition.method symbol=Animal.constructor slot=constructor role=constructor type=(string) => Animal

    name: string;
    /// @type.symbol symbol=Animal.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Animal.constructor type=(string) => Animal
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
/// @definition.method symbol=Dog.constructor slot=constructor role=constructor type=(string, int32) => Dog
/// @resolution.name source=Animal target=Animal

    tricks: int32;
    /// @type.symbol symbol=Dog.tricks source="tricks: int32" type=int32

    constructor(name: string, tricks: int32) {
    /// @type.symbol symbol=Dog.constructor type=(string, int32) => Dog
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

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { Animal } from "./base.ds";

class Dog extends Animal<string> {}

const dog: Dog = new Dog("rex");

=== dir ===
import { Animal } from "./base.ds";

class Dog extends Animal<string> {}
/// @type.symbol symbol=Dog source="class Dog extends Animal<string> {}" type=Dog
/// @generic.instance id=base.Animal<string> template=base.Animal arguments=(string)
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

    session.assert_dir_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-assignable message="type '(value: int32) => Counter' is not assignable to type 'new (value: int32) => Counter'"
/// @diagnostic.label line=13 column=47 span="build" line_source="const broken: new (value: int32) => Counter = build;"
/// @diagnostic.related line=13 column=15 span="new (value: int32) => Counter" line_source="const broken: new (value: int32) => Counter = build;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_new_without_arguments_admits_induced_only_template() {
    let session = TestSession::single(
        r#"
class State {
    value: unknown | undefined;

    constructor() {
        this.value = undefined;
    }
}

class Holder {
    state: State;

    constructor() {
        this.state = new State();
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class State {
    value: unknown | undefined;

    constructor(): this {
        this.value = undefined as unknown | undefined;
    }
}

class Holder {
    state: State;

    constructor(): this {
        this.state = new State();
    }
}

=== dir ===
class State {
/// @type.symbol symbol=State type=State
/// @definition.class symbol=State
/// @definition.field symbol=State.value source="value: unknown | undefined" key=value type=unknown | undefined
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=() => State

    value: unknown | undefined;
    /// @type.symbol symbol=State.value source="value: unknown | undefined" type=unknown | undefined

    constructor() {
    /// @type.symbol symbol=State.constructor type=() => State

        this.value = undefined;
        /// @type.node source="this.value = undefined" type=undefined
        /// @type.node source=this type=State
        /// @type.node source=this.value type=unknown | undefined
        /// @resolution.receiver source=this kind=this declaration=State type=State
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=State, target=field(receiver=State, target=State.value, type=unknown | undefined), type=unknown | undefined" type=unknown | undefined
        /// @type.node source=undefined type=undefined

    }
}

class Holder {
/// @type.symbol symbol=Holder type=Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.state source="state: State" key=state type=State
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=() => Holder

    state: State;
    /// @type.symbol symbol=Holder.state source="state: State" type=State
    /// @resolution.name source=State target=State

    constructor() {
    /// @type.symbol symbol=Holder.constructor type=() => Holder

        this.state = new State();
        /// @type.node source="this.state = new State()" type=State
        /// @type.node source=this type=Holder
        /// @type.node source=this.state type=State
        /// @resolution.receiver source=this kind=this declaration=Holder type=Holder
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.state kind=place
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.assignment source=this.state write="receiver=Holder, target=field(receiver=Holder, target=Holder.state, type=State), type=State" type=State
        /// @type.node source="new State()" type=State
        /// @resolution.construct source="new State()" parameters=() return=State kind=class target=State constructor=State.constructor
        /// @resolution.name source=State target=State

    }
}
"#,
    );
}

#[test]
fn test_new_infers_class_arguments_from_constructor_arguments() {
    let session = TestSession::single(
        r#"
class Box<T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

const value: int32 = 1;
const box = new Box(value);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box<in out T> {
    value: T;

    constructor(value: T): this {
        this.value = value;
    }
}

const value: int32 = 1;
const box: Box<int32> = new Box<int32>(value);

=== dir ===
class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(T) => this
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(T) => this
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @type.node source="this.value = value" type=T
        /// @type.node source=this type=Box<T>
        /// @type.node source=this.value type=T
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box<T>, target=field(receiver=Box<T>, target=Box.value, type=T), type=T" type=T
        /// @type.node source=value type=T
        /// @resolution.name source=value target=Box.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value

    }
}

const value: int32 = 1;
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=1 type=1

const box = new Box(value);
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.pattern source=box kind=binding target=box
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @type.node source="new Box(value)" type=Box<int32>
/// @resolution.construct source="new Box(value)" parameters=(int32) arguments=(provided(value) as int32) return=Box<int32> kind=class target=Box constructor=Box.constructor instance=Box<int32>
/// @generic.instantiation id=Box.constructor<int32> template=Box.constructor arguments=(int32)
/// @generic.instantiation id=Box<int32> template=Box arguments=(int32)
/// @generic.instance id=Box.constructor<int32> template=Box.constructor arguments=(int32)
/// @resolution.name source=Box target=Box
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="readonly"
/// @resolution.access source=value root=value
"#,
    );
}

#[test]
fn test_new_without_arguments_fills_defaulted_class_arguments() {
    let session = TestSession::single(
        r#"
class Box<T = string> {
    value: T | undefined;

    constructor() {
        this.value = undefined;
    }
}

const box = new Box();
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box<in out T = string> {
    value: T | undefined;

    constructor(): this {
        this.value = undefined as T | undefined;
    }
}

const box: Box<string> = new Box<string>();

=== dir ===
class Box<T = string> {
/// @generic.template symbol=Box parameters=(in out T = string)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(in out T = string)
/// @definition.field symbol=Box.value source="value: T | undefined" key=value type=T | undefined
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=() => this
/// @type.symbol symbol=Box.T source="T = string" type=T

    value: T | undefined;
    /// @type.symbol symbol=Box.value source="value: T | undefined" type=T | undefined
    /// @resolution.name source=T target=Box.T

    constructor() {
    /// @type.symbol symbol=Box.constructor type=() => this

        this.value = undefined;
        /// @type.node source="this.value = undefined" type=undefined
        /// @type.node source=this type=Box<T>
        /// @type.node source=this.value type=T | undefined
        /// @resolution.receiver source=this kind=this declaration=Box type=Box<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=Box<T>, target=field(receiver=Box<T>, target=Box.value, type=T | undefined), type=T | undefined" type=T | undefined
        /// @type.node source=undefined type=undefined

    }
}

const box = new Box();
/// @type.symbol symbol=box source=box type=Box<string>
/// @resolution.pattern source=box kind=binding target=box
/// @generic.instance id=Box<string> template=Box arguments=(string)
/// @type.node source="new Box()" type=Box<string>
/// @resolution.construct source="new Box()" parameters=() return=Box<string> kind=class target=Box constructor=Box.constructor instance=Box<string>
/// @generic.instantiation id=Box.constructor<string> template=Box.constructor arguments=(string)
/// @generic.instantiation id=Box<string> template=Box arguments=(string)
/// @generic.instance id=Box.constructor<string> template=Box.constructor arguments=(string)
/// @resolution.name source=Box target=Box
"#,
    );
}
