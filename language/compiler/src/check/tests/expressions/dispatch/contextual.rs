use crate::tests::{DirRows, TestSession};

#[test]
fn test_field_initializer_instantiates_a_generic_static_from_the_declared_type() {
    let session = TestSession::single(
        r#"
class Bag<T> {
    values: T[] = Array.new();
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Bag<in out T> {
    values: T[] = Array.new<T>() as T[];
}

=== checked ===
class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T)
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag template=(in out T)
/// @definition.field symbol=Bag.values source="values: T[] = Array.new()" key=values type=Array<T>
/// @type.symbol symbol=Bag.T source=T type=T

    values: T[] = Array.new();
    /// @type.symbol symbol=Bag.values source="values: T[] = Array.new()" type=Array<T>
    /// @resolution.name source=T target=Bag.T
    /// @type.node source=Array type=Array
    /// @type.node source=Array.new type=() => Owned<Array<collections.array.T#6>>
    /// @type.node source=Array.new() type=Owned<Array<T>>
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.member source=Array.new receiver=Array type=() => Owned<Array<collections.array.T#6>> kind=symbol target_receiver=Array target=collections.array.new
    /// @resolution.call source=Array.new() parameters=() return=Owned<Array<T>> kind=symbol target=collections.array.new receiver=Array instance=Array<T>.<extension#6>.new
    /// @generic.instance source=Array.new id=Array<collections.array.T#6>
    /// @generic.instance source=Array.new() id=Array<T>
    /// @generic.instance source=Array.new() id=Array<T>.<extension#6>.new

}

/// @generic.instance id=Array<T> template=collections.array.Array arguments=(T)
/// @generic.instance id=Array<T>.<extension#6>.new template=collections.array.new arguments=(T)
/// @generic.instance id=Array<collections.array.T#6> template=collections.array.Array arguments=(collections.array.T#6)
"#,
    );
}

#[test]
fn test_let_initializer_instantiates_a_generic_static_from_the_annotation() {
    let session = TestSession::single(
        r#"
function build(): void {
    let values: int32[] = Array.new();
    values;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function build(): void {
    let values: int32[] = Array.new<int32>() as int32[];
    values;
}

=== checked ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    let values: int32[] = Array.new();
    /// @type.symbol symbol=build.values source=values type=Array<int32>
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @type.node source=Array type=Array
    /// @type.node source=Array.new type=() => Owned<Array<collections.array.T#6>>
    /// @type.node source=Array.new() type=Owned<Array<int32>>
    /// @resolution.name source=Array target=collections.array.Array
    /// @resolution.member source=Array.new receiver=Array type=() => Owned<Array<collections.array.T#6>> kind=symbol target_receiver=Array target=collections.array.new
    /// @resolution.call source=Array.new() parameters=() return=Owned<Array<int32>> kind=symbol target=collections.array.new receiver=Array instance=Array<int32>.<extension#6>.new
    /// @generic.instance source=Array.new id=Array<collections.array.T#6>
    /// @generic.instance source=Array.new() id=Array<int32>
    /// @generic.instance source=Array.new() id=Array<int32>.<extension#6>.new

    values;
    /// @type.node source=values type=Array<int32>
    /// @resolution.name source=values target=build.values
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=build.values

}

/// @generic.instance id=Array<collections.array.T#6> template=collections.array.Array arguments=(collections.array.T#6)
/// @generic.instance id=Array<int32> template=collections.array.Array arguments=(int32)
/// @generic.instance id=Array<int32>.<extension#6>.new template=collections.array.new arguments=(int32)
"#,
    );
}

#[test]
fn test_construct_rigid_generic_fields_through_bound() {
    let session = TestSession::single(
        r#"
import { Numeric } from "destack:math";

struct Pair<T> {
    x: T;
    y: T;
}

export extension<T: Numeric> of Pair<T> {
    static zero(): Pair<T> {
        Pair { x: T.zero(), y: T.zero() }
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
import { Numeric } from "destack:math";

struct Pair<out T> {
    x: T;
    y: T;
}

export extension<T: Numeric> of Pair<T> {
    static zero(): Pair<T> {
        Pair<T> { x: T.zero(), y: T.zero() }
    }
}

=== checked ===
import { Numeric } from "destack:math";

struct Pair<T> {
/// @generic.template symbol=Pair parameters=(out T#1)
/// @type.symbol symbol=Pair type=Pair
/// @definition.struct symbol=Pair template=(out T#1)
/// @definition.field symbol=Pair.x source="x: T" key=x type=T#1
/// @definition.field symbol=Pair.y source="y: T" key=y type=T#1
/// @type.symbol symbol=Pair.T source=T type=T#1

    x: T;
    /// @type.symbol symbol=Pair.x source="x: T" type=T#1
    /// @resolution.name source=T target=Pair.T

    y: T;
    /// @type.symbol symbol=Pair.y source="y: T" type=T#1
    /// @resolution.name source=T target=Pair.T

}

export extension<T: Numeric> of Pair<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2: math.numeric.Numeric)
/// @definition.extension symbol=<module>#2 form=exported target=Pair<T#2>
/// @definition.method symbol=zero slot=zero static=true type=() => Pair<T#2>
/// @type.symbol symbol=T source="T: Numeric" type=T#2
/// @resolution.name source=Numeric target=math.numeric.Numeric
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T

    static zero(): Pair<T> {
    /// @type.symbol symbol=zero type=() => Pair<T#2>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=T

        Pair { x: T.zero(), y: T.zero() }
        /// @type.node source="Pair { x: T.zero(), y: T.zero() }" type=Pair<T#2>
        /// @resolution.name source=Pair target=Pair
        /// @generic.instance source="Pair { x: T.zero(), y: T.zero() }" id=Pair<T#2>
        /// @type.node source=T type=T#2
        /// @type.node source=T.zero type=() => T#2
        /// @type.node source=T.zero() type=T#2
        /// @resolution.name source=T target=T
        /// @resolution.member source=T.zero type=() => T#2 kind=union arms=[receiver=T#2, target=math.identity.Zero.zero, type=() => T#2, receiver=T#2, target=math.identity.Zero.zero, type=() => T#2]
        /// @resolution.call source=T.zero() return=T#2 kind=union arms=[math.identity.Zero.zero(parameters=(), arguments=(), return=T#2), math.identity.Zero.zero(parameters=(), arguments=(), return=T#2)]
        /// @type.node source=T type=T#2
        /// @type.node source=T.zero type=() => T#2
        /// @type.node source=T.zero() type=T#2
        /// @resolution.name source=T target=T
        /// @resolution.member source=T.zero type=() => T#2 kind=union arms=[receiver=T#2, target=math.identity.Zero.zero, type=() => T#2, receiver=T#2, target=math.identity.Zero.zero, type=() => T#2]
        /// @resolution.call source=T.zero() return=T#2 kind=union arms=[math.identity.Zero.zero(parameters=(), arguments=(), return=T#2), math.identity.Zero.zero(parameters=(), arguments=(), return=T#2)]

    }
}

/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
"#,
    );
}

#[test]
fn test_phantom_parameters_instantiate_from_the_declared_result() {
    // parameters no field mentions still take their canonical
    // instantiation from the declared result type
    let session = TestSession::single(
        r#"
struct Tag<in out T> {
    name: string;
}

export extension<T> of Tag<T> {
    static new(name: string): Tag<T> {
        Tag { name }
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Tag<in out T> {
    name: string;
}

export extension<T> of Tag<T> {
    static new(name: string): Tag<T> {
        Tag<T> { name }
    }
}

=== checked ===
struct Tag<in out T> {
/// @generic.template symbol=Tag parameters=(in out T#1)
/// @type.symbol symbol=Tag type=Tag
/// @definition.struct symbol=Tag template=(in out T#1)
/// @definition.field symbol=Tag.name source="name: string" key=name type=string
/// @type.symbol symbol=Tag.T source="in out T" type=T#1

    name: string;
    /// @type.symbol symbol=Tag.name source="name: string" type=string

}

export extension<T> of Tag<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Tag<T#2>
/// @definition.method symbol=new slot=new static=true type=(string) => Tag<T#2>
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Tag target=Tag
/// @resolution.name source=T target=T

    static new(name: string): Tag<T> {
    /// @type.symbol symbol=new type=(string) => Tag<T#2>
    /// @type.symbol symbol=new.name source="name: string" type=string
    /// @resolution.name source=Tag target=Tag
    /// @resolution.name source=T target=T

        Tag { name }
        /// @type.node source="Tag { name }" type=Tag<T#2>
        /// @resolution.name source=Tag target=Tag
        /// @generic.instance source="Tag { name }" id=Tag<T#2>
        /// @type.node source=name type=string
        /// @resolution.name source=name target=new.name
        /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=name root=new.name

    }
}

/// @generic.instance id=Tag<T#2> template=Tag arguments=(T#2)
"#,
    );
}
