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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Bag<in out T> {
    values: T[] = Array.new<T>() as T[];
}

=== dir ===
class Bag<T> {
/// @generic.template symbol=Bag parameters=(in out T)
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag template=(in out T)
/// @definition.field symbol=Bag.values source="values: T[] = Array.new()" key=values type=T[]
/// @type.symbol symbol=Bag.T source=T type=T

    values: T[] = Array.new();
    /// @type.symbol symbol=Bag.values source="values: T[] = Array.new()" type=T[]
    /// @generic.instance id=Array<T> template=Array arguments=(T)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<T>> template=sliceAssumeInit arguments=(MaybeUninit<T>)
    /// @generic.instance id=sliceUninit<MaybeUninit<T>> template=sliceUninit arguments=(MaybeUninit<T>)
    /// @resolution.name source=T target=Bag.T
    /// @type.node source=Array type=Array
    /// @type.node source=Array.new type=() => ^T#6[]
    /// @type.node source=Array.new() type=^T[]
    /// @resolution.name source=Array target=Array
    /// @resolution.member source=Array.new receiver=Array type=() => ^T#6[] kind=symbol target_receiver=Array target=new
    /// @resolution.call source=Array.new() parameters=() return=^T[] kind=symbol target=new instance=Array<T>.<extension#6>.new
    /// @generic.instantiation id=new<T> template=new arguments=(T) owner=Bag
    /// @generic.instance id=new<T> template=new arguments=(T)

}
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

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function build(): void {
    let values: int32[] = Array.new<int32>() as int32[];
    values;
}

=== dir ===
function build(): void {
/// @type.symbol symbol=build type=() => void

    let values: int32[] = Array.new();
    /// @type.symbol symbol=build.values source=values type=int32[]
    /// @resolution.pattern source=values kind=binding target=build.values
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
    /// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
    /// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id=Array<int32> template=Array arguments=(int32)
    /// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
    /// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
    /// @generic.instance id=clear<int32> template=clear arguments=(int32)
    /// @generic.instance id=drop<int32> template=drop arguments=(int32)
    /// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
    /// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
    /// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
    /// @type.node source=Array type=Array
    /// @type.node source=Array.new type=() => ^T#6[]
    /// @type.node source=Array.new() type=^int32[]
    /// @resolution.name source=Array target=Array
    /// @resolution.member source=Array.new receiver=Array type=() => ^T#6[] kind=symbol target_receiver=Array target=new
    /// @resolution.call source=Array.new() parameters=() return=^int32[] kind=symbol target=new instance=Array<int32>.<extension#6>.new
    /// @generic.instantiation id=new<int32> template=new arguments=(int32)
    /// @generic.instance id=new<int32> template=new arguments=(int32)

    values;
    /// @type.node source=values type=int32[]
    /// @resolution.name source=values target=build.values
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=build.values

}
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

    session.assert_dir(
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

=== dir ===
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
/// @generic.template symbol=<module>#2 parameters=(T#2: Numeric)
/// @generic.instance id=Pair<T#2> template=Pair arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=exported target=Pair<T#2>
/// @definition.method symbol=zero slot=zero static=true type=() => Pair<T#2>
/// @type.symbol symbol=T source="T: Numeric" type=T#2
/// @resolution.name source=Numeric target=Numeric
/// @resolution.name source=Pair target=Pair
/// @resolution.name source=T target=T

    static zero(): Pair<T> {
    /// @type.symbol symbol=zero type=() => Pair<T#2>
    /// @resolution.name source=Pair target=Pair
    /// @resolution.name source=T target=T

        Pair { x: T.zero(), y: T.zero() }
        /// @type.node source="Pair { x: T.zero(), y: T.zero() }" type=Pair<T#2>
        /// @resolution.name source=Pair target=Pair
        /// @type.node source=T type=T#2
        /// @type.node source=T.zero type=() => T#2
        /// @type.node source=T.zero() type=T#2
        /// @resolution.name source=T target=T
        /// @resolution.member source=T.zero type=() => T#2 kind=union arms=[receiver=T#2, target=Zero.zero, type=() => T#2, receiver=T#2, target=Zero.zero, type=() => T#2]
        /// @resolution.call source=T.zero() parameters=() return=T#2 kind=symbol target=Zero.zero
        /// @generic.instantiation id=Zero.zero<T#2> template=Zero.zero arguments=() owner=zero
        /// @generic.instance id=Zero.zero<T#2> template=Zero.zero arguments=()
        /// @type.node source=T type=T#2
        /// @type.node source=T.zero type=() => T#2
        /// @type.node source=T.zero() type=T#2
        /// @resolution.name source=T target=T
        /// @resolution.member source=T.zero type=() => T#2 kind=union arms=[receiver=T#2, target=Zero.zero, type=() => T#2, receiver=T#2, target=Zero.zero, type=() => T#2]
        /// @resolution.call source=T.zero() parameters=() return=T#2 kind=symbol target=Zero.zero

    }
}
"#,
    );
}

/// Instantiate parameters no field mentions from the declared result type.
#[test]
fn test_phantom_parameters_instantiate_from_the_declared_result() {
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

    session.assert_dir(
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

=== dir ===
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
/// @generic.instance id=Tag<T#2> template=Tag arguments=(T#2)
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
        /// @type.node source=name type=string
        /// @resolution.name source=name target=new.name
        /// @resolution.place source=name placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=name root=new.name

    }
}
"#,
    );
}

#[test]
fn test_destructured_callback_parameter_reads_contextual_fields() {
    let session = TestSession::single(
        r#"
function values(entries: { value: int32 }[]): int32[] {
    return entries.map(({ value }) => value);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function values(entries: { value: int32 }[]): int32[] {
    return entries.map<{ value: int32 }, int32>(({ value }): int32 => value) as int32[];
}

=== dir ===
function values(entries: { value: int32 }[]): int32[] {
/// @type.symbol symbol=values type=({ value: int32 }[]) => int32[]
/// @generic.instance id="Array<{ value: int32 }>" template=Array arguments=({ value: int32 })
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitDrop#1<{ value: int32 }>" template=assumeInitDrop#1 arguments=({ value: int32 })
/// @generic.instance id="assumeInitDrop<{ value: int32 }>" template=assumeInitDrop arguments=({ value: int32 })
/// @generic.instance id="clear<{ value: int32 }>" template=clear arguments=({ value: int32 })
/// @generic.instance id="drop<{ value: int32 }>" template=drop arguments=({ value: int32 })
/// @generic.instance id="dropInPlace<{ value: int32 }>" template=dropInPlace arguments=({ value: int32 })
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="elementSlot<{ value: int32 }, \"mutable\">" template=elementSlot arguments=({ value: int32 }, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<{ value: int32 }, \"mutable\">" template=initAsPointer arguments=({ value: int32 }, "mutable")
/// @generic.instance id="sliceAssumeInit<MaybeUninit<{ value: int32 }>>" template=sliceAssumeInit arguments=(MaybeUninit<{ value: int32 }>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<{ value: int32 }>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<{ value: int32 }>, "mutable")
/// @generic.instance id="sliceUninit<MaybeUninit<{ value: int32 }>>" template=sliceUninit arguments=(MaybeUninit<{ value: int32 }>)
/// @generic.instance id="truncate<{ value: int32 }>" template=truncate arguments=({ value: int32 })
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=values.entries source="entries: { value: int32 }[]" type={ value: int32 }[]
/// @type.symbol symbol=values.value source="value: int32" type=int32

    return entries.map(({ value }) => value);
    /// @resolution.name source=entries target=values.entries
    /// @resolution.member source=entries.map receiver={ value: int32 }[] type=<map.U#2, map#2.P1: Place>(this: Managed<{ value: int32 }[], map#2.P1>, Function<({ value: int32 }, isize), map.U#2>) => ^map.U#2[] kind=symbol target_receiver={ value: int32 }[] target=map#2
    /// @resolution.call source="entries.map(({ value }) => value)" parameters=(Function<({ value: int32 }, isize), int32>) arguments=(provided(({ value }) => value) as Function<({ value: int32 }, isize), int32>) return=^int32[] kind=symbol target=map#2 receiver={ value: int32 }[] instance="Array<{ value: int32 }>.<extension#4>.map#2<int32, \"local\">"
    /// @resolution.place source=entries placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=entries root=values.entries
    /// @generic.instantiation id="map#2<{ value: int32 }, int32, \"local\">" template=map#2 arguments=({ value: int32 }, int32, "local")
    /// @generic.instantiation id="map#2<{ value: int32 }>" template=map#2 arguments=({ value: int32 })
    /// @generic.instance id="map#2<{ value: int32 }, int32, \"local\">" template=map#2 arguments=({ value: int32 }, int32, "local")
    /// @type.symbol symbol=values.symbol5 source="({ value }) => value" type=Function<({ value: int32 },), int32, "readonly">
    /// @resolution.pattern source={ value } kind=object fields={ value }
    /// @type.symbol symbol=values.symbol5.value source=value type=int32
    /// @resolution.name source=value target=values.symbol5.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=value root=values.symbol5.value

}
"#,
    );
}

#[test]
fn test_must_callback_result_infers_the_unwrapped_element() {
    let session = TestSession::single(
        r#"
function unwrap(values: (int32 | undefined)[]): int32[] {
    return values.map((value) => value!);
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function unwrap(values: (int32 | undefined)[]): int32[] {
    return values.map<int32 | undefined, int32>(
        (value: int32 | undefined): int32 => value!,
    ) as int32[];
}

=== dir ===
function unwrap(values: (int32 | undefined)[]): int32[] {
/// @type.symbol symbol=unwrap type=(int32 | undefined[]) => int32[]
/// @generic.instance id="Array<int32 | undefined>" template=Array arguments=(int32 | undefined)
/// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
/// @generic.instance id="assumeInitDrop#1<int32 | undefined>" template=assumeInitDrop#1 arguments=(int32 | undefined)
/// @generic.instance id="assumeInitDrop<int32 | undefined>" template=assumeInitDrop arguments=(int32 | undefined)
/// @generic.instance id="clear<int32 | undefined>" template=clear arguments=(int32 | undefined)
/// @generic.instance id="drop<int32 | undefined>" template=drop arguments=(int32 | undefined)
/// @generic.instance id="dropInPlace<int32 | undefined>" template=dropInPlace arguments=(int32 | undefined)
/// @generic.instance id="elementSlot<int32 | undefined, \"mutable\">" template=elementSlot arguments=(int32 | undefined, "mutable")
/// @generic.instance id="elementSlot<int32, \"mutable\">" template=elementSlot arguments=(int32, "mutable")
/// @generic.instance id="initAsPointer<int32 | undefined, \"mutable\">" template=initAsPointer arguments=(int32 | undefined, "mutable")
/// @generic.instance id="initAsPointer<int32, \"mutable\">" template=initAsPointer arguments=(int32, "mutable")
/// @generic.instance id="sliceAssumeInit<MaybeUninit<int32 | undefined>>" template=sliceAssumeInit arguments=(MaybeUninit<int32 | undefined>)
/// @generic.instance id="sliceIndex<MaybeUninit<int32 | undefined>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32 | undefined>, "mutable")
/// @generic.instance id="sliceIndex<MaybeUninit<int32>, \"mutable\">" template=sliceIndex arguments=(MaybeUninit<int32>, "mutable")
/// @generic.instance id="sliceUninit<MaybeUninit<int32 | undefined>>" template=sliceUninit arguments=(MaybeUninit<int32 | undefined>)
/// @generic.instance id="truncate<int32 | undefined>" template=truncate arguments=(int32 | undefined)
/// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=assumeInitDrop#1<int32> template=assumeInitDrop#1 arguments=(int32)
/// @generic.instance id=assumeInitDrop<int32> template=assumeInitDrop arguments=(int32)
/// @generic.instance id=clear<int32> template=clear arguments=(int32)
/// @generic.instance id=drop<int32> template=drop arguments=(int32)
/// @generic.instance id=dropInPlace<int32> template=dropInPlace arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)
/// @generic.instance id=truncate<int32> template=truncate arguments=(int32)
/// @type.symbol symbol=unwrap.values source="values: (int32 | undefined)[]" type=int32 | undefined[]

    return values.map((value) => value!);
    /// @resolution.name source=values target=unwrap.values
    /// @resolution.member source=values.map receiver=int32 | undefined[] type=<map.U#2, map#2.P1: Place>(this: Managed<int32 | undefined[], map#2.P1>, Function<(int32 | undefined, isize), map.U#2>) => ^map.U#2[] kind=symbol target_receiver=int32 | undefined[] target=map#2
    /// @resolution.call source="values.map((value) => value!)" parameters=(Function<(int32 | undefined, isize), int32>) arguments=(provided((value) => value!) as Function<(int32 | undefined, isize), int32>) return=^int32[] kind=symbol target=map#2 receiver=int32 | undefined[] instance="Array<int32 | undefined>.<extension#4>.map#2<int32, \"local\">"
    /// @resolution.place source=values placement="local" lifetime="managed" access="mutable"
    /// @resolution.access source=values root=unwrap.values
    /// @generic.instantiation id="map#2<int32 | undefined, int32, \"local\">" template=map#2 arguments=(int32 | undefined, int32, "local")
    /// @generic.instantiation id="map#2<int32 | undefined>" template=map#2 arguments=(int32 | undefined)
    /// @generic.instance id="map#2<int32 | undefined, int32, \"local\">" template=map#2 arguments=(int32 | undefined, int32, "local")
    /// @type.symbol symbol=unwrap.symbol3 source="(value) => value!" type=Function<(int32 | undefined,), int32, "readonly">
    /// @type.symbol symbol=unwrap.symbol3.value source=value type=int32 | undefined
    /// @resolution.name source=value target=unwrap.symbol3.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="mutable"
    /// @resolution.access source=value root=unwrap.symbol3.value
    /// @resolution.residual source=value! target=trap residual=TryResidual<int32 | undefined>

}
"#,
    );
}
