use crate::tests::{DirRows, TestSession};

#[test]
fn test_block_lambda_completion_keeps_void_return() {
    let session = TestSession::single(
        r#"
class Box<T> {}
declare function use<T>(callback: () => T | Box<T>): T;

const value = use(() => {});
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
class Box<T> {}
/// @generic.template source=declaration parameters=[T#1]
/// @type.symbol symbol=Box source="class Box<T> {}" type=Box<T#1>
/// @definition.class symbol=Box source="class Box<T> {}" template=LocalGenericTemplateId(0)
/// @type.symbol symbol=Box.T source=T type=T#1

declare function use<T>(callback: () => T | Box<T>): T;
/// @generic.template source=declaration parameters=[T#2]
/// @type.symbol symbol=use source="declare function use<T>(callback: () => T | Box<T>): T" type=<T#2>(() => T#2 | Box<T#2>) => T#2
/// @type.symbol symbol=use.T source=T type=T#2
/// @type.symbol symbol=callback source="callback: () => T | Box<T>" type=() => T#2 | Box<T#2>
/// @resolution.name source=T target=use.T
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=use.T
/// @resolution.name source=T target=use.T

const value = use(() => {});
/// @type.symbol symbol=value source=value type=void
/// @generic.instance source="use(() => {})" id=use<void>
/// @type.node source="use(() => {})" type=void
/// @type.node source=use type=<T#2>(() => T#2 | Box<T#2>) => T#2
/// @resolution.name source=use target=use
/// @resolution.call source="use(() => {})" parameters=(() => void | Box<void>) return=void kind=symbol target=use instance=use<void>
/// @type.symbol symbol=symbol6 source="() => {}" type=() => void
/// @type.node source="() => {}" type=() => void

/// @generic.instance id=use<void> template=use arguments=[void]
"#,
    );
}

#[test]
fn test_infer_lambda_return_from_parameter() {
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
declare function map<T, U>(value: T, callback: (value: T) => U): U;
/// @generic.template source=declaration parameters=[T, U]
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U): U" type=<T, U>(T, (T) => U) => U
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=value#1 source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=callback source="callback: (value: T) => U" type=(T) => U
/// @type.symbol symbol=value#2 source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value#3 source=value type=1
/// @generic.instance source="map(1, (item) => item)" id="map<1, 1>"
/// @type.node source="map(1, (item) => item)" type=1
/// @type.node source=map type=<T, U>(T, (T) => U) => U
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(1, (1) => 1) return=1 kind=symbol target=map instance="map<1, 1>"
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol7 source="(item) => item" type=(1) => 1
/// @type.node source="(item) => item" type=(1) => 1
/// @type.symbol symbol=item source=item type=1
/// @type.node source=item type=1
/// @resolution.name source=item target=item

/// @generic.instance id="map<1, 1>" template=map arguments=[1, 1]
"#,
    );
}

#[test]
fn test_infer_method_lambda_return_from_receiver() {
    let session = TestSession::single(
        r#"
class Box<T> {
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
class Box<T> {
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Box type=Box<T>
/// @definition.class symbol=Box template=LocalGenericTemplateId(0)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" slot=map type=<U>(this: Box<T>, (T) => U) => Box<U>
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U): Box<U>;
    /// @generic.template source=member parent=template#0 parameters=[U]
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U): Box<U>" type=<U>(this: Box<T>, (T) => U) => Box<U>
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=callback source="callback: (value: T) => U" type=(T) => U
    /// @type.symbol symbol=value#1 source="value: T" type=T
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
/// @generic.instance source="box.map((value) => value)" id=member<int32>
/// @generic.instance source=box.map id=member<int32>
/// @type.node source="box.map((value) => value)" type=Box<int32>
/// @type.node source=box type=Box<int32>
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> kind=symbol target=Box.map instance=member<int32>
/// @resolution.call source="box.map((value) => value)" parameters=((int32) => int32) return=Box<int32> kind=symbol target=Box.map receiver=Box<int32> instance=member<int32>
/// @type.symbol symbol=symbol9 source="(value) => value" type=(int32) => int32
/// @type.node source="(value) => value" type=(int32) => int32
/// @type.symbol symbol=value#2 source=value type=int32
/// @type.node source=value type=int32
/// @resolution.name source=value target=value#2

/// @generic.instance id=member<int32> template=member arguments=[int32]
"#,
    );
}

#[test]
fn test_infer_lambda_return_through_union() {
    let session = TestSession::single(
        r#"
class Box<T> {}
declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;

const value = map(1, (item) => item);
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
class Box<T> {}
/// @generic.template source=declaration parameters=[T#1]
/// @type.symbol symbol=Box source="class Box<T> {}" type=Box<T#1>
/// @definition.class symbol=Box source="class Box<T> {}" template=LocalGenericTemplateId(0)
/// @type.symbol symbol=Box.T source=T type=T#1

declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U;
/// @generic.template source=declaration parameters=[T#2, U]
/// @type.symbol symbol=map source="declare function map<T, U>(value: T, callback: (value: T) => U | Box<U>): U" type=<T#2, U>(T#2, (T#2) => U | Box<U>) => U
/// @type.symbol symbol=map.T source=T type=T#2
/// @type.symbol symbol=map.U source=U type=U
/// @type.symbol symbol=value#1 source="value: T" type=T#2
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=callback source="callback: (value: T) => U | Box<U>" type=(T#2) => U | Box<U>
/// @type.symbol symbol=value#2 source="value: T" type=T#2
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=Box target=Box
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

const value = map(1, (item) => item);
/// @type.symbol symbol=value#3 source=value type=1
/// @generic.instance source="map(1, (item) => item)" id="map<1, 1>"
/// @type.node source="map(1, (item) => item)" type=1
/// @type.node source=map type=<T#2, U>(T#2, (T#2) => U | Box<U>) => U
/// @resolution.name source=map target=map
/// @resolution.call source="map(1, (item) => item)" parameters=(1, (1) => 1 | Box<1>) return=1 kind=symbol target=map instance="map<1, 1>"
/// @type.node source=1 type=1
/// @type.symbol symbol=symbol9 source="(item) => item" type=(1) => 1
/// @type.node source="(item) => item" type=(1) => 1
/// @type.symbol symbol=item source=item type=1
/// @type.node source=item type=1
/// @resolution.name source=item target=item

/// @generic.instance id="map<1, 1>" template=map arguments=[1, 1]
"#,
    );
}

#[test]
fn test_infer_method_lambda_return_through_union() {
    let session = TestSession::single(
        r#"
class Box<T> {
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
class Box<T> {
/// @generic.template source=declaration parameters=[T]
/// @type.symbol symbol=Box type=Box<T>
/// @definition.class symbol=Box template=LocalGenericTemplateId(0)
/// @definition.method symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" slot=map type=<U>(this: Box<T>, (T) => U | Box<U>) => U
/// @type.symbol symbol=Box.T source=T type=T

    map<U>(callback: (value: T) => U | Box<U>): U;
    /// @generic.template source=member parent=template#0 parameters=[U]
    /// @type.symbol symbol=Box.map source="map<U>(callback: (value: T) => U | Box<U>): U" type=<U>(this: Box<T>, (T) => U | Box<U>) => U
    /// @type.symbol symbol=Box.map.U source=U type=U
    /// @type.symbol symbol=callback source="callback: (value: T) => U | Box<U>" type=(T) => U | Box<U>
    /// @type.symbol symbol=value#1 source="value: T" type=T
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
/// @type.symbol symbol=value#2 source=value type=int32
/// @generic.instance source="box.map((item) => item)" id=member<int32>
/// @generic.instance source=box.map id=member<int32>
/// @type.node source="box.map((item) => item)" type=int32
/// @type.node source=box type=Box<int32>
/// @resolution.name source=box target=box
/// @resolution.member source=box.map receiver=Box<int32> kind=symbol target=Box.map instance=member<int32>
/// @resolution.call source="box.map((item) => item)" parameters=((int32) => int32 | Box<int32>) return=int32 kind=symbol target=Box.map receiver=Box<int32> instance=member<int32>
/// @type.symbol symbol=symbol9 source="(item) => item" type=(int32) => int32
/// @type.node source="(item) => item" type=(int32) => int32
/// @type.symbol symbol=item source=item type=int32
/// @type.node source=item type=int32
/// @resolution.name source=item target=item

/// @generic.instance id=member<int32> template=member arguments=[int32]
"#,
    );
}
