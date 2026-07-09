use crate::tests::{DirRows, TestSession};

#[test]
fn test_block_closure_completion_keeps_void_return() {
    let session = TestSession::single(
        r#"
declare class Box<T> {}
declare function use<T>(callback: () => T | Box<T>): T;

const value = use(() => {});
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<T> {}
declare function use<T>(callback: () => T | Box<T>): T;

const value: void = use<void>((): void | Box<void> => {});

=== checked ===
declare class Box<T> {}
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="declare class Box<T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<T> {}" template=(T#1)
/// @type.symbol symbol=Box.T source=T type=T#1

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
/// @type.node source="use(() => {})" type=void
/// @type.node source=use type=(Function<(), void | Box<void>>) => void
/// @resolution.name source=use target=use
/// @resolution.call source="use(() => {})" parameters=(Function<(), void | Box<void>>) arguments=(provided(() => {}) as Function<(), void | Box<void>>) return=void kind=symbol target=use instance=use<void>
/// @generic.instance source="use(() => {})" id=use<void>
/// @generic.instance source=use id=Box<void>
/// @type.symbol symbol=symbol6 source="() => {}" type=Function<(), void | Box<void>>
/// @type.node source="() => {}" type=Function<(), void | Box<void>>
/// @generic.instance source="() => {}" id=Box<void>

/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Box<void> template=Box arguments=(void)
/// @generic.instance id=use<void> template=use arguments=(void)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function map<T, U>(value: T, callback: (arg0: T) => U): U;

const value: 1 = map<1, 1>(1, (item: 1): 1 => item);

=== checked ===
declare function map<T, U>(value: T, callback: (value: T) => U): U;
/// @generic.template symbol=map parameters=(T, U)
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U): U" type=<T, U>(T, Function<(T,), U>) => U
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=map.value#1 source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=map.callback source="callback: (value: T) => U" type=Function<(T,), U>
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value source=value type=1
/// @type.node source="map(1, (item) => item)" type=1
/// @type.node source=map type=(1, Function<(1,), 1>) => 1
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(1, Function<(1,), 1>) arguments=(provided(1) as 1, provided((item) => item) as Function<(1,), 1>) return=1 kind=symbol target=map instance="map<1, 1>"
/// @generic.instance source="map(1, (item) => item)" id="map<1, 1>"
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol7 source="(item) => item" type=Function<(1,), 1>
/// @type.node source="(item) => item" type=Function<(1,), 1>
/// @type.symbol symbol=symbol7.item source=item type=1
/// @type.node source=item type=1
/// @resolution.name source=item target=symbol7.item

/// @generic.instance id="map<1, 1>" template=map arguments=(1, 1)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<T> {
    map<U>(callback: (arg0: T) => U): Box<U>;
}

declare const box: Box<int32>;
const mapped: Box<int32> = box.map<int32, int32>((value: int32): int32 => value);

=== checked ===
declare class Box<T> {
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(T)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" slot=map type=<U>(this: Box<T>, Function<(T,), U>) => Box<U>
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U): Box<U>;
    /// @generic.template symbol=Box.map parent=template#0 parameters=(U)
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" type=<U>(this: Box<T>, Function<(T,), U>) => Box<U>
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=Box.map.callback source="callback: (value: T) => U" type=Function<(T,), U>
    /// @resolution.name source=T target=Box.T
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=U target=Box.map.U

}

declare const box: Box<int32>;
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.name source=Box target=Box

const mapped = box.map((value) => value);
/// @type.symbol symbol=mapped source=mapped type=Box<int32>
/// @type.node source="box.map((value) => value)" type=Box<int32>
/// @type.node source=box type=Box<int32>
/// @type.node source=box.map type=<U>(this: Box<int32>, Function<(int32,), U>) => Box<U>
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> kind=symbol target=Box.map
/// @resolution.call source="box.map((value) => value)" parameters=(Function<(int32,), int32>) arguments=(provided((value) => value) as Function<(int32,), int32>) return=Box<int32> kind=symbol target=Box.map receiver=Box<int32> instance=Box<int32>.map<int32>
/// @generic.instance source="box.map((value) => value)" id=Box<int32>
/// @generic.instance source="box.map((value) => value)" id=Box<int32>.map<int32>
/// @generic.instance source=box id=Box<int32>
/// @generic.instance source=box.map id=Box<U>
/// @generic.instance source=box.map id=Box<int32>
/// @type.symbol symbol=symbol9 source="(value) => value" type=Function<(int32,), int32>
/// @type.node source="(value) => value" type=Function<(int32,), int32>
/// @type.symbol symbol=symbol9.value source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=symbol9.value

/// @generic.instance id=Box<T> template=Box arguments=(T)
/// @generic.instance id=Box<U> template=Box arguments=(U)
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @generic.instance id=Box<int32>.map<int32> template=Box.map arguments=(int32, int32)
"#,
    );
}

#[test]
fn test_infer_closure_return_through_union() {
    let session = TestSession::single(
        r#"
declare class Box<T> {}
declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;

const value = map(1, (item) => item);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<T> {}
declare function map<T, U>(value: T, callback: (arg0: T) => U | Box<U>): U;

const value: 1 = map<1, 1>(1, (item: 1): 1 | Box<1> => item as 1 | Box<1>);

=== checked ===
declare class Box<T> {}
/// @generic.template symbol=Box parameters=(T#1)
/// @type.symbol symbol=Box source="declare class Box<T> {}" type=Box
/// @definition.class symbol=Box source="declare class Box<T> {}" template=(T#1)
/// @type.symbol symbol=Box.T source=T type=T#1

declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;
/// @generic.template symbol=map parameters=(T#2, U)
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U" type=<T#2, U>(T#2, Function<(T#2,), U | Box<U>>) => U
/// @type.symbol symbol=map.T source=T type=T#2
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=map.value#1 source="value: T" type=T#2
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=map.callback source="callback: (value: T) => U | Box<U>" type=Function<(T#2,), U | Box<U>>
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value source=value type=1
/// @type.node source="map(1, (item) => item)" type=1
/// @type.node source=map type=(1, Function<(1,), 1 | Box<1>>) => 1
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(1, Function<(1,), 1 | Box<1>>) arguments=(provided(1) as 1, provided((item) => item) as Function<(1,), 1 | Box<1>>) return=1 kind=symbol target=map instance="map<1, 1>"
/// @generic.instance source="map(1, (item) => item)" id="map<1, 1>"
/// @generic.instance source=map id=Box<1>
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol9 source="(item) => item" type=Function<(1,), 1 | Box<1>>
/// @type.node source="(item) => item" type=Function<(1,), 1 | Box<1>>
/// @generic.instance source="(item) => item" id=Box<1>
/// @type.symbol symbol=symbol9.item source=item type=1
/// @type.node source=item type=1
/// @resolution.name source=item target=symbol9.item

/// @generic.instance id="map<1, 1>" template=map arguments=(1, 1)
/// @generic.instance id=Box<1> template=Box arguments=(1)
/// @generic.instance id=Box<U> template=Box arguments=(U)
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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class Box<T> {
    map<U>(callback: (arg0: T) => U | Box<U>): U;
}

declare const box: Box<int32>;
const value: int32 = box.map<int32, int32>(
    (item: int32): int32 | Box<int32> => item as int32 | Box<int32>,
);

=== checked ===
declare class Box<T> {
/// @generic.template symbol=Box parameters=(T)
/// @type.symbol symbol=Box type=Box
/// @definition.class symbol=Box template=(T)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" slot=map type=<U>(this: Box<T>, Function<(T,), U | Box<U>>) => U
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U | Box<U>): U;
    /// @generic.template symbol=Box.map parent=template#0 parameters=(U)
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" type=<U>(this: Box<T>, Function<(T,), U | Box<U>>) => U
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=Box.map.callback source="callback: (value: T) => U | Box<U>" type=Function<(T,), U | Box<U>>
    /// @resolution.name source=T target=Box.T
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=Box target=Box
    /// @resolution.name source=U target=Box.map.U
    /// @resolution.name source=U target=Box.map.U

}

declare const box: Box<int32>;
/// @type.symbol symbol=box source=box type=Box<int32>
/// @resolution.name source=Box target=Box

const value = box.map((item) => item);
/// @type.symbol symbol=value source=value type=int32
/// @type.node source="box.map((item) => item)" type=int32
/// @type.node source=box type=Box<int32>
/// @type.node source=box.map type=<U>(this: Box<int32>, Function<(int32,), U | Box<U>>) => U
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> kind=symbol target=Box.map
/// @resolution.call source="box.map((item) => item)" parameters=(Function<(int32,), int32 | Box<int32>>) arguments=(provided((item) => item) as Function<(int32,), int32 | Box<int32>>) return=int32 kind=symbol target=Box.map receiver=Box<int32> instance=Box<int32>.map<int32>
/// @generic.instance source="box.map((item) => item)" id=Box<int32>.map<int32>
/// @generic.instance source=box id=Box<int32>
/// @generic.instance source=box.map id=Box<U>
/// @generic.instance source=box.map id=Box<int32>
/// @type.symbol symbol=symbol9 source="(item) => item" type=Function<(int32,), int32 | Box<int32>>
/// @type.node source="(item) => item" type=Function<(int32,), int32 | Box<int32>>
/// @generic.instance source="(item) => item" id=Box<int32>
/// @type.symbol symbol=symbol9.item source=item type=int32
/// @type.node source=item type=int32
/// @resolution.name source=item target=symbol9.item

/// @generic.instance id=Box<T> template=Box arguments=(T)
/// @generic.instance id=Box<U> template=Box arguments=(U)
/// @generic.instance id=Box<int32> template=Box arguments=(int32)
/// @generic.instance id=Box<int32>.map<int32> template=Box.map arguments=(int32, int32)
"#,
    );
}
