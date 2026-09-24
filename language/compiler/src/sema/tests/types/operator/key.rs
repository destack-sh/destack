use crate::tests::{DirRows, TestSession};

/// A keyof projects the keys an object type declares.
#[test]
fn test_keyof_projects_object_keys() {
    let session = TestSession::single(
        r#"
type User = { name: string; age: int32 };
type Keys = keyof User;

declare const key: Keys;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { name: string; age: int32 };
type Keys = keyof User;

declare const key: Keys;

=== dir ===
type User = { name: string; age: int32 };
/// @type.symbol symbol=User source="type User = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=User source="type User = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=User.name source="name: string" type=string
/// @type.symbol symbol=User.age source="age: int32" type=int32

type Keys = keyof User;
/// @type.symbol symbol=Keys source="type Keys = keyof User" type="name" | "age"
/// @definition.type symbol=Keys source="type Keys = keyof User" value=keyof User
/// @resolution.name source=User target=User

declare const key: Keys;
/// @type.symbol symbol=key source=key type=Keys
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A keyof over a union projects the keys every arm declares.
#[test]
fn test_keyof_union_uses_shared_keys() {
    let session = TestSession::single(
        r#"
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left | Right);

declare const key: Keys;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left | Right);

declare const key: Keys;

=== dir ===
type Left = { shared: string; left: int32 };
/// @type.symbol symbol=Left source="type Left = { shared: string; left: int32 }" type={ shared: string; left: int32 }
/// @definition.type symbol=Left source="type Left = { shared: string; left: int32 }" value={ shared: string; left: int32 }
/// @type.symbol symbol=Left.shared source="shared: string" type=string
/// @type.symbol symbol=Left.left source="left: int32" type=int32

type Right = { shared: string; right: int32 };
/// @type.symbol symbol=Right source="type Right = { shared: string; right: int32 }" type={ shared: string; right: int32 }
/// @definition.type symbol=Right source="type Right = { shared: string; right: int32 }" value={ shared: string; right: int32 }
/// @type.symbol symbol=Right.shared source="shared: string" type=string
/// @type.symbol symbol=Right.right source="right: int32" type=int32

type Keys = keyof (Left | Right);
/// @type.symbol symbol=Keys source="type Keys = keyof (Left | Right)" type="shared"
/// @definition.type symbol=Keys source="type Keys = keyof (Left | Right)" value=keyof Left | Right
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

declare const key: Keys;
/// @type.symbol symbol=key source=key type=Keys
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A key of one union arm reports a diagnostic against the shared keyof.
#[test]
fn test_keyof_union_rejects_arm_specific_key() {
    let session = TestSession::single(
        r#"
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left | Right);

const bad: Keys = "left";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left | Right);

const bad: Keys = "left";

=== dir ===
type Left = { shared: string; left: int32 };
/// @type.symbol symbol=Left source="type Left = { shared: string; left: int32 }" type={ shared: string; left: int32 }
/// @definition.type symbol=Left source="type Left = { shared: string; left: int32 }" value={ shared: string; left: int32 }
/// @type.symbol symbol=Left.shared source="shared: string" type=string
/// @type.symbol symbol=Left.left source="left: int32" type=int32

type Right = { shared: string; right: int32 };
/// @type.symbol symbol=Right source="type Right = { shared: string; right: int32 }" type={ shared: string; right: int32 }
/// @definition.type symbol=Right source="type Right = { shared: string; right: int32 }" value={ shared: string; right: int32 }
/// @type.symbol symbol=Right.shared source="shared: string" type=string
/// @type.symbol symbol=Right.right source="right: int32" type=int32

type Keys = keyof (Left | Right);
/// @type.symbol symbol=Keys source="type Keys = keyof (Left | Right)" type="shared"
/// @definition.type symbol=Keys source="type Keys = keyof (Left | Right)" value=keyof Left | Right
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const bad: Keys = "left";
/// @type.symbol symbol=bad source=bad type=Keys
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Keys target=Keys
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"left\"' is not assignable to type 'Keys'"
/// @diagnostic.label line=6 column=19 span="\"left\"" line_source="const bad: Keys = \"left\";"
/// @diagnostic.related line=6 column=12 span="Keys" line_source="const bad: Keys = \"left\";" message="expected due to this annotation"
/// @diagnostic.note message="'Keys' reduces to '\"shared\"'"
"#,
    );
}

/// A keyof over an intersection projects the keys of both arms.
#[test]
fn test_keyof_intersection_includes_each_key() {
    let session = TestSession::single(
        r#"
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left & Right);

declare const key: Keys;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };
type Keys = keyof (Left & Right);

declare const key: Keys;

=== dir ===
type Left = { shared: string; left: int32 };
/// @type.symbol symbol=Left source="type Left = { shared: string; left: int32 }" type={ shared: string; left: int32 }
/// @definition.type symbol=Left source="type Left = { shared: string; left: int32 }" value={ shared: string; left: int32 }
/// @type.symbol symbol=Left.shared source="shared: string" type=string
/// @type.symbol symbol=Left.left source="left: int32" type=int32

type Right = { shared: string; right: int32 };
/// @type.symbol symbol=Right source="type Right = { shared: string; right: int32 }" type={ shared: string; right: int32 }
/// @definition.type symbol=Right source="type Right = { shared: string; right: int32 }" value={ shared: string; right: int32 }
/// @type.symbol symbol=Right.shared source="shared: string" type=string
/// @type.symbol symbol=Right.right source="right: int32" type=int32

type Keys = keyof (Left & Right);
/// @type.symbol symbol=Keys source="type Keys = keyof (Left & Right)" type="shared" | "left" | "right"
/// @definition.type symbol=Keys source="type Keys = keyof (Left & Right)" value=keyof { shared: string; left: int32; right: int32 }
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

declare const key: Keys;
/// @type.symbol symbol=key source=key type=Keys
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A keyof over a generic alias projects the keys of its instantiated argument.
#[test]
fn test_keyof_generic_alias_uses_instantiated_keys() {
    let session = TestSession::single(
        r#"
type Keys<T: { a: int32 }> = keyof T;
type Actual = Keys<{ a: int32; b: string }>;

declare const key: Actual;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Keys<T: { a: int32 }> = keyof T;
type Actual = Keys<{ a: int32; b: string }>;

declare const key: Actual;

=== dir ===
type Keys<T: { a: int32 }> = keyof T;
/// @generic.template symbol=Keys parameters=(T: { a: int32 })
/// @type.symbol symbol=Keys source="type Keys<T: { a: int32 }> = keyof T" type=keyof T
/// @definition.type symbol=Keys source="type Keys<T: { a: int32 }> = keyof T" template=(T: { a: int32 }) value=keyof T
/// @type.symbol symbol=Keys.T source="T: { a: int32 }" type=T
/// @type.symbol symbol=Keys.a source="a: int32" type=int32
/// @resolution.name source=T target=Keys.T

type Actual = Keys<{ a: int32; b: string }>;
/// @type.symbol symbol=Actual source="type Actual = Keys<{ a: int32; b: string }>" type="a" | "b"
/// @definition.type symbol=Actual source="type Actual = Keys<{ a: int32; b: string }>" value=Keys<{ a: int32; b: string }>
/// @resolution.name source=Keys target=Keys
/// @type.symbol symbol=Actual.a source="a: int32" type=int32
/// @type.symbol symbol=Actual.b source="b: string" type=string

declare const key: Actual;
/// @type.symbol symbol=key source=key type=Actual
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Actual target=Actual
"#,
    );
}

/// A keyof includes optional and readonly field names.
#[test]
fn test_keyof_includes_optional_and_readonly_fields() {
    let session = TestSession::single(
        r#"
type User = { readonly name: string; age?: int32 };
type Keys = keyof User;

declare const key: Keys;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type User = { readonly name: string; age?: int32 };
type Keys = keyof User;

declare const key: Keys;

=== dir ===
type User = { readonly name: string; age?: int32 };
/// @type.symbol symbol=User source="type User = { readonly name: string; age?: int32 }" type={ readonly name: string; age?: int32 }
/// @definition.type symbol=User source="type User = { readonly name: string; age?: int32 }" value={ readonly name: string; age?: int32 }
/// @type.symbol symbol=User.name source="readonly name: string" type=string
/// @type.symbol symbol=User.age source="age?: int32" type=int32

type Keys = keyof User;
/// @type.symbol symbol=Keys source="type Keys = keyof User" type="name" | "age"
/// @definition.type symbol=Keys source="type Keys = keyof User" value=keyof User
/// @resolution.name source=User target=User

declare const key: Keys;
/// @type.symbol symbol=key source=key type=Keys
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A keyof projects the field names a struct declares.
#[test]
fn test_keyof_projects_struct_fields() {
    let session = TestSession::single(
        r#"
struct Point {
    x: int32;
    y: int32;
}

type Keys = keyof Point;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct Point {
    x: int32;
    y: int32;
}

type Keys = keyof Point;

=== dir ===
struct Point {
/// @type.symbol symbol=Point type=Point
/// @definition.struct symbol=Point
/// @definition.field symbol=Point.x source="x: int32" key=x type=int32
/// @definition.field symbol=Point.y source="y: int32" key=y type=int32

    x: int32;
    /// @type.symbol symbol=Point.x source="x: int32" type=int32

    y: int32;
    /// @type.symbol symbol=Point.y source="y: int32" type=int32

}

type Keys = keyof Point;
/// @type.symbol symbol=Keys source="type Keys = keyof Point" type="x" | "y"
/// @definition.type symbol=Keys source="type Keys = keyof Point" value=keyof Point
/// @resolution.name source=Point target=Point
"#,
    );
}

/// A keyof projects the field and method names a class declares.
#[test]
fn test_keyof_projects_class_fields() {
    let session = TestSession::single(
        r#"
class User {
    name: string = "";

    print(): string {
        return this.name;
    }
}

type Keys = keyof User;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {
    name: string = "";

    print(): string {
        return this.name;
    }
}

type Keys = keyof User;

=== dir ===
class User {
/// @type.symbol symbol=User type=typeof User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string = \"\"" key=name type=string
/// @definition.method symbol=User.print slot=print type=(this: User) => string

    name: string = "";
    /// @type.symbol symbol=User.name source="name: string = \"\"" type=string
    /// @type.node source="\"\"" type=""

    print(): string {
    /// @type.symbol symbol=User.print type=(this: User) => string
    /// @type.symbol symbol=User.print.this type=User

        return this.name;
        /// @type.node source=this type=User
        /// @type.node source=this.name type=string
        /// @resolution.member source=this.name receiver=User type=string kind=field target_receiver=User key=name target=User.name target_type=string
        /// @resolution.receiver source=this kind=this declaration=User type=User
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.name root=this keys=[name]

    }
}

type Keys = keyof User;
/// @type.symbol symbol=Keys source="type Keys = keyof User" type="name" | "print"
/// @definition.type symbol=Keys source="type Keys = keyof User" value=keyof User
/// @resolution.name source=User target=User
"#,
    );
}

/// A keyof over a string index signature accepts string and usize keys.
#[test]
fn test_keyof_string_index_signature_includes_string_and_usize_keys() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };
type Keys = keyof Bag;

const text: Keys = "name";
const index: Keys = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };
type Keys = keyof Bag;

const text: Keys = "name" as Keys;
const index: Keys = 1 as Keys;

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

type Keys = keyof Bag;
/// @type.symbol symbol=Keys source="type Keys = keyof Bag" type=string | usize
/// @definition.type symbol=Keys source="type Keys = keyof Bag" value=keyof Bag
/// @resolution.name source=Bag target=Bag

const text: Keys = "name";
/// @type.symbol symbol=text source=text type=Keys
/// @resolution.pattern source=text kind=binding target=text
/// @resolution.name source=Keys target=Keys

const index: Keys = 1;
/// @type.symbol symbol=index source=index type=Keys
/// @resolution.pattern source=index kind=binding target=index
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A boolean key reports a diagnostic against a string index signature keyof.
#[test]
fn test_keyof_string_index_signature_rejects_boolean_key() {
    let session = TestSession::single(
        r#"
type Bag = { readonly [key: string]: int32 };
type Keys = keyof Bag;

const bad: Keys = true;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Bag = { readonly [key: string]: int32 };
type Keys = keyof Bag;

const bad: Keys = true;

=== dir ===
type Bag = { readonly [key: string]: int32 };
/// @type.symbol symbol=Bag source="type Bag = { readonly [key: string]: int32 }" type={ readonly [key: string]: int32 }
/// @definition.type symbol=Bag source="type Bag = { readonly [key: string]: int32 }" value={ readonly [key: string]: int32 }

type Keys = keyof Bag;
/// @type.symbol symbol=Keys source="type Keys = keyof Bag" type=string | usize
/// @definition.type symbol=Keys source="type Keys = keyof Bag" value=keyof Bag
/// @resolution.name source=Bag target=Bag

const bad: Keys = true;
/// @type.symbol symbol=bad source=bad type=Keys
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Keys target=Keys
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'true' is not assignable to type 'Keys'"
/// @diagnostic.label line=5 column=19 span="true" line_source="const bad: Keys = true;"
/// @diagnostic.related line=5 column=12 span="Keys" line_source="const bad: Keys = true;" message="expected due to this annotation"
/// @diagnostic.note message="'Keys' reduces to 'string | usize'"
"#,
    );
}

/// A keyof over a usize index signature accepts usize keys.
#[test]
fn test_keyof_usize_index_signature_uses_usize_keys() {
    let session = TestSession::single(
        r#"
type Slots = { readonly [key: usize]: string };
type Keys = keyof Slots;

const key: Keys = 1;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Slots = { readonly [key: usize]: string };
type Keys = keyof Slots;

const key: Keys = 1;

=== dir ===
type Slots = { readonly [key: usize]: string };
/// @type.symbol symbol=Slots source="type Slots = { readonly [key: usize]: string }" type={ readonly [key: usize]: string }
/// @definition.type symbol=Slots source="type Slots = { readonly [key: usize]: string }" value={ readonly [key: usize]: string }

type Keys = keyof Slots;
/// @type.symbol symbol=Keys source="type Keys = keyof Slots" type=usize
/// @definition.type symbol=Keys source="type Keys = keyof Slots" value=keyof Slots
/// @resolution.name source=Slots target=Slots

const key: Keys = 1;
/// @type.symbol symbol=key source=key type=Keys
/// @resolution.pattern source=key kind=binding target=key
/// @resolution.name source=Keys target=Keys
"#,
    );
}

/// A string key reports a diagnostic against a usize index signature keyof.
#[test]
fn test_keyof_usize_index_signature_rejects_string_key() {
    let session = TestSession::single(
        r#"
type Slots = { readonly [key: usize]: string };
type Keys = keyof Slots;

const bad: Keys = "name";
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Slots = { readonly [key: usize]: string };
type Keys = keyof Slots;

const bad: Keys = "name";

=== dir ===
type Slots = { readonly [key: usize]: string };
/// @type.symbol symbol=Slots source="type Slots = { readonly [key: usize]: string }" type={ readonly [key: usize]: string }
/// @definition.type symbol=Slots source="type Slots = { readonly [key: usize]: string }" value={ readonly [key: usize]: string }

type Keys = keyof Slots;
/// @type.symbol symbol=Keys source="type Keys = keyof Slots" type=usize
/// @definition.type symbol=Keys source="type Keys = keyof Slots" value=keyof Slots
/// @resolution.name source=Slots target=Slots

const bad: Keys = "name";
/// @type.symbol symbol=bad source=bad type=Keys
/// @resolution.pattern source=bad kind=binding target=bad
/// @resolution.name source=Keys target=Keys
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type '\"name\"' is not assignable to type 'Keys'"
/// @diagnostic.label line=5 column=19 span="\"name\"" line_source="const bad: Keys = \"name\";"
/// @diagnostic.related line=5 column=12 span="Keys" line_source="const bad: Keys = \"name\";" message="expected due to this annotation"
/// @diagnostic.note message="'Keys' reduces to 'usize'"
"#,
    );
}

/// A conditional extending keyof answers whether an object declares a key.
#[test]
fn test_key_membership_conditionals_answer_object_keys() {
    let session = TestSession::single(
        r#"
type Person = { name: string; age: int32 };
type HasName = "name" extends keyof Person ? true : false;
type HasTitle = "title" extends keyof Person ? true : false;

const name: HasName = true;
const title: HasTitle = false;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Person = { name: string; age: int32 };
type HasName = "name" extends keyof Person ? true : false;
type HasTitle = "title" extends keyof Person ? true : false;

const name: HasName = true;
const title: HasTitle = false;

=== dir ===
type Person = { name: string; age: int32 };
/// @type.symbol symbol=Person source="type Person = { name: string; age: int32 }" type={ name: string; age: int32 }
/// @definition.type symbol=Person source="type Person = { name: string; age: int32 }" value={ name: string; age: int32 }
/// @type.symbol symbol=Person.name source="name: string" type=string
/// @type.symbol symbol=Person.age source="age: int32" type=int32

type HasName = "name" extends keyof Person ? true : false;
/// @type.symbol symbol=HasName source="type HasName = \"name\" extends keyof Person ? true : false" type=true
/// @definition.type symbol=HasName source="type HasName = \"name\" extends keyof Person ? true : false" value="name" extends keyof Person ? true : false
/// @resolution.name source=Person target=Person

type HasTitle = "title" extends keyof Person ? true : false;
/// @type.symbol symbol=HasTitle source="type HasTitle = \"title\" extends keyof Person ? true : false" type=false
/// @definition.type symbol=HasTitle source="type HasTitle = \"title\" extends keyof Person ? true : false" value="title" extends keyof Person ? true : false
/// @resolution.name source=Person target=Person

const name: HasName = true;
/// @type.symbol symbol=name source=name type=HasName
/// @resolution.pattern source=name kind=binding target=name
/// @resolution.name source=HasName target=HasName

const title: HasTitle = false;
/// @type.symbol symbol=title source=title type=HasTitle
/// @resolution.pattern source=title kind=binding target=title
/// @resolution.name source=HasTitle target=HasTitle
"#,
    );
}

/// A key membership conditional follows the keyof of unions and intersections.
#[test]
fn test_key_membership_conditionals_follow_union_and_intersection_keys() {
    let session = TestSession::single(
        r#"
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasUnionLeft = "left" extends keyof (Left | Right) ? true : false;
type HasUnionShared = "shared" extends keyof (Left | Right) ? true : false;
type HasIntersectionLeft = "left" extends keyof (Left & Right) ? true : false;

const unionLeft: HasUnionLeft = false;
const unionShared: HasUnionShared = true;
const intersectionLeft: HasIntersectionLeft = true;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Left = { shared: string; left: int32 };
type Right = { shared: string; right: int32 };

type HasUnionLeft = "left" extends keyof (Left | Right) ? true : false;
type HasUnionShared = "shared" extends keyof (Left | Right) ? true : false;
type HasIntersectionLeft = "left" extends keyof (Left & Right) ? true : false;

const unionLeft: HasUnionLeft = false;
const unionShared: HasUnionShared = true;
const intersectionLeft: HasIntersectionLeft = true;

=== dir ===
type Left = { shared: string; left: int32 };
/// @type.symbol symbol=Left source="type Left = { shared: string; left: int32 }" type={ shared: string; left: int32 }
/// @definition.type symbol=Left source="type Left = { shared: string; left: int32 }" value={ shared: string; left: int32 }
/// @type.symbol symbol=Left.shared source="shared: string" type=string
/// @type.symbol symbol=Left.left source="left: int32" type=int32

type Right = { shared: string; right: int32 };
/// @type.symbol symbol=Right source="type Right = { shared: string; right: int32 }" type={ shared: string; right: int32 }
/// @definition.type symbol=Right source="type Right = { shared: string; right: int32 }" value={ shared: string; right: int32 }
/// @type.symbol symbol=Right.shared source="shared: string" type=string
/// @type.symbol symbol=Right.right source="right: int32" type=int32

type HasUnionLeft = "left" extends keyof (Left | Right) ? true : false;
/// @type.symbol symbol=HasUnionLeft source="type HasUnionLeft = \"left\" extends keyof (Left | Right) ? true : false" type=false
/// @definition.type symbol=HasUnionLeft source="type HasUnionLeft = \"left\" extends keyof (Left | Right) ? true : false" value="left" extends keyof Left | Right ? true : false
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

type HasUnionShared = "shared" extends keyof (Left | Right) ? true : false;
/// @type.symbol symbol=HasUnionShared source="type HasUnionShared = \"shared\" extends keyof (Left | Right) ? true : false" type=true
/// @definition.type symbol=HasUnionShared source="type HasUnionShared = \"shared\" extends keyof (Left | Right) ? true : false" value="shared" extends keyof Left | Right ? true : false
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

type HasIntersectionLeft = "left" extends keyof (Left & Right) ? true : false;
/// @type.symbol symbol=HasIntersectionLeft source="type HasIntersectionLeft = \"left\" extends keyof (Left & Right) ? true : false" type=true
/// @definition.type symbol=HasIntersectionLeft source="type HasIntersectionLeft = \"left\" extends keyof (Left & Right) ? true : false" value="left" extends keyof { shared: string; left: int32; right: int32 } ? true : false
/// @resolution.name source=Left target=Left
/// @resolution.name source=Right target=Right

const unionLeft: HasUnionLeft = false;
/// @type.symbol symbol=unionLeft source=unionLeft type=HasUnionLeft
/// @resolution.pattern source=unionLeft kind=binding target=unionLeft
/// @resolution.name source=HasUnionLeft target=HasUnionLeft

const unionShared: HasUnionShared = true;
/// @type.symbol symbol=unionShared source=unionShared type=HasUnionShared
/// @resolution.pattern source=unionShared kind=binding target=unionShared
/// @resolution.name source=HasUnionShared target=HasUnionShared

const intersectionLeft: HasIntersectionLeft = true;
/// @type.symbol symbol=intersectionLeft source=intersectionLeft type=HasIntersectionLeft
/// @resolution.pattern source=intersectionLeft kind=binding target=intersectionLeft
/// @resolution.name source=HasIntersectionLeft target=HasIntersectionLeft
"#,
    );
}
