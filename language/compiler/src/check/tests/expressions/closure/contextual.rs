use crate::tests::{DirRows, TestSession};

#[test]
fn test_closure_parameters_type_from_an_aliased_contextual_signature() {
    let session = TestSession::single(
        r#"
type Consume<T> = (value: T) => void;

class Cell<T> {
    constructor(executor: (consume: Consume<T>) => void) {
        executor;
    }
}

function capture<T>(): void {
    let seen: Consume<T> | undefined = undefined;
    let cell = new Cell<T>((inner) => {
        seen = inner;
    });
    cell;
    seen;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Consume<T> = (value: T) => void;

class Cell<T> {
    constructor(executor: (arg0: Consume<T>) => void): this {
        executor;
    }
}

function capture<T>(): void {
    let seen: Consume<T> | undefined = undefined as Consume<T> | undefined;
    let cell: Cell<T> = new Cell<T>((inner: Consume<T>): void => {
        seen = inner as Consume<T> | undefined;
    });
    cell;
    seen;
}

=== checked ===
type Consume<T> = (value: T) => void;
/// @generic.template symbol=Consume parameters=(T#1)
/// @type.symbol symbol=Consume source="type Consume<T> = (value: T) => void" type=Function<(T#1,), void>
/// @definition.type symbol=Consume source="type Consume<T> = (value: T) => void" template=(T#1) value=Function<(T#1,), void>
/// @type.symbol symbol=Consume.T source=T type=T#1
/// @type.symbol symbol=Consume.value source="value: T" type=T#1
/// @resolution.name source=T target=Consume.T

class Cell<T> {
/// @generic.template symbol=Cell parameters=(T#2)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(T#2)
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(Function<(Consume<T#2>,), void>) => this
/// @type.symbol symbol=Cell.T source=T type=T#2

    constructor(executor: (consume: Consume<T>) => void) {
    /// @type.symbol symbol=Cell.constructor type=(Function<(Consume<T#2>,), void>) => this
    /// @type.symbol symbol=Cell.constructor.executor source="executor: (consume: Consume<T>) => void" type=Function<(Consume<T#2>,), void>
    /// @type.symbol symbol=Cell.constructor.consume source="consume: Consume<T>" type=Consume<T#2>
    /// @resolution.name source=Consume target=Consume
    /// @resolution.name source=T target=Cell.T

        executor;
        /// @type.node source=executor type=Function<(Consume<T#2>,), void>
        /// @resolution.name source=executor target=Cell.constructor.executor
        /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=executor root=Cell.constructor.executor
        /// @generic.instance source=executor id=Consume<T#2>

    }
}

function capture<T>(): void {
/// @generic.template symbol=capture parameters=(T#3)
/// @type.symbol symbol=capture type=<T#3>() => void
/// @type.symbol symbol=capture.T source=T type=T#3

    let seen: Consume<T> | undefined = undefined;
    /// @type.symbol symbol=capture.seen source=seen type=Consume<T#3> | undefined
    /// @resolution.pattern source=seen kind=binding target=capture.seen
    /// @resolution.name source=Consume target=Consume
    /// @resolution.name source=T target=capture.T
    /// @type.node source=undefined type=undefined

    let cell = new Cell<T>((inner) => {
    /// @type.symbol symbol=capture.cell source=cell type=Cell<T#3>
    /// @resolution.pattern source=cell kind=binding target=capture.cell
    /// @type.node type=Cell<T#3>
    /// @resolution.construct parameters=(Function<(Consume<T#3>,), void>) arguments=(provided(argument) as Function<(Consume<T#3>,), void>) return=Cell<T#3> kind=class target=Cell constructor=Cell.constructor instance=Cell<T#3>
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=T target=capture.T
    /// @type.symbol symbol=capture.symbol13 type=Function<(Consume<T#3>,), void>
    /// @type.node type=Function<(Consume<T#3>,), void>
    /// @type.symbol symbol=capture.symbol13.inner source=inner type=Consume<T#3>

        seen = inner;
        /// @type.node source="seen = inner" type=Consume<T#3>
        /// @type.node source=seen type=Consume<T#3> | undefined
        /// @resolution.name source=seen target=capture.seen
        /// @resolution.pattern.assign source=seen kind=place
        /// @resolution.access source=seen root=capture.seen
        /// @resolution.assignment source=seen write=binding(capture.seen) type=Consume<T#3> | undefined
        /// @generic.instance source="seen = inner" id=Consume<T#3>
        /// @generic.instance source=seen id=Consume<T#3>
        /// @type.node source=inner type=Consume<T#3>
        /// @resolution.name source=inner target=capture.symbol13.inner
        /// @resolution.place source=inner placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=inner root=capture.symbol13.inner
        /// @generic.instance source=inner id=Consume<T#3>

    });
    cell;
    /// @type.node source=cell type=Cell<T#3>
    /// @resolution.name source=cell target=capture.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=cell root=capture.cell
    /// @generic.instance source=cell id=Cell<T#3>

    seen;
    /// @type.node source=seen type=Consume<T#3> | undefined
    /// @resolution.name source=seen target=capture.seen
    /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=seen root=capture.seen
    /// @generic.instance source=seen id=Consume<T#3>

}

/// @generic.instance id=Cell<T#3> template=Cell arguments=(T#3)
/// @generic.instance id=Consume<T#2> template=Consume arguments=(T#2)
/// @generic.instance id=Consume<T#3> template=Consume arguments=(T#3)
"#,
    );
}

#[test]
fn test_closure_parameters_type_from_the_contextual_signature() {
    let session = TestSession::single(
        r#"
class Cell<T> {
    constructor(executor: (value: T) => void) {
        executor;
    }
}

function capture(): void {
    let seen: int32 | undefined = undefined;
    let cell = new Cell<int32>((inner) => {
        seen = inner;
    });
    cell;
    seen;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Cell<out T> {
    constructor(executor: (arg0: T) => void): this {
        executor;
    }
}

function capture(): void {
    let seen: int32 | undefined = undefined as int32 | undefined;
    let cell: Cell<int32> = new Cell<int32>((inner: int32): void => {
        seen = inner as int32 | undefined;
    });
    cell;
    seen;
}

=== checked ===
class Cell<T> {
/// @generic.template symbol=Cell parameters=(out T)
/// @type.symbol symbol=Cell type=Cell
/// @definition.class symbol=Cell template=(out T)
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(Function<(T,), void>) => this
/// @type.symbol symbol=Cell.T source=T type=T

    constructor(executor: (value: T) => void) {
    /// @type.symbol symbol=Cell.constructor type=(Function<(T,), void>) => this
    /// @type.symbol symbol=Cell.constructor.executor source="executor: (value: T) => void" type=Function<(T,), void>
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        executor;
        /// @type.node source=executor type=Function<(T,), void>
        /// @resolution.name source=executor target=Cell.constructor.executor
        /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=executor root=Cell.constructor.executor

    }
}

function capture(): void {
/// @type.symbol symbol=capture type=() => void

    let seen: int32 | undefined = undefined;
    /// @type.symbol symbol=capture.seen source=seen type=int32 | undefined
    /// @resolution.pattern source=seen kind=binding target=capture.seen
    /// @type.node source=undefined type=undefined

    let cell = new Cell<int32>((inner) => {
    /// @type.symbol symbol=capture.cell source=cell type=Cell<int32>
    /// @resolution.pattern source=cell kind=binding target=capture.cell
    /// @type.node type=Cell<int32>
    /// @resolution.construct parameters=(Function<(int32,), void>) arguments=(provided(argument) as Function<(int32,), void>) return=Cell<int32> kind=class target=Cell constructor=Cell.constructor instance=Cell<int32>
    /// @resolution.name source=Cell target=Cell
    /// @type.symbol symbol=capture.symbol9 type=Function<(int32,), void>
    /// @type.node type=Function<(int32,), void>
    /// @type.symbol symbol=capture.symbol9.inner source=inner type=int32

        seen = inner;
        /// @type.node source="seen = inner" type=int32
        /// @type.node source=seen type=int32 | undefined
        /// @resolution.name source=seen target=capture.seen
        /// @resolution.pattern.assign source=seen kind=place
        /// @resolution.access source=seen root=capture.seen
        /// @resolution.assignment source=seen write=binding(capture.seen) type=int32 | undefined
        /// @type.node source=inner type=int32
        /// @resolution.name source=inner target=capture.symbol9.inner
        /// @resolution.place source=inner placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=inner root=capture.symbol9.inner

    });
    cell;
    /// @type.node source=cell type=Cell<int32>
    /// @resolution.name source=cell target=capture.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=cell root=capture.cell
    /// @generic.instance source=cell id=Cell<int32>

    seen;
    /// @type.node source=seen type=int32 | undefined
    /// @resolution.name source=seen target=capture.seen
    /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=seen root=capture.seen

}

/// @generic.instance id=Cell<int32> template=Cell arguments=(int32)
"#,
    );
}
