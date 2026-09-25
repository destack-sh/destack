use crate::tests::{DirRows, TestSession};

/// Abstract classes expose a signature but cannot produce an allocating function.
#[test]
fn test_reference_abstract_class_constructor() {
    let session = TestSession::single(
        r#"
abstract class Writer {}

type Constructor = typeof Writer;

const create = Writer;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
abstract class Writer {}

type Constructor = typeof Writer;

const create = Writer;

=== dir ===
abstract class Writer {}
/// @type.symbol symbol=Writer source="abstract class Writer {}" type=typeof Writer
/// @definition.class symbol=Writer source="abstract class Writer {}" abstract=true

type Constructor = typeof Writer;
/// @type.symbol symbol=Constructor source="type Constructor = typeof Writer" type=Function<(), Writer, "readonly">
/// @definition.type symbol=Constructor source="type Constructor = typeof Writer" value=typeof Writer
/// @resolution.name source=Writer target=Writer

const create = Writer;
/// @type.symbol symbol=create source=create type=<error>
/// @resolution.pattern source=create kind=binding target=create
/// @resolution.name source=Writer target=Writer
"#,
        r#"
/// @diagnostic.error id=cannot-construct-abstract-type message="abstract class 'Writer' cannot be constructed"
/// @diagnostic.label line=6 column=16 span="Writer" line_source="const create = Writer;"
/// @diagnostic.help message="construct a concrete subclass instead"
"#,
    );
}

/// A stored constructor exposes its function signature without class static members.
#[test]
fn test_read_static_member_from_constructor_function() {
    let session = TestSession::single(
        r#"
class Counter {
    static version: int32 = 1;
}

const create = Counter;

const version = create.version;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Counter {
    static version: int32 = 1;
}

const create: new () => Counter = Counter;

const version = create.version;

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.version source="static version: int32 = 1" key=version static=true type=int32

    static version: int32 = 1;
    /// @type.symbol symbol=Counter.version source="static version: int32 = 1" type=int32

}

const create = Counter;
/// @type.symbol symbol=create source=create type=Function<(), Counter, "readonly">
/// @resolution.pattern source=create kind=binding target=create
/// @resolution.name source=Counter target=Counter
/// @resolution.function source=Counter type=Function<(), Counter, "readonly"> target=Counter

const version = create.version;
/// @type.symbol symbol=version source=version type=<error>
/// @resolution.pattern source=version kind=binding target=version
/// @resolution.name source=create target=create
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create
/// @resolution.rejected source=create.version
"#,
        r#"
/// @diagnostic.error id=missing-member message="member 'version' does not exist on type 'Function<(), Counter, \"readonly\">'"
/// @diagnostic.label line=8 column=24 span="version" line_source="const version = create.version;"
"#,
    );
}

/// A specialized class value satisfies its constructor signature.
#[test]
fn test_assign_specialized_constructor() {
    let session = TestSession::single(
        r#"
class Box<out T> {}

const create: new () => Box<int32> = Box<int32>;

const box = new create();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class Box<out T> {}

const create: new () => Box<int32> = Box<int32>;

const box: Box<int32> = new create();

=== dir ===
class Box<out T> {}
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box source="class Box<out T> {}" type=typeof Box
/// @definition.class symbol=Box source="class Box<out T> {}" template=(out T)
/// @type.symbol symbol=Box.T source="out T" type=T

const create: new () => Box<int32> = Box<int32>;
/// @type.symbol symbol=create source=create type=new () => Box<int32>
/// @resolution.pattern source=create kind=binding target=create
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box<int32> target=Box
/// @resolution.function source=Box<int32> type=new () => Box<int32> target=Box
/// @generic.instantiation id=Box<int32> template=Box arguments=(int32)

const box = new create();
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.call source="new create()" parameters=() return=Box<int32> kind=expression target=expression
/// @resolution.name source=create target=create
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create
"#,
    );
}

/// A constructor call reads the selected static field.
#[test]
fn test_construct_through_static_class_value() {
    let session = TestSession::single(
        r#"
class User {}

declare class Constructors {
    static user: typeof User;
}

const user = new Constructors.user();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare class Constructors {
    static user: new () => User;
}

const user: User = new Constructors.user();

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare class Constructors {
/// @type.symbol symbol=Constructors type=typeof Constructors
/// @definition.class symbol=Constructors
/// @definition.field symbol=Constructors.user source="static user: typeof User" key=user static=true type=Function<(), User, "readonly">

    static user: typeof User;
    /// @type.symbol symbol=Constructors.user source="static user: typeof User" type=Function<(), User, "readonly">
    /// @resolution.name source=User target=User

}

const user = new Constructors.user();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.call source="new Constructors.user()" parameters=() return=User kind=expression target=expression
/// @resolution.name source=Constructors target=Constructors
/// @resolution.member source=Constructors.user receiver=typeof Constructors type=Function<(), User, "readonly"> kind=field target_receiver=typeof Constructors key=user target=Constructors.user target_type=Function<(), User, "readonly">
/// @resolution.place source=Constructors.user placement="local" lifetime="managed" access="mutable"
/// @resolution.access source=Constructors.user root=Constructors keys=[user]
"#,
    );
}

/// A constructor call reads the selected instance field.
#[test]
fn test_construct_through_instance_class_value() {
    let session = TestSession::single(
        r#"
class User {}

struct Factory {
    user: typeof User;
}

declare const factory: Factory;

const user = new factory.user();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

struct Factory {
    user: new () => User;
}

declare const factory: Factory;

const user: User = new factory.user();

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct Factory {
/// @type.symbol symbol=Factory type=Factory
/// @definition.struct symbol=Factory
/// @definition.field symbol=Factory.user source="user: typeof User" key=user type=Function<(), User, "readonly">

    user: typeof User;
    /// @type.symbol symbol=Factory.user source="user: typeof User" type=Function<(), User, "readonly">
    /// @resolution.name source=User target=User

}

declare const factory: Factory;
/// @type.symbol symbol=factory source=factory type=Factory
/// @resolution.pattern source=factory kind=binding target=factory
/// @resolution.name source=Factory target=Factory

const user = new factory.user();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.call source="new factory.user()" parameters=() return=User kind=expression target=expression
/// @resolution.name source=factory target=factory
/// @resolution.member source=factory.user receiver=Factory type=Function<(), User, "readonly"> kind=field target_receiver=Factory key=user target=Factory.user target_type=Function<(), User, "readonly">
/// @resolution.place source=factory placement="local" lifetime="static" access="immutable"
/// @resolution.access source=factory root=factory
/// @resolution.place source=factory.user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=factory.user root=factory keys=[user]
"#,
    );
}

/// Indexed constructor values construct the selected class.
#[test]
fn test_construct_through_indexed_class_values() {
    let session = TestSession::single(
        r#"
class User {}

declare const constructors: [typeof User];

const user = new constructors[0]();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const constructors: [new () => User];

const user: User = new constructors[0]();

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const constructors: [typeof User];
/// @type.symbol symbol=constructors source=constructors type=Slice<Function<(), User, "readonly">>
/// @resolution.pattern source=constructors kind=binding target=constructors
/// @resolution.name source=User target=User

const user = new constructors[0]();
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.call source="new constructors[0]()" parameters=() return=User kind=expression target=expression
/// @resolution.name source=constructors target=constructors
/// @resolution.place source=constructors placement="local" lifetime="static" access="immutable"
/// @resolution.access source=constructors root=constructors
/// @resolution.subscript source=constructors[0] type=Function<(), User, "readonly"> kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=Function<(), User, \"readonly\">, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<Function<(), User, \"readonly\">, \"managed\" & \"local\">" template=index#2 arguments=(Function<(), User, "readonly">, "managed" & "local")
/// @generic.instance id="Slice<Function<(), User, \"readonly\">>" template=Slice arguments=(Function<(), User, "readonly">)
/// @generic.instance id="index#2<Function<(), User, \"readonly\">, \"bound0\" & \"local\">" template=index#2 arguments=(Function<(), User, "readonly">, "bound0" & "local")
"#,
    );
}

/// Constructor signatures accept arguments and return the declared instance.
#[test]
fn test_construct_through_constructor_signatures() {
    let session = TestSession::single(
        r#"
class User {}

declare const create: new (name: string) => User;

const user = new create("Ada");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

declare const create: new (name: string) => User;

const user: User = new create("Ada");

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

declare const create: new (name: string) => User;
/// @type.symbol symbol=create source=create type=new (string) => User
/// @resolution.pattern source=create kind=binding target=create
/// @type.symbol symbol=name source="name: string" type=string
/// @resolution.name source=User target=User

const user = new create("Ada");
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.call source="new create(\"Ada\")" parameters=(string) arguments=(provided("Ada") as string) return=User kind=expression target=expression
/// @resolution.name source=create target=create
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create
/// @coercion.node source="\"Ada\"" from="Ada" adjustments=[{ kind: materialize, target: string }] origin=implicit
"#,
    );
}

/// Specialized class constructors retain explicit arguments and declared defaults.
#[test]
fn test_construct_through_generic_class_values() {
    let session = TestSession::single(
        r#"
class Box<out T, out U = string> {}

const create = Box<int32>;

const box = new create();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class Box<out T, out U = string> {}

const create: new () => Box<int32> = Box<int32>;

const box: Box<int32> = new create();

=== dir ===
class Box<out T, out U = string> {}
/// @generic.template symbol=Box parameters=(out T, out U = string)
/// @type.symbol symbol=Box source="class Box<out T, out U = string> {}" type=typeof Box
/// @definition.class symbol=Box source="class Box<out T, out U = string> {}" template=(out T, out U = string)
/// @type.symbol symbol=Box.T source="out T" type=T
/// @type.symbol symbol=Box.U source="out U = string" type=U

const create = Box<int32>;
/// @type.symbol symbol=create source=create type=Function<(), Box<int32, string>, "readonly">
/// @resolution.pattern source=create kind=binding target=create
/// @generic.instance id="Box<int32, string>" template=Box arguments=(int32, string)
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box<int32> target=Box
/// @resolution.function source=Box<int32> type=Function<(), Box<int32, string>, "readonly"> target=Box
/// @generic.instantiation id="Box<int32, string>" template=Box arguments=(int32, string)

const box = new create();
/// @type.symbol symbol=box source=box type=Box<int32, string>
/// @resolution.pattern source=box kind=binding target=box
/// @resolution.call source="new create()" parameters=() return=Box<int32, string> kind=expression target=expression generic_arguments=(int32, string)
/// @resolution.name source=create target=create
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create
"#,
    );
}

/// Variables and indexed expressions preserve a generic constructor's parameters.
#[test]
fn test_specialize_constructor_expressions() {
    let session = TestSession::single(
        r#"
class Box<out T> {}

const create = Box;

const integerBox = create<int32>;

declare const constructors: [typeof Box];

const stringBox = constructors[0]<string>;

const first = new integerBox();

const second = new stringBox();
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class Box<out T> {}

const create = Box;

const integerBox: new () => Box<int32> = create<int32>;

declare const constructors: [typeof Box];

const stringBox: new () => Box<string> = constructors[0]<string>;

const first: Box<int32> = new integerBox();

const second: Box<string> = new stringBox();

=== dir ===
class Box<out T> {}
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box source="class Box<out T> {}" type=typeof Box
/// @definition.class symbol=Box source="class Box<out T> {}" template=(out T)
/// @type.symbol symbol=Box.T source="out T" type=T

const create = Box;
/// @type.symbol symbol=create source=create type=Function<(), Box<T>, "readonly">
/// @resolution.pattern source=create kind=binding target=create
/// @resolution.name source=Box target=Box
/// @resolution.function source=Box type=Function<(), Box<T>, "readonly"> target=Box
/// @generic.instantiation id=Box<T> template=Box arguments=(T)

const integerBox = create<int32>;
/// @type.symbol symbol=integerBox source=integerBox type=Function<(), Box<int32>, "readonly">
/// @resolution.pattern source=integerBox kind=binding target=integerBox
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @resolution.name source=create target=create
/// @resolution.function source=create<int32> type=Function<(), Box<int32>, "readonly">
/// @resolution.place source=create placement="local" lifetime="static" access="immutable"
/// @resolution.access source=create root=create

declare const constructors: [typeof Box];
/// @type.symbol symbol=constructors source=constructors type=Slice<Function<(), Box<T>, "readonly">>
/// @resolution.pattern source=constructors kind=binding target=constructors
/// @resolution.name source=Box target=Box

const stringBox = constructors[0]<string>;
/// @type.symbol symbol=stringBox source=stringBox type=Function<(), Box<string>, "readonly">
/// @resolution.pattern source=stringBox kind=binding target=stringBox
/// @generic.instance id=Box<string> template=Box arguments=(string)
/// @resolution.name source=constructors target=constructors
/// @resolution.function source=constructors[0]<string> type=Function<(), Box<string>, "readonly">
/// @resolution.place source=constructors placement="local" lifetime="static" access="immutable"
/// @resolution.access source=constructors root=constructors
/// @resolution.subscript source=constructors[0] type=Function<(), Box<T>, "readonly"> kind=call target="index#2(parameters=(isize), arguments=(provided(0) as isize), return=Function<(), Box<T>, \"readonly\">, regions=(\"managed\" & \"local\"))"
/// @generic.instantiation id="index#2<Function<(), Box<T>, \"readonly\">, \"managed\" & \"local\">" template=index#2 arguments=(Function<(), Box<T>, "readonly">, "managed" & "local")

const first = new integerBox();
/// @type.symbol symbol=first source=first type=Box<int32>
/// @resolution.pattern source=first kind=binding target=first
/// @resolution.call source="new integerBox()" parameters=() return=Box<int32> kind=expression target=expression generic_arguments=(int32)
/// @resolution.name source=integerBox target=integerBox
/// @resolution.place source=integerBox placement="local" lifetime="static" access="immutable"
/// @resolution.access source=integerBox root=integerBox

const second = new stringBox();
/// @type.symbol symbol=second source=second type=Box<string>
/// @resolution.pattern source=second kind=binding target=second
/// @resolution.call source="new stringBox()" parameters=() return=Box<string> kind=expression target=expression generic_arguments=(string)
/// @resolution.name source=stringBox target=stringBox
/// @resolution.place source=stringBox placement="local" lifetime="static" access="immutable"
/// @resolution.access source=stringBox root=stringBox
"#,
    );
}

/// A class constructor has the type selected by typeof on that class.
#[test]
fn test_assign_class_constructor_to_typeof() {
    let session = TestSession::single(
        r#"
class User {}

const ctor: typeof User = User;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
class User {}

const ctor: new () => User = User;

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

const ctor: typeof User = User;
/// @type.symbol symbol=ctor source=ctor type=Function<(), User, "readonly">
/// @resolution.pattern source=ctor kind=binding target=ctor
/// @resolution.name source=User target=User
/// @resolution.name source=User target=User
/// @resolution.function source=User type=Function<(), User, "readonly"> target=User
"#,
    );
}

/// Instances supply field values and cannot serve as construction types.
#[test]
fn test_construct_from_instance_bindings() {
    let session = TestSession::single(
        r#"
class User {}

struct Position {
    x: int32;
}

declare const user: User;

declare const position: Position;

const otherUser = new user();

const otherPosition = position { x: 1 };
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {}

struct Position {
    x: int32;
}

declare const user: User;

declare const position: Position;

const otherUser = new user();

const otherPosition = position { x: 1 };

=== dir ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=typeof User
/// @definition.class symbol=User source="class User {}"

struct Position {
/// @type.symbol symbol=Position type=Position
/// @definition.struct symbol=Position
/// @definition.field symbol=Position.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Position.x source="x: int32" type=int32

}

declare const user: User;
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.name source=User target=User

declare const position: Position;
/// @type.symbol symbol=position source=position type=Position
/// @resolution.pattern source=position kind=binding target=position
/// @resolution.name source=Position target=Position

const otherUser = new user();
/// @type.symbol symbol=otherUser source=otherUser type=<error>
/// @resolution.pattern source=otherUser kind=binding target=otherUser
/// @resolution.rejected source="new user()"
/// @resolution.name source=user target=user
/// @resolution.place source=user placement="local" lifetime="static" access="immutable"
/// @resolution.access source=user root=user

const otherPosition = position { x: 1 };
/// @type.symbol symbol=otherPosition source=otherPosition type=<error>
/// @resolution.pattern source=otherPosition kind=binding target=otherPosition
/// @resolution.name source=position target=position
"#,
        r#"
/// @diagnostic.error id=not-constructible message="type 'User' cannot be constructed with 'new'"
/// @diagnostic.label line=12 column=19 span="new user()" line_source="const otherUser = new user();"
/// @diagnostic.error id=value-used-as-type message="expected a type, found value 'position'"
/// @diagnostic.label line=14 column=23 span="position" line_source="const otherPosition = position { x: 1 };"
"#,
    );
}

/// A copied class constructor creates instances with its declared arguments.
#[test]
fn test_construct_through_a_class_value() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string) {}
}

const ctor = User;

const user = new ctor("Ada");
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class User {
    constructor(name: string) {}
}

const ctor: new (name: string) => User = User;

const user: User = new ctor("Ada");

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(name: string) {}" slot=constructor role=constructor type=(this: &'managed User, string) => User

    constructor(name: string) {}
    /// @type.symbol symbol=User.constructor source="constructor(name: string) {}" type=(this: &'managed User, string) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User
    /// @type.symbol symbol=User.constructor.name source="name: string" type=string

}

const ctor = User;
/// @type.symbol symbol=ctor source=ctor type=Function<(string,), User, "readonly">
/// @resolution.pattern source=ctor kind=binding target=ctor
/// @resolution.name source=User target=User
/// @resolution.function source=User type=Function<(string,), User, "readonly"> target=User

const user = new ctor("Ada");
/// @type.symbol symbol=user source=user type=User
/// @resolution.pattern source=user kind=binding target=user
/// @resolution.call source="new ctor(\"Ada\")" parameters=(string) arguments=(provided("Ada") as string) return=User kind=expression target=expression
/// @resolution.name source=ctor target=ctor
/// @resolution.place source=ctor placement="local" lifetime="static" access="immutable"
/// @resolution.access source=ctor root=ctor
"#,
    );
}

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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor(): this {}
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor source="constructor(): this {}" slot=constructor role=constructor type=(this: &'managed User) => User

    constructor(): this {}
    /// @type.symbol symbol=User.constructor source="constructor(): this {}" type=(this: &'managed User) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User

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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor() {
        return this;
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(this: &'managed User) => User

    constructor() {
    /// @type.symbol symbol=User.constructor type=(this: &'managed User) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User

        return this;
        /// @type.node source=this type=&'managed User
        /// @resolution.receiver source=this kind=this declaration=User type=&'managed User
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    constructor() {
        return;
    }
}

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.method symbol=User.constructor slot=constructor role=constructor type=(this: &'managed User) => User

    constructor() {
    /// @type.symbol symbol=User.constructor type=(this: &'managed User) => User
    /// @type.symbol symbol=User.constructor.this type=&'managed User

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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Counter {
    value: int32 = 0;
}

const counter: Counter = new Counter();

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
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
/// @type.node source=Counter type=typeof Counter
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Base {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

class Derived extends Base {}

const derived: Derived = new Derived(1);

=== dir ===
class Base {
/// @type.symbol symbol=Base type=typeof Base
/// @definition.class symbol=Base
/// @definition.field symbol=Base.value source="value: int32" key=value type=int32
/// @definition.method symbol=Base.constructor slot=constructor role=constructor type=(this: &'managed Base, int32) => Base

    value: int32;
    /// @type.symbol symbol=Base.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Base.constructor type=(this: &'managed Base, int32) => Base
    /// @type.symbol symbol=Base.constructor.this type=&'managed Base
    /// @type.symbol symbol=Base.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=&'managed Base
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Base type=&'managed Base
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Base, target=field(receiver=&'managed Base, target=Base.value, type=int32), type=int32" type=int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Base.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Base.constructor.value

    }
}

class Derived extends Base {}
/// @type.symbol symbol=Derived source="class Derived extends Base {}" type=typeof Derived
/// @definition.class symbol=Derived source="class Derived extends Base {}"
/// @definition.extends symbol=Derived source=Base target=Base
/// @resolution.name source=Base target=Base

const derived = new Derived(1);
/// @type.symbol symbol=derived source=derived type=Derived
/// @resolution.pattern source=derived kind=binding target=derived
/// @type.node source="new Derived(1)" type=Derived
/// @resolution.construct source="new Derived(1)" parameters=(int32) arguments=(provided(1) as int32) return=Derived kind=class target=Derived constructor=forwarded:Base.constructor
/// @type.node source=Derived type=typeof Derived
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Counter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

const counter: Counter = new Counter(1);

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, int32) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, int32) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=&'managed Counter
        /// @type.node source=this.value type=int32
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.value, type=int32), type=int32" type=int32
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
/// @type.node source=Counter type=typeof Counter
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box {
    value: string | int32;

    constructor(value: string) {
        this.value = value as string | int32;
    }

    constructor(value: int32) {
        this.value = value as string | int32;
    }
}

const text: Box = new Box("x");
const number: Box = new Box(1);

=== dir ===
class Box {
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(this: &'managed Box, string) => Box
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(this: &'managed Box, int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(this: &'managed Box, string) => Box
    /// @type.symbol symbol=Box.constructor.this#1 type=&'managed Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=&'managed Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box, target=field(receiver=&'managed Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(this: &'managed Box, int32) => Box
    /// @type.symbol symbol=Box.constructor.this#2 type=&'managed Box
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=&'managed Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box, target=field(receiver=&'managed Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
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
/// @type.node source=Box type=typeof Box
/// @resolution.name source=Box target=Box
/// @type.node source="\"x\"" type="x"

const number = new Box(1);
/// @type.symbol symbol=number source=number type=Box
/// @resolution.pattern source=number kind=binding target=number
/// @type.node source="new Box(1)" type=Box
/// @resolution.construct source="new Box(1)" parameters=(int32) arguments=(provided(1) as int32) return=Box kind=class target=Box constructor=Box.constructor#2
/// @type.node source=Box type=typeof Box
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box {
    value: string | int32;

    constructor(value: string) {
        this.value = value as string | int32;
    }

    constructor(value: int32) {
        this.value = value as string | int32;
    }
}

new Box(true);

=== dir ===
class Box {
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 slot=constructor role=constructor type=(this: &'managed Box, string) => Box
/// @definition.method symbol=Box.constructor#2 slot=constructor role=constructor type=(this: &'managed Box, int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string) {
    /// @type.symbol symbol=Box.constructor#1 type=(this: &'managed Box, string) => Box
    /// @type.symbol symbol=Box.constructor.this#1 type=&'managed Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

        this.value = value;
        /// @type.node source="this.value = value" type=string
        /// @type.node source=this type=&'managed Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box, target=field(receiver=&'managed Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=string
        /// @resolution.name source=value target=Box.constructor.value#1
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#1

    }

    constructor(value: int32) {
    /// @type.symbol symbol=Box.constructor#2 type=(this: &'managed Box, int32) => Box
    /// @type.symbol symbol=Box.constructor.this#2 type=&'managed Box
    /// @type.symbol symbol=Box.constructor.value#2 source="value: int32" type=int32

        this.value = value;
        /// @type.node source="this.value = value" type=int32
        /// @type.node source=this type=&'managed Box
        /// @type.node source=this.value type=string | int32
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box, target=field(receiver=&'managed Box, target=Box.value, type=string | int32), type=string | int32" type=string | int32
        /// @type.node source=value type=int32
        /// @resolution.name source=value target=Box.constructor.value#2
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Box.constructor.value#2

    }
}

new Box(true);
/// @type.node source="new Box(true)" type=<error>
/// @resolution.rejected source="new Box(true)"
/// @type.node source=Box type=typeof Box
/// @resolution.name source=Box target=Box
/// @type.node source=true type=true
"#,
        r#"
/// @diagnostic.error id=no-matching-construct message="no constructor matches arguments ('true')"
/// @diagnostic.label line=14 column=1 span="new Box(true)" line_source="new Box(true);"
/// @diagnostic.note message="the candidate '(value: string) => Box' rejects argument 0: 'true' is not assignable to 'string'"
/// @diagnostic.note message="the candidate '(value: int32) => Box' rejects argument 0: 'true' is not assignable to 'int32'"
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
        "main.tspp",
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
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.value source="value: string | int32" key=value type=string | int32
/// @definition.method symbol=Box.constructor#1 source="constructor(value: string)" slot=constructor role=constructor type=(this: &'managed Box, string) => Box
/// @definition.method symbol=Box.constructor#2 source="constructor(value: int32)" slot=constructor role=constructor type=(this: &'managed Box, int32) => Box

    value: string | int32;
    /// @type.symbol symbol=Box.value source="value: string | int32" type=string | int32

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor#1 source="constructor(value: string)" type=(this: &'managed Box, string) => Box
    /// @type.symbol symbol=Box.constructor.value#1 source="value: string" type=string

    constructor(value: int32);
    /// @type.symbol symbol=Box.constructor#2 source="constructor(value: int32)" type=(this: &'managed Box, int32) => Box
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box {
    constructor(value: string);
}

=== dir ===
class Box {
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.method symbol=Box.constructor source="constructor(value: string)" slot=constructor role=constructor type=(this: &'managed Box, string) => Box

    constructor(value: string);
    /// @type.symbol symbol=Box.constructor source="constructor(value: string)" type=(this: &'managed Box, string) => Box
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
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
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

const dog: Dog = new Dog("rex", 3);

=== dir ===
class Animal {
/// @type.symbol symbol=Animal type=typeof Animal
/// @definition.class symbol=Animal
/// @definition.field symbol=Animal.name source="name: string" key=name type=string
/// @definition.method symbol=Animal.constructor slot=constructor role=constructor type=(this: &'managed Animal, string) => Animal

    name: string;
    /// @type.symbol symbol=Animal.name source="name: string" type=string

    constructor(name: string) {
    /// @type.symbol symbol=Animal.constructor type=(this: &'managed Animal, string) => Animal
    /// @type.symbol symbol=Animal.constructor.this type=&'managed Animal
    /// @type.symbol symbol=Animal.constructor.name source="name: string" type=string

        this.name = name;
        /// @resolution.receiver source=this kind=this declaration=Animal type=&'managed Animal
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.name kind=place
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]
        /// @resolution.assignment source=this.name write="receiver=&'managed Animal, target=field(receiver=&'managed Animal, target=Animal.name, type=string), type=string" type=string
        /// @resolution.name source=name target=Animal.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Animal.constructor.name

    }
}

class Dog extends Animal {
/// @type.symbol symbol=Dog type=typeof Dog
/// @definition.class symbol=Dog
/// @definition.extends symbol=Dog source=Animal target=Animal
/// @definition.field symbol=Dog.tricks source="tricks: int32" key=tricks type=int32
/// @definition.method symbol=Dog.constructor slot=constructor role=constructor type=(this: &'managed Dog, string, int32) => Dog
/// @resolution.name source=Animal target=Animal

    tricks: int32;
    /// @type.symbol symbol=Dog.tricks source="tricks: int32" type=int32

    constructor(name: string, tricks: int32) {
    /// @type.symbol symbol=Dog.constructor type=(this: &'managed Dog, string, int32) => Dog
    /// @type.symbol symbol=Dog.constructor.this type=&'managed Dog
    /// @type.symbol symbol=Dog.constructor.name source="name: string" type=string
    /// @type.symbol symbol=Dog.constructor.tricks source="tricks: int32" type=int32

        super(name);
        /// @resolution.receiver source=super kind=super declaration=Dog type=&'managed Animal
        /// @resolution.place source=super placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=super root=this
        /// @resolution.construct source=super(name) parameters=(string) arguments=(provided(name) as string) return=void kind=class target=Animal constructor=Animal.constructor
        /// @resolution.name source=name target=Dog.constructor.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=Dog.constructor.name

        this.tricks = tricks;
        /// @resolution.receiver source=this kind=this declaration=Dog type=&'managed Dog
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.tricks kind=place
        /// @resolution.place source=this.tricks placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.tricks root=this keys=[tricks]
        /// @resolution.assignment source=this.tricks write="receiver=&'managed Dog, target=field(receiver=&'managed Dog, target=Dog.tricks, type=int32), type=int32" type=int32
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
            "base.tspp",
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
            "main.tspp",
            r#"
import { Animal } from "./base.tspp";

class Dog extends Animal<string> {}

const dog = new Dog("rex");
"#,
        )
        .build();

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Animal } from "./base.tspp";

class Dog extends Animal<string> {}

const dog: Dog = new Dog("rex");

=== dir ===
import { Animal } from "./base.tspp";

class Dog extends Animal<string> {}
/// @type.symbol symbol=Dog source="class Dog extends Animal<string> {}" type=typeof Dog
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

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
class Counter {
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

declare function build(value: int32): Counter;

const make: new (value: int32) => Counter = Counter;
const broken: new (value: int32) => Counter = build;

=== dir ===
class Counter {
/// @type.symbol symbol=Counter type=typeof Counter
/// @definition.class symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32
/// @definition.method symbol=Counter.constructor slot=constructor role=constructor type=(this: &'managed Counter, int32) => Counter

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

    constructor(value: int32) {
    /// @type.symbol symbol=Counter.constructor type=(this: &'managed Counter, int32) => Counter
    /// @type.symbol symbol=Counter.constructor.this type=&'managed Counter
    /// @type.symbol symbol=Counter.constructor.value source="value: int32" type=int32

        this.value = value;
        /// @resolution.receiver source=this kind=this declaration=Counter type=&'managed Counter
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Counter, target=field(receiver=&'managed Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.name source=value target=Counter.constructor.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Counter.constructor.value

    }
}

declare function build(value: int32): Counter;
/// @type.symbol symbol=build source="declare function build(value: int32): Counter" type=(int32) => Counter
/// @resolution.name source=Counter target=Counter

const make: new (value: int32) => Counter = Counter;
/// @type.symbol symbol=make source=make type=new (int32) => Counter
/// @resolution.pattern source=make kind=binding target=make
/// @type.symbol symbol=value#1 source="value: int32" type=int32
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=Counter target=Counter
/// @resolution.function source=Counter type=new (int32) => Counter target=Counter

const broken: new (value: int32) => Counter = build;
/// @type.symbol symbol=broken source=broken type=new (int32) => Counter
/// @resolution.pattern source=broken kind=binding target=broken
/// @type.symbol symbol=value#2 source="value: int32" type=int32
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=build target=build
/// @resolution.function source=build type=Function<(int32,), Counter, "readonly"> target=build
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Function<(value: int32,), Counter, \"readonly\">' is not assignable to type 'new (value: int32) => Counter'"
/// @diagnostic.label line=13 column=47 span="build" line_source="const broken: new (value: int32) => Counter = build;"
/// @diagnostic.related line=13 column=15 span="new (value: int32) => Counter" line_source="const broken: new (value: int32) => Counter = build;" message="expected due to this annotation"
"#,
    );
}

#[test]
fn test_construct_a_class_without_arguments_inside_another_constructor() {
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class State {
    value: unknown | undefined;

    constructor() {
        this.value = undefined as unknown | undefined;
    }
}

class Holder {
    state: State;

    constructor() {
        this.state = new State();
    }
}

=== dir ===
class State {
/// @type.symbol symbol=State type=typeof State
/// @definition.class symbol=State
/// @definition.field symbol=State.value source="value: unknown | undefined" key=value type=unknown | undefined
/// @definition.method symbol=State.constructor slot=constructor role=constructor type=(this: &'managed State) => State

    value: unknown | undefined;
    /// @type.symbol symbol=State.value source="value: unknown | undefined" type=unknown | undefined

    constructor() {
    /// @type.symbol symbol=State.constructor type=(this: &'managed State) => State
    /// @type.symbol symbol=State.constructor.this type=&'managed State

        this.value = undefined;
        /// @type.node source="this.value = undefined" type=undefined
        /// @type.node source=this type=&'managed State
        /// @type.node source=this.value type=unknown | undefined
        /// @resolution.receiver source=this kind=this declaration=State type=&'managed State
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed State, target=field(receiver=&'managed State, target=State.value, type=unknown | undefined), type=unknown | undefined" type=unknown | undefined
        /// @type.node source=undefined type=undefined

    }
}

class Holder {
/// @type.symbol symbol=Holder type=typeof Holder
/// @definition.class symbol=Holder
/// @definition.field symbol=Holder.state source="state: State" key=state type=State
/// @definition.method symbol=Holder.constructor slot=constructor role=constructor type=(this: &'managed Holder) => Holder

    state: State;
    /// @type.symbol symbol=Holder.state source="state: State" type=State
    /// @resolution.name source=State target=State

    constructor() {
    /// @type.symbol symbol=Holder.constructor type=(this: &'managed Holder) => Holder
    /// @type.symbol symbol=Holder.constructor.this type=&'managed Holder

        this.state = new State();
        /// @type.node source="this.state = new State()" type=State
        /// @type.node source=this type=&'managed Holder
        /// @type.node source=this.state type=State
        /// @resolution.receiver source=this kind=this declaration=Holder type=&'managed Holder
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.state kind=place
        /// @resolution.place source=this.state placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.state root=this keys=[state]
        /// @resolution.assignment source=this.state write="receiver=&'managed Holder, target=field(receiver=&'managed Holder, target=Holder.state, type=State), type=State" type=State
        /// @type.node source="new State()" type=State
        /// @resolution.construct source="new State()" parameters=() return=State kind=class target=State constructor=State.constructor
        /// @type.node source=State type=typeof State
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box<in out T> {
    value: T;

    constructor(value: T) {
        this.value = value;
    }
}

const value: int32 = 1;
const box: Box<int32> = new Box<int32>(value);

=== dir ===
class Box<T> {
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box template=(in out T)
/// @definition.field symbol=Box.value source="value: T" key=value type=T
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<T>, T) => Box<T>
/// @type.symbol symbol=Box.T source=T type=T

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

    constructor(value: T) {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<T>, T) => Box<T>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<T>
    /// @type.symbol symbol=Box.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T

        this.value = value;
        /// @type.node source="this.value = value" type=T
        /// @type.node source=this type=&'managed Box<T>
        /// @type.node source=this.value type=T
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box<T>, target=field(receiver=&'managed Box<T>, target=Box.value, type=T), type=T" type=T
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
/// @type.node source=Box type=typeof Box
/// @resolution.name source=Box target=Box
/// @type.node source=value type=int32
/// @resolution.name source=value target=value
/// @resolution.place source=value placement="local" lifetime="static" access="immutable"
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
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Box<in out T = string> {
    value: T | undefined;

    constructor() {
        this.value = undefined as T | undefined;
    }
}

const box: Box = new Box<string>();

=== dir ===
class Box<T = string> {
/// @generic.template symbol=Box parameters=(in out T = string)
/// @type.symbol symbol=Box type=typeof Box
/// @generic.instance id=Box<string> template=Box arguments=(string)
/// @definition.class symbol=Box template=(in out T = string)
/// @definition.field symbol=Box.value source="value: T | undefined" key=value type=T | undefined
/// @definition.method symbol=Box.constructor slot=constructor role=constructor type=(this: &'managed Box<T>) => Box<T>
/// @type.symbol symbol=Box.T source="T = string" type=T

    value: T | undefined;
    /// @type.symbol symbol=Box.value source="value: T | undefined" type=T | undefined
    /// @resolution.name source=T target=Box.T

    constructor() {
    /// @type.symbol symbol=Box.constructor type=(this: &'managed Box<T>) => Box<T>
    /// @type.symbol symbol=Box.constructor.this type=&'managed Box<T>

        this.value = undefined;
        /// @type.node source="this.value = undefined" type=undefined
        /// @type.node source=this type=&'managed Box<T>
        /// @type.node source=this.value type=T | undefined
        /// @resolution.receiver source=this kind=this declaration=Box type=&'managed Box<T>
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.assignment source=this.value write="receiver=&'managed Box<T>, target=field(receiver=&'managed Box<T>, target=Box.value, type=T | undefined), type=T | undefined" type=T | undefined
        /// @type.node source=undefined type=undefined

    }
}

const box = new Box();
/// @type.symbol symbol=box source=box type=Box<string>
/// @resolution.pattern source=box kind=binding target=box
/// @type.node source="new Box()" type=Box<string>
/// @resolution.construct source="new Box()" parameters=() return=Box<string> kind=class target=Box constructor=Box.constructor instance=Box<string>
/// @generic.instantiation id=Box.constructor<string> template=Box.constructor arguments=(string)
/// @generic.instantiation id=Box<string> template=Box arguments=(string)
/// @generic.instance id=Box.constructor<string> template=Box.constructor arguments=(string)
/// @type.node source=Box type=typeof Box
/// @resolution.name source=Box target=Box
"#,
    );
}

#[test]
fn test_reject_a_field_left_uninitialized_on_a_constructor_path() {
    let session = TestSession::single(
        r#"
class Point {
    x: float64;
    y: float64;

    constructor(x: float64) {
        this.x = x;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Point {
    x: float64;
    y: float64;

    constructor(x: float64) {
        this.x = x;
    }
}

=== dir ===
class Point {
/// @type.symbol symbol=Point type=typeof Point
/// @definition.class symbol=Point
/// @definition.field symbol=Point.x source="x: float64" key=x type=float64
/// @definition.field symbol=Point.y source="y: float64" key=y type=float64
/// @definition.method symbol=Point.constructor slot=constructor role=constructor type=(this: &'managed Point, float64) => Point

    x: float64;
    /// @type.symbol symbol=Point.x source="x: float64" type=float64

    y: float64;
    /// @type.symbol symbol=Point.y source="y: float64" type=float64

    constructor(x: float64) {
    /// @type.symbol symbol=Point.constructor type=(this: &'managed Point, float64) => Point
    /// @type.symbol symbol=Point.constructor.this type=&'managed Point
    /// @type.symbol symbol=Point.constructor.x source="x: float64" type=float64

        this.x = x;
        /// @resolution.receiver source=this kind=this declaration=Point type=&'managed Point
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.place source=this.x placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.assignment source=this.x write="receiver=&'managed Point, target=field(receiver=&'managed Point, target=Point.x, type=float64), type=float64" type=float64
        /// @resolution.name source=x target=Point.constructor.x
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=Point.constructor.x

    }
}
"#,
        r#"
/// @diagnostic.error id=field-not-definitely-initialized message="field 'y' is not initialized on every constructor path"
/// @diagnostic.label line=4 column=5 span="y" line_source="y: float64;"
"#,
    );
}

#[test]
fn test_reject_new_outside_the_class_family() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }
newtype Meters = float64;
interface Greet {
    greet(): int32;
}
type Pair = { left: int32; right: int32 };

function invalid(): void {
    const a = new Status();
    const b = new Meters(1.0);
    const c = new Greet();
    const d = new Pair();
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}
newtype Meters = float64;
interface Greet {
    greet(): int32;
}
type Pair = { left: int32; right: int32 };

function invalid(): void {
    const a = new Status();
    const b = new Meters(1.0);
    const c = new Greet();
    const d = new Pair();
}

=== dir ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

newtype Meters = float64;
/// @type.symbol symbol=Meters source="newtype Meters = float64" type=Meters
/// @definition.newtype symbol=Meters source="newtype Meters = float64" backing=float64 constructors=[(float64) => Meters]

interface Greet {
/// @generic.template symbol=Greet parameters=(this: Greet)
/// @type.symbol symbol=Greet type=Greet
/// @definition.interface symbol=Greet template=(this: Greet)
/// @definition.where symbol=Greet relation=satisfies left=this right=Greet
/// @definition.method symbol=Greet.greet source="greet(): int32" slot=greet type=() => int32

    greet(): int32;
    /// @type.symbol symbol=Greet.greet source="greet(): int32" type=() => int32

}
type Pair = { left: int32; right: int32 };
/// @type.symbol symbol=Pair source="type Pair = { left: int32; right: int32 }" type={ left: int32; right: int32 }
/// @definition.type symbol=Pair source="type Pair = { left: int32; right: int32 }" value={ left: int32; right: int32 }
/// @type.symbol symbol=Pair.left source="left: int32" type=int32
/// @type.symbol symbol=Pair.right source="right: int32" type=int32

function invalid(): void {
/// @type.symbol symbol=invalid type=() => void

    const a = new Status();
    /// @type.symbol symbol=invalid.a source=a type=<error>
    /// @resolution.pattern source=a kind=binding target=invalid.a
    /// @resolution.rejected source="new Status()"
    /// @resolution.name source=Status target=Status

    const b = new Meters(1.0);
    /// @type.symbol symbol=invalid.b source=b type=<error>
    /// @resolution.pattern source=b kind=binding target=invalid.b
    /// @resolution.rejected source="new Meters(1.0)"
    /// @resolution.name source=Meters target=Meters

    const c = new Greet();
    /// @type.symbol symbol=invalid.c source=c type=<error>
    /// @resolution.pattern source=c kind=binding target=invalid.c
    /// @resolution.rejected source="new Greet()"
    /// @resolution.name source=Greet target=Greet

    const d = new Pair();
    /// @type.symbol symbol=invalid.d source=d type=<error>
    /// @resolution.pattern source=d kind=binding target=invalid.d
    /// @resolution.rejected source="new Pair()"
    /// @resolution.name source=Pair target=Pair

}
"#,
        r#"
/// @diagnostic.error id=invalid-value-reference message="'Status' is not a value"
/// @diagnostic.label line=10 column=19 span="Status" line_source="const a = new Status();"
/// @diagnostic.error id=invalid-value-reference message="'Meters' is not a value"
/// @diagnostic.label line=11 column=19 span="Meters" line_source="const b = new Meters(1.0);"
/// @diagnostic.help message="construct newtypes with 'T(…)'"
/// @diagnostic.error id=invalid-value-reference message="'Greet' is not a value"
/// @diagnostic.label line=12 column=19 span="Greet" line_source="const c = new Greet();"
/// @diagnostic.error id=invalid-value-reference message="'Pair' is not a value"
/// @diagnostic.label line=13 column=19 span="Pair" line_source="const d = new Pair();"
"#,
    );
}

#[test]
fn test_reject_aggregate_literals_outside_the_value_families() {
    let session = TestSession::single(
        r#"
class Point {
    x: float64;

    constructor(x: float64) {
        this.x = x;
    }
}
enum Status { Idle, Busy }
interface Greet {
    greet(): int32;
}

function invalid(): void {
    const a = Point { x: 1.0 };
    const b = Status { };
    const c = Greet { };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
class Point {
    x: float64;

    constructor(x: float64) {
        this.x = x;
    }
}
enum Status {
    Idle,
    Busy,
}
interface Greet {
    greet(): int32;
}

function invalid(): void {
    const a = Point { x: 1.0 };
    const b = Status {};
    const c = Greet {};
}

=== dir ===
class Point {
/// @type.symbol symbol=Point type=typeof Point
/// @definition.class symbol=Point
/// @definition.field symbol=Point.x source="x: float64" key=x type=float64
/// @definition.method symbol=Point.constructor slot=constructor role=constructor type=(this: &'managed Point, float64) => Point

    x: float64;
    /// @type.symbol symbol=Point.x source="x: float64" type=float64

    constructor(x: float64) {
    /// @type.symbol symbol=Point.constructor type=(this: &'managed Point, float64) => Point
    /// @type.symbol symbol=Point.constructor.this type=&'managed Point
    /// @type.symbol symbol=Point.constructor.x source="x: float64" type=float64

        this.x = x;
        /// @resolution.receiver source=this kind=this declaration=Point type=&'managed Point
        /// @resolution.place source=this placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.x kind=place
        /// @resolution.place source=this.x placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.x root=this keys=[x]
        /// @resolution.assignment source=this.x write="receiver=&'managed Point, target=field(receiver=&'managed Point, target=Point.x, type=float64), type=float64" type=float64
        /// @resolution.name source=x target=Point.constructor.x
        /// @resolution.place source=x placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=x root=Point.constructor.x

    }
}
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

interface Greet {
/// @generic.template symbol=Greet parameters=(this: Greet)
/// @type.symbol symbol=Greet type=Greet
/// @definition.interface symbol=Greet template=(this: Greet)
/// @definition.where symbol=Greet relation=satisfies left=this right=Greet
/// @definition.method symbol=Greet.greet source="greet(): int32" slot=greet type=() => int32

    greet(): int32;
    /// @type.symbol symbol=Greet.greet source="greet(): int32" type=() => int32

}

function invalid(): void {
/// @type.symbol symbol=invalid type=() => void

    const a = Point { x: 1.0 };
    /// @type.symbol symbol=invalid.a source=a type=<error>
    /// @resolution.pattern source=a kind=binding target=invalid.a
    /// @resolution.name source=Point target=Point
    /// @resolution.rejected source="Point { x: 1.0 }"

    const b = Status { };
    /// @type.symbol symbol=invalid.b source=b type=<error>
    /// @resolution.pattern source=b kind=binding target=invalid.b
    /// @resolution.name source=Status target=Status
    /// @resolution.rejected source="Status { }"

    const c = Greet { };
    /// @type.symbol symbol=invalid.c source=c type=<error>
    /// @resolution.pattern source=c kind=binding target=invalid.c
    /// @resolution.name source=Greet target=Greet
    /// @resolution.rejected source="Greet { }"

}
"#,
        r#"
/// @diagnostic.error id=not-constructible message="type 'Point' cannot be constructed with 'T { … }'; construct classes with 'new T(…)'"
/// @diagnostic.label line=15 column=15 span="Point { x: 1.0 }" line_source="const a = Point { x: 1.0 };"
/// @diagnostic.error id=not-constructible message="type 'Status' cannot be constructed with 'T { … }'; construct enum values through their variants"
/// @diagnostic.label line=16 column=15 span="Status { }" line_source="const b = Status { };"
/// @diagnostic.error id=not-constructible message="type 'Greet' cannot be constructed with 'T { … }'"
/// @diagnostic.label line=17 column=15 span="Greet { }" line_source="const c = Greet { };"
"#,
    );
}

#[test]
fn test_refuse_an_elided_setter_through_a_mutable_borrow() {
    let session = TestSession::single(
        r#"
enum Status { Idle, Busy }

struct Machine {
    status: Status;

    get state(): Status {
        this.status
    }

    set state(value: Status) {
        this.status = value;
    }
}

function update(machine: &Machine): void {
    machine.state = Status.Busy;
}

function read(machine: &readonly Machine): Status {
    machine.state
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
enum Status {
    Idle,
    Busy,
}

struct Machine {
    status: Status;

    get state(): Status {
        this.status
    }

    set state(value: Status): void {
        this.status = value;
    }
}

function update<'a>(machine: &'a Machine): void {
    machine.state = Status.Busy;
}

function read<'a>(machine: &'a readonly Machine): Status {
    machine.state
}

=== dir ===
enum Status { Idle, Busy }
/// @type.symbol symbol=Status source="enum Status { Idle, Busy }" type=Status
/// @definition.enum symbol=Status source="enum Status { Idle, Busy }"
/// @definition.variant symbol=Status.Busy source=Busy key=Busy value=1
/// @definition.variant symbol=Status.Idle source=Idle key=Idle value=0
/// @type.symbol symbol=Status.Idle source=Idle type=Status.Idle
/// @type.symbol symbol=Status.Busy source=Busy type=Status.Busy

struct Machine {
/// @type.symbol symbol=Machine type=Machine
/// @definition.struct symbol=Machine
/// @definition.field symbol=Machine.status source="status: Status" key=status type=Status
/// @definition.method symbol=Machine.state#1 slot=state role=getter type=<Machine.state#1.'a>(this: &Machine.state#1.'a readonly Machine) => Status
/// @definition.method symbol=Machine.state#2 slot=state role=setter type=<Machine.state#2.'a>(this: &Machine.state#2.'a Machine, Status) => void

    status: Status;
    /// @type.symbol symbol=Machine.status source="status: Status" type=Status
    /// @resolution.name source=Status target=Status

    get state(): Status {
    /// @generic.template symbol=Machine.state#1 parameters=('a)
    /// @type.symbol symbol=Machine.state#1 type=<Machine.state#1.'a>(this: &Machine.state#1.'a readonly Machine) => Status
    /// @type.symbol symbol=Machine.state.this#1 type=&Machine.state#1.'a readonly Machine
    /// @resolution.name source=Status target=Status

        this.status
        /// @resolution.member source=this.status receiver=&Machine.state#1.'a readonly Machine type=Status kind=field target_receiver=&Machine.state#1.'a readonly Machine key=status target=Machine.status target_type=Status
        /// @resolution.receiver source=this kind=this declaration=Machine type=&Machine.state#1.'a readonly Machine
        /// @resolution.place source=this placement=Machine.state#1.'a lifetime=Machine.state#1.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.status placement=Machine.state#1.'a lifetime=Machine.state#1.'a access="readonly"
        /// @resolution.access source=this.status root=this keys=[status]

    }

    set state(value: Status) {
    /// @generic.template symbol=Machine.state#2 parameters=('a)
    /// @type.symbol symbol=Machine.state#2 type=<Machine.state#2.'a>(this: &Machine.state#2.'a Machine, Status) => void
    /// @type.symbol symbol=Machine.state.this#2 type=&Machine.state#2.'a Machine
    /// @type.symbol symbol=Machine.state.value source="value: Status" type=Status
    /// @resolution.name source=Status target=Status

        this.status = value;
        /// @resolution.receiver source=this kind=this declaration=Machine type=&Machine.state#2.'a Machine
        /// @resolution.place source=this placement=Machine.state#2.'a lifetime=Machine.state#2.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.status kind=place
        /// @resolution.place source=this.status placement=Machine.state#2.'a lifetime=Machine.state#2.'a access="mutable"
        /// @resolution.access source=this.status root=this keys=[status]
        /// @resolution.assignment source=this.status write="receiver=&Machine.state#2.'a Machine, target=field(receiver=&Machine.state#2.'a Machine, target=Machine.status, type=Status), type=Status" type=Status
        /// @resolution.name source=value target=Machine.state.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=Machine.state.value

    }
}

function update(machine: &Machine): void {
/// @generic.template symbol=update parameters=('a)
/// @type.symbol symbol=update type=<update.'a>(&update.'a Machine) => void
/// @type.symbol symbol=update.machine source="machine: &Machine" type=&update.'a Machine
/// @resolution.name source=Machine target=Machine

    machine.state = Status.Busy;
    /// @resolution.name source=machine target=update.machine
    /// @resolution.place source=machine placement=update.'a lifetime=update.'a access="mutable"
    /// @resolution.access source=machine root=update.machine
    /// @resolution.pattern.assign source=machine.state kind=place
    /// @resolution.assignment source=machine.state write="receiver=&update.'a Machine, target=Machine.state#2(parameters=(Status), arguments=(supplied(0) as Status), return=void, regions=(update.'a)), type=Status" type=Status
    /// @generic.instantiation id=Machine.state#2<update.'a> template=Machine.state#2 arguments=(update.'a)
    /// @resolution.name source=Status target=Status
    /// @resolution.member source=Status.Busy receiver=Status type=Status.Busy kind=symbol target_receiver=Status target=Status.Busy

}

function read(machine: &readonly Machine): Status {
/// @generic.template symbol=read parameters=('a)
/// @type.symbol symbol=read type=<read.'a>(&read.'a readonly Machine) => Status
/// @type.symbol symbol=read.machine source="machine: &readonly Machine" type=&read.'a readonly Machine
/// @resolution.name source=Machine target=Machine
/// @resolution.name source=Status target=Status

    machine.state
    /// @resolution.name source=machine target=read.machine
    /// @resolution.member source=machine.state receiver=&read.'a readonly Machine type=Status kind=call target="Machine.state#1(parameters=(), arguments=(), return=Status, regions=(read.'a))"
    /// @resolution.place source=machine placement=read.'a lifetime=read.'a access="readonly"
    /// @resolution.access source=machine root=read.machine
    /// @generic.instantiation id=Machine.state#1<read.'a> template=Machine.state#1 arguments=(read.'a)

}
"#,
        r#"

"#,
    );
}

/// Reject a constructor receiver other than a borrow.
#[test]
fn test_reject_a_constructor_receiver_other_than_a_borrow() {
    let session = TestSession::single(
        r#"
class User {
    constructor(this: User) {}
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
class User {
    constructor(this: User) {}
}

=== dir ===
class User {
    constructor(this: User) {}
}
"#,
        r#"
/// @diagnostic.error id=constructor-receiver-not-borrow message="constructor receiver must be a borrow"
/// @diagnostic.label line=3 column=17 span="this" line_source="constructor(this: User) {}"
"#,
    );
}

#[test]
fn test_reject_this_before_the_super_call() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        this.level = 1;
        super();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::none(), r#"
=== annotated ===
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        this.level = 1;
        super();
    }
}

=== dir ===
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        this.level = 1;
        super();
    }
}
"#, r#"
/// @diagnostic.error id=this-before-super message="'super' must be called before accessing 'this' in the constructor of a derived class"
/// @diagnostic.label line=10 column=9 span="this" line_source="this.level = 1;"
"#);
}

#[test]
fn test_reject_a_derived_constructor_without_a_super_call() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {}
}

class Admin extends User {
    constructor() {}
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::none(), r#"
=== annotated ===
class User {
    constructor() {}
}

class Admin extends User {
    constructor() {}
}

=== dir ===
class User {
    constructor() {}
}

class Admin extends User {
    constructor() {}
}
"#, r#"
/// @diagnostic.error id=missing-super-call message="constructors for derived classes must contain a 'super' call"
/// @diagnostic.label line=7 column=5 span="constructor" line_source="constructor() {}"
"#);
}

#[test]
fn test_reject_a_super_call_outside_the_constructor() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {}
}

class Admin extends User {
    reset(this): void {
        super();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::none(), r#"
=== annotated ===
class User {
    constructor() {}
}

class Admin extends User {
    reset(this): void {
        super();
    }
}

=== dir ===
class User {
    constructor() {}
}

class Admin extends User {
    reset(this): void {
        super();
    }
}
"#, r#"
/// @diagnostic.error id=super-call-outside-constructor message="super calls are permitted only in the constructor of a derived class"
/// @diagnostic.label line=8 column=9 span="super()" line_source="super();"
"#);
}

#[test]
fn test_read_this_after_the_super_call() {
    let session = TestSession::single(
        r#"
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        super();
        this.level = 1;
        this.promote();
    }

    promote(this): void {
        this.level += 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        super();
        this.level = 1;
        this.promote();
    }

    promote(this): void {
        this.level += 1;
    }
}

=== dir ===
class User {
    constructor() {}
}

class Admin extends User {
    level: int32;

    constructor() {
        super();
        this.level = 1;
        this.promote();
    }

    promote(this): void {
        this.level += 1;
    }
}
"#,
        r#"

"#,
    );
}
