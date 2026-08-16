use crate::tests::{DirRows, TestSession};

#[test]
fn test_block_closure_completion_keeps_void_return() {
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function use<T>(callback: () => T | Box<T>): T;

const value = use(() => {});
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function use<T>(callback: () => T | Box<T>): T;

const value: void = use<void>((): void | Box<void> => {});

=== dir ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T#1)
/// @type.symbol symbol=Box.T source="in out T" type=T#1

declare function use<T>(callback: () => T | Box<T>): T;
/// @generic.template symbol=use parameters=(T#2)
/// @type.symbol symbol=use source="declare function use<T>(callback: () => T | Box<T>): T" type=<T#2>(Function<(), T#2 | Box<T#2>>) => T#2
/// @type.symbol symbol=use.T source=T type=T#2
/// @type.symbol symbol=use.callback source="callback: () => T | Box<T>" type=Function<(), T#2 | Box<T#2>>
/// @resolution.name source=T target=use.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=use.T
/// @resolution.name source=T target=use.T

const value = use(() => {});
/// @type.symbol symbol=value source=value type=void
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="use(() => {})" type=void
/// @type.node source=use type=(Function<(), void | Box<void>>) => void
/// @resolution.name source=use target=use
/// @resolution.call source="use(() => {})" parameters=(Function<(), void | Box<void>>) arguments=(provided(() => {}) as Function<(), void | Box<void>>) return=void kind=symbol target=use instance=use<void>
/// @generic.instantiation id=use<void> template=use arguments=(void)
/// @generic.instance id=use<void> template=use arguments=(void)
/// @type.symbol symbol=symbol6 source="() => {}" type=Function<(), void | Box<void>>
/// @type.node source="() => {}" type=Function<(), void | Box<void>>
/// @generic.instance id=Box<void> template=Box arguments=(void)
"#,
    );
}

#[test]
fn test_closure_call_return_injects_into_concrete_union() {
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function load(): int32;
declare function use(callback: () => int32 | Box<int32>): int32;

const value = use(() => load());
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function load(): int32;
declare function use(callback: () => int32 | Box<int32>): int32;

const value: int32 = use((): int32 => load());

=== dir ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T)
/// @type.symbol symbol=Box.T source="in out T" type=T

declare function load(): int32;
/// @type.symbol symbol=load source="declare function load(): int32" type=() => int32

declare function use(callback: () => int32 | Box<int32>): int32;
/// @type.symbol symbol=use source="declare function use(callback: () => int32 | Box<int32>): int32" type=(Function<(), int32 | Box<int32>>) => int32
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @type.symbol symbol=use.callback source="callback: () => int32 | Box<int32>" type=Function<(), int32 | Box<int32>>
/// @resolution.name source=Box target=Box

const value = use(() => load());
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="use(() => load())" type=int32
/// @type.node source=use type=(Function<(), int32 | Box<int32>>) => int32
/// @resolution.name source=use target=use
/// @resolution.call source="use(() => load())" parameters=(Function<(), int32 | Box<int32>>) arguments=(provided(() => load()) as Function<(), int32 | Box<int32>>) return=int32 kind=symbol target=use
/// @type.symbol symbol=symbol6 source=() => load() type=Function<(), int32>
/// @type.node source=() => load() type=Function<(), int32>
/// @type.node source=load type=() => int32
/// @type.node source=load() type=int32
/// @resolution.name source=load target=load
/// @resolution.call source=load() parameters=() return=int32 kind=symbol target=load
"#,
    );
}

#[test]
fn test_closure_return_matching_both_union_arms_requires_annotation() {
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function make(): Box<Box<int32>>;
declare function use<T>(callback: () => Box<T> | Box<Box<T>>): T;

const value = use(() => make());
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function make(): Box<Box<int32>>;
declare function use<T>(callback: () => Box<T> | Box<Box<T>>): T;

const value = use((): Box<Box<int32>> => make());

=== dir ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T#1)
/// @type.symbol symbol=Box.T source="in out T" type=T#1

declare function make(): Box<Box<int32>>;
/// @type.symbol symbol=make source="declare function make(): Box<Box<int32>>" type=() => Box<Box<int32>>
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box

declare function use<T>(callback: () => Box<T> | Box<Box<T>>): T;
/// @generic.template symbol=use parameters=(T#2)
/// @type.symbol symbol=use source="declare function use<T>(callback: () => Box<T> | Box<Box<T>>): T" type=<T#2>(Function<(), Box<T#2> | Box<Box<T#2>>>) => T#2
/// @type.symbol symbol=use.T source=T type=T#2
/// @type.symbol symbol=use.callback source="callback: () => Box<T> | Box<Box<T>>" type=Function<(), Box<T#2> | Box<Box<T#2>>>
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=use.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=use.T
/// @resolution.name source=T target=use.T

const value = use(() => make());
/// @type.symbol symbol=value source=value type=<error>
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="use(() => make())" type=<error>
/// @type.node source=use type=(Function<(), Box<<error>> | Box<Box<<error>>>>) => <error>
/// @resolution.name source=use target=use
/// @resolution.call source="use(() => make())" parameters=(Function<(), Box<<error>> | Box<Box<<error>>>>) arguments=(provided(() => make()) as Function<(), Box<<error>> | Box<Box<<error>>>>) return=<error> kind=symbol target=use instance=use<<error>>
/// @generic.instantiation id=use<<error>> template=use arguments=(<error>)
/// @type.symbol symbol=symbol7 source=() => make() type=Function<(), Box<Box<int32>>>
/// @type.node source=() => make() type=Function<(), Box<Box<int32>>>
/// @type.node source=make type=() => Box<Box<int32>>
/// @type.node source=make() type=Box<Box<int32>>
/// @resolution.name source=make target=make
/// @resolution.call source=make() parameters=() return=Box<Box<int32>> kind=symbol target=make
"#,
        r#"
/// @diagnostic.error id=not-assignable message="type 'Box<Box<int32>>' is not assignable to type 'Box<_> | Box<Box<_>>'"
/// @diagnostic.label line=6 column=19 span="() => make()" line_source="const value = use(() => make());"
/// @diagnostic.related line=6 column=15 span="use(() => make())" line_source="const value = use(() => make());" message="in this call"
/// @diagnostic.note message="inference cannot decide this relation"
/// @diagnostic.note message="the mismatch is in the return type"
/// @diagnostic.help message="annotate the type explicitly"
"#,
    );
}

#[test]
fn test_infer_closure_return_from_parameter() {
    let session = TestSession::single(
        r#"
declare function map<T, U>(value: T, callback: (value: T) => U): U;

const value = map(1, (item) => item);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function map<T, U>(value: T, callback: (arg0: T) => U): U;

const value: float64 = map<float64, float64>(1, (item: float64): float64 => item);

=== dir ===
declare function map<T, U>(value: T, callback: (value: T) => U): U;
/// @generic.template symbol=map parameters=(T, U)
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U): U" type=<T, U>(T, Function<(T,), U>) => U
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=map.value#1 source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=map.callback source="callback: (value: T) => U" type=Function<(T,), U>
/// @type.symbol symbol=map.value#2 source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="map(1, (item) => item)" type=float64
/// @type.node source=map type=(float64, Function<(float64,), float64>) => float64
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(float64, Function<(float64,), float64>) arguments=(provided(1) as float64, provided((item) => item) as Function<(float64,), float64>) return=float64 kind=symbol target=map instance="map<float64, float64>"
/// @generic.instantiation id="map<float64, float64>" template=map arguments=(float64, float64)
/// @generic.instance id="map<float64, float64>" template=map arguments=(float64, float64)
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol7 source="(item) => item" type=Function<(float64,), float64>
/// @type.node source="(item) => item" type=Function<(float64,), float64>
/// @type.symbol symbol=symbol7.item source=item type=float64
/// @type.node source=item type=float64
/// @resolution.name source=item target=symbol7.item
/// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=item root=symbol7.item
"#,
    );
}

#[test]
fn test_infer_method_closure_return_from_receiver() {
    let session = TestSession::single(
        r#"
declare class Box<T> {
    map<U>(callback: (value: T) => U): Box<U>;
}

declare const box: Box<int32>;
const mapped = box.map((value) => value);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<out T> {
    map<U>(callback: (arg0: T) => U): Box<U>;
}

declare const box: Box<int32>;
const mapped: Box<int32> = box.map<int32, int32>((value: int32): int32 => value);

=== dir ===
declare class Box<T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(out T)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" slot=map type=<U>(this: this, Function<(T,), U>) => Box<U>
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U): Box<U>;
    /// @generic.template symbol=Box.map parent=template#0 parameters=(U)
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" type=<U>(this: this, Function<(T,), U>) => Box<U>
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=Box.map.callback source="callback: (value: T) => U" type=Function<(T,), U>
    /// @type.symbol symbol=Box.map.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=U target=Box.map.U

}

declare const box: Box<int32>;
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.pattern source=box kind=binding target=box
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @resolution.name source=Box target=Box

const mapped = box.map((value) => value);
/// @type.symbol symbol=mapped source=mapped type=Box<int32>
/// @resolution.pattern source=mapped kind=binding target=mapped
/// @type.node source="box.map((value) => value)" type=Box<int32>
/// @type.node source=box type=Box<int32>
/// @type.node source=box.map type=<U>(this: Box<int32>, Function<(int32,), U>) => Box<U>
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> type=<U>(this: Box<int32>, Function<(int32,), U>) => Box<U> kind=symbol target_receiver=Box<int32> target=Box.map
/// @resolution.call source="box.map((value) => value)" parameters=(Function<(int32,), int32>) arguments=(provided((value) => value) as Function<(int32,), int32>) return=Box<int32> kind=symbol target=Box.map receiver=Box<int32> instance=Box<int32>.map<int32>
/// @resolution.place source=box placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=box root=box
/// @generic.instantiation id="Box.map<int32, int32>" template=Box.map arguments=(int32, int32)
/// @generic.instantiation id=Box.map<int32> template=Box.map arguments=(int32)
/// @generic.instance id="Box.map<int32, int32>" template=Box.map arguments=(int32, int32)
/// @type.symbol symbol=symbol9 source="(value) => value" type=Function<(int32,), int32>
/// @type.node source="(value) => value" type=Function<(int32,), int32>
/// @type.symbol symbol=symbol9.value source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol9.value
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol9.value
"#,
    );
}

#[test]
fn test_infer_closure_return_through_union() {
    let session = TestSession::single(
        r#"
declare class Box<in out T> {}
declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;

const value = map(1, (item) => item);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<in out T> {}
declare function map<T, U>(value: T, callback: (arg0: T) => U | Box<U>): U;

const value: float64 = map<float64, float64>(1, (item: float64): float64 => item);

=== dir ===
declare class Box<in out T> {}
/// @generic.template symbol=Box parameters=(in out T#1)
/// @type.symbol symbol=Box source="declare class Box<in out T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<in out T> {}" template=(in out T#1)
/// @type.symbol symbol=Box.T source="in out T" type=T#1

declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;
/// @generic.template symbol=map parameters=(T#2, U)
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U" type=<T#2, U>(T#2, Function<(T#2,), U | Box<U>>) => U
/// @type.symbol symbol=map.T source=T type=T#2
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=map.value#1 source="value: T" type=T#2
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=map.callback source="callback: (value: T) => U | Box<U>" type=Function<(T#2,), U | Box<U>>
/// @type.symbol symbol=map.value#2 source="value: T" type=T#2
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value source=value type=float64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="map(1, (item) => item)" type=float64
/// @type.node source=map type=(float64, Function<(float64,), float64 | Box<float64>>) => float64
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(float64, Function<(float64,), float64 | Box<float64>>) arguments=(provided(1) as float64, provided((item) => item) as Function<(float64,), float64 | Box<float64>>) return=float64 kind=symbol target=map instance="map<float64, float64>"
/// @generic.instantiation id="map<float64, float64>" template=map arguments=(float64, float64)
/// @generic.instance id="map<float64, float64>" template=map arguments=(float64, float64)
/// @generic.instance id=Box<float64> template=Box arguments=(float64)
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol9 source="(item) => item" type=Function<(float64,), float64>
/// @type.node source="(item) => item" type=Function<(float64,), float64>
/// @type.symbol symbol=symbol9.item source=item type=float64
/// @type.node source=item type=float64
/// @resolution.name source=item target=symbol9.item
/// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=item root=symbol9.item
"#,
    );
}

#[test]
fn test_infer_method_closure_return_through_union() {
    let session = TestSession::single(
        r#"
declare class Box<T> {
    map<U>(callback: (value: T) => U | Box<U>): U;
}

declare const box: Box<int32>;
const value = box.map((item) => item);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<out T> {
    map<U>(callback: (arg0: T) => U | Box<U>): U;
}

declare const box: Box<int32>;
const value: int32 = box.map<int32, int32>((item: int32): int32 => item);

=== dir ===
declare class Box<T> {
/// @generic.template symbol=Box parameters=(out T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(out T)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" slot=map type=<U>(this: this, Function<(T,), U | Box<U>>) => U
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U | Box<U>): U;
    /// @generic.template symbol=Box.map parent=template#0 parameters=(U)
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" type=<U>(this: this, Function<(T,), U | Box<U>>) => U
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=Box.map.callback source="callback: (value: T) => U | Box<U>" type=Function<(T,), U | Box<U>>
    /// @type.symbol symbol=Box.map.value source="value: T" type=T
    /// @resolution.name source=T target=Box.T
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=U target=Box.map.U

}

declare const box: Box<int32>;
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.pattern source=box kind=binding target=box
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @resolution.name source=Box target=Box

const value = box.map((item) => item);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="box.map((item) => item)" type=int32
/// @type.node source=box type=Box<int32>
/// @type.node source=box.map type=<U>(this: Box<int32>, Function<(int32,), U | Box<U>>) => U
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> type=<U>(this: Box<int32>, Function<(int32,), U | Box<U>>) => U kind=symbol target_receiver=Box<int32> target=Box.map
/// @resolution.call source="box.map((item) => item)" parameters=(Function<(int32,), int32 | Box<int32>>) arguments=(provided((item) => item) as Function<(int32,), int32 | Box<int32>>) return=int32 kind=symbol target=Box.map receiver=Box<int32> instance=Box<int32>.map<int32>
/// @resolution.place source=box placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=box root=box
/// @generic.instantiation id="Box.map<int32, int32>" template=Box.map arguments=(int32, int32)
/// @generic.instantiation id=Box.map<int32> template=Box.map arguments=(int32)
/// @generic.instance id="Box.map<int32, int32>" template=Box.map arguments=(int32, int32)
/// @type.symbol symbol=symbol9 source="(item) => item" type=Function<(int32,), int32>
/// @type.node source="(item) => item" type=Function<(int32,), int32>
/// @type.symbol symbol=symbol9.item source=item type=int32
/// @type.node source=item type=int32
/// @resolution.name source=item target=symbol9.item
/// @resolution.place source=item placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=item root=symbol9.item
"#,
    );
}
