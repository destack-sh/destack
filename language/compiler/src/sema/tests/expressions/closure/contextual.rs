use crate::tests::{DirRows, TestSession};

#[test]
fn test_type_closure_parameters_from_an_aliased_contextual_signature() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Consume<T> = (value: T) => void;

class Cell<T> {
    constructor(executor: (consume: Consume<T>) => void) {
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

=== dir ===
type Consume<T> = (value: T) => void;
/// @generic.template symbol=Consume parameters=(T#1)
/// @type.symbol symbol=Consume source="type Consume<T> = (value: T) => void" type=(T#1) => void
/// @definition.type symbol=Consume source="type Consume<T> = (value: T) => void" template=(T#1) value=(T#1) => void
/// @type.symbol symbol=Consume.T source=T type=T#1
/// @type.symbol symbol=Consume.value source="value: T" type=T#1
/// @resolution.name source=T target=Consume.T

class Cell<T> {
/// @generic.template symbol=Cell parameters=(T#2)
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell template=(T#2)
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell<T#2>, ((T#2) => void) => void) => Cell<T#2>
/// @type.symbol symbol=Cell.T source=T type=T#2

    constructor(executor: (consume: Consume<T>) => void) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell<T#2>, ((T#2) => void) => void) => Cell<T#2>
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell<T#2>
    /// @type.symbol symbol=Cell.constructor.executor source="executor: (consume: Consume<T>) => void" type=((T#2) => void) => void
    /// @type.symbol symbol=Cell.constructor.consume source="consume: Consume<T>" type=(T#2) => void
    /// @resolution.name source=Consume target=Consume
    /// @resolution.name source=T target=Cell.T

        executor;
        /// @type.node source=executor type=((T#2) => void) => void
        /// @resolution.name source=executor target=Cell.constructor.executor
        /// @resolution.place source=executor placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=executor root=Cell.constructor.executor

    }
}

function capture<T>(): void {
/// @generic.template symbol=capture parameters=(T#3)
/// @type.symbol symbol=capture type=<T#3>() => void
/// @type.symbol symbol=capture.T source=T type=T#3

    let seen: Consume<T> | undefined = undefined;
    /// @type.symbol symbol=capture.seen source=seen type=(T#3) => void | undefined
    /// @resolution.pattern source=seen kind=binding target=capture.seen
    /// @resolution.name source=Consume target=Consume
    /// @resolution.name source=T target=capture.T
    /// @type.node source=undefined type=undefined

    let cell = new Cell<T>((inner) => {
    /// @type.symbol symbol=capture.cell source=cell type=Cell<T#3>
    /// @resolution.pattern source=cell kind=binding target=capture.cell
    /// @generic.instance id=Cell<T#3> template=Cell arguments=(T#3)
    /// @type.node type=Cell<T#3>
    /// @resolution.construct parameters=((Consume<T#3>) => void) arguments=(provided(argument) as (Consume<T#3>) => void) return=Cell<T#3> kind=class target=Cell constructor=Cell.constructor instance=Cell<T#3>
    /// @generic.instantiation id=Cell.constructor<T#3> template=Cell.constructor arguments=(T#3) owner=capture
    /// @generic.instantiation id=Cell<T#3> template=Cell arguments=(T#3) owner=capture
    /// @generic.instance id=Cell.constructor<T#3> template=Cell.constructor arguments=(T#3)
    /// @type.node source=Cell type=typeof Cell
    /// @resolution.name source=Cell target=Cell
    /// @resolution.name source=T target=capture.T
    /// @type.symbol symbol=capture.symbol13 type=((T#3) => void) => void
    /// @type.node type=((T#3) => void) => void
    /// @type.symbol symbol=capture.symbol13.inner source=inner type=(T#3) => void

        seen = inner;
        /// @type.node source="seen = inner" type=(T#3) => void
        /// @type.node source=seen type=(T#3) => void | undefined
        /// @resolution.name source=seen target=capture.seen
        /// @resolution.pattern.assign source=seen kind=place
        /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=seen root=capture.seen
        /// @resolution.assignment source=seen write=binding(capture.seen) type=Consume<T#3> | undefined
        /// @type.node source=inner type=(T#3) => void
        /// @resolution.name source=inner target=capture.symbol13.inner
        /// @resolution.place source=inner placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=inner root=capture.symbol13.inner

    });
    cell;
    /// @type.node source=cell type=Cell<T#3>
    /// @resolution.name source=cell target=capture.cell
    /// @resolution.place source=cell placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=cell root=capture.cell

    seen;
    /// @type.node source=seen type=(T#3) => void | undefined
    /// @resolution.name source=seen target=capture.seen
    /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=seen root=capture.seen

}
"#,
    );
}

#[test]
fn test_type_closure_parameters_from_the_contextual_signature() {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Cell<T> {
    constructor(executor: (value: T) => void) {
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

=== dir ===
class Cell<T> {
/// @generic.template symbol=Cell parameters=(T)
/// @type.symbol symbol=Cell type=typeof Cell
/// @definition.class symbol=Cell template=(T)
/// @definition.method symbol=Cell.constructor slot=constructor role=constructor type=(this: &'managed Cell<T>, (T) => void) => Cell<T>
/// @type.symbol symbol=Cell.T source=T type=T

    constructor(executor: (value: T) => void) {
    /// @type.symbol symbol=Cell.constructor type=(this: &'managed Cell<T>, (T) => void) => Cell<T>
    /// @type.symbol symbol=Cell.constructor.this type=&'managed Cell<T>
    /// @type.symbol symbol=Cell.constructor.executor source="executor: (value: T) => void" type=(T) => void
    /// @type.symbol symbol=Cell.constructor.value source="value: T" type=T
    /// @resolution.name source=T target=Cell.T

        executor;
        /// @type.node source=executor type=(T) => void
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
    /// @generic.instance id=Cell<int32> template=Cell arguments=(int32)
    /// @type.node type=Cell<int32>
    /// @resolution.construct parameters=((int32) => void) arguments=(provided(argument) as (int32) => void) return=Cell<int32> kind=class target=Cell constructor=Cell.constructor instance=Cell<int32>
    /// @generic.instantiation id=Cell.constructor<int32> template=Cell.constructor arguments=(int32)
    /// @generic.instantiation id=Cell<int32> template=Cell arguments=(int32)
    /// @generic.instance id=Cell.constructor<int32> template=Cell.constructor arguments=(int32)
    /// @type.node source=Cell type=typeof Cell
    /// @resolution.name source=Cell target=Cell
    /// @type.symbol symbol=capture.symbol9 type=(int32) => void
    /// @type.node type=(int32) => void
    /// @type.symbol symbol=capture.symbol9.inner source=inner type=int32

        seen = inner;
        /// @type.node source="seen = inner" type=int32
        /// @type.node source=seen type=int32 | undefined
        /// @resolution.name source=seen target=capture.seen
        /// @resolution.pattern.assign source=seen kind=place
        /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
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

    seen;
    /// @type.node source=seen type=int32 | undefined
    /// @resolution.name source=seen target=capture.seen
    /// @resolution.place source=seen placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=seen root=capture.seen

}
"#,
    );
}

#[test]
fn test_commit_the_inferred_result_of_a_contextual_lambda() {
    let session = TestSession::single(
        r#"
const callback: (value: int32) => int32 | undefined = (value) => value + 1;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
const callback: (value: int32) => int32 | undefined = (value: int32): int32 | undefined =>
    (value + 1) as int32 | undefined;

=== dir ===
const callback: (value: int32) => int32 | undefined = (value) => value + 1;
/// @type.symbol symbol=callback source=callback type=(int32) => int32 | undefined
/// @resolution.pattern source=callback kind=binding target=callback
/// @type.symbol symbol=value source="value: int32" type=int32
/// @type.symbol symbol=symbol1 source="(value) => value + 1" type=Function<(int32,), int32 | undefined, "readonly">
/// @type.symbol symbol=symbol1.value source=value type=int32
/// @resolution.name source=value target=symbol1.value
/// @resolution.operator source="value + 1" type=int32 operator="+" kind=builtin operands=[value as int32 families=(integer), 1 as int32 families=(integer)]
/// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=value root=symbol1.value
"#,
        r#"
"#,
    );
}

/// Type closure parameters from the element type a sibling argument infers.
#[test]
fn test_type_closure_parameters_from_a_generic_call_argument() {
    let session = TestSession::single(
        r#"
declare function map<T, U>(values: T[], callback: (value: T) => U): U[];

declare const counts: int32[];

const labels = map(counts, (count) => count > 0);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function map<T, U>(values: T[], callback: (value: T) => U): U[];

declare const counts: int32[];

const labels: boolean[] = map<int32, boolean>(counts, (count: int32): boolean => count > 0);

=== dir ===
declare function map<T, U>(values: T[], callback: (value: T) => U): U[];
/// @generic.template symbol=map parameters=(T, U)
/// @type.symbol symbol=map source="declare function map<T, U>(values: T[], callback: (value: T) => U): U[]" type=<T, U>(T[], (T) => U) => U[]
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.U source=U type=U
/// @resolution.name source=T target=map.T
/// @type.symbol symbol=map.value source="value: T" type=T
/// @resolution.name source=T target=map.T
/// @resolution.name source=U target=map.U
/// @resolution.name source=U target=map.U

declare const counts: int32[];
/// @type.symbol symbol=counts source=counts type=int32[]
/// @resolution.pattern source=counts kind=binding target=counts

const labels = map(counts, (count) => count > 0);
/// @type.symbol symbol=labels source=labels type=boolean[]
/// @resolution.pattern source=labels kind=binding target=labels
/// @type.node source="map(counts, (count) => count > 0)" type=boolean[]
/// @type.node source=map type=(int32[], (int32) => boolean) => boolean[]
/// @resolution.name source=map target=map
/// @resolution.call source="map(counts, (count) => count > 0)" parameters=(int32[], (int32) => boolean) arguments=(provided(counts) as int32[], provided((count) => count > 0) as (int32) => boolean) return=boolean[] kind=symbol target=map instance="map<int32, boolean>"
/// @generic.instantiation id="map<int32, boolean>" template=map arguments=(int32, boolean)
/// @type.node source=counts type=int32[]
/// @resolution.name source=counts target=counts
/// @resolution.place source=counts placement="local" lifetime="static" access="immutable"
/// @resolution.access source=counts root=counts
/// @type.symbol symbol=symbol8 source="(count) => count > 0" type=Function<(int32,), boolean, "readonly">
/// @type.node source="(count) => count > 0" type=Function<(int32,), boolean, "readonly">
/// @type.symbol symbol=symbol8.count source=count type=int32
/// @type.node source="count > 0" type=boolean
/// @type.node source=count type=int32
/// @resolution.name source=count target=symbol8.count
/// @resolution.operator source="count > 0" type=boolean operator=">" kind=builtin operands=[count as int32 families=(integer), 0 as int32 families=(integer)]
/// @resolution.place source=count placement="local" lifetime="frame" access="exclusive"
/// @resolution.access source=count root=symbol8.count
/// @type.node source=0 type=0
"#,
        r#"

"#,
    );
}

/// Reject a closure that takes more parameters than its contextual signature.
#[test]
fn test_reject_a_closure_with_more_parameters_than_the_contextual_signature() {
    let session = TestSession::single(
        r#"
declare function run(callback: (value: int32) => void): void;

run((value, extra) => {
    value;
    extra;
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function run(callback: (value: int32) => void): void;

run((value, extra): void => {
    value;
    extra;
});

=== dir ===
declare function run(callback: (value: int32) => void): void;
/// @type.symbol symbol=run source="declare function run(callback: (value: int32) => void): void" type=((int32) => void) => void
/// @type.symbol symbol=run.value source="value: int32" type=int32

run((value, extra) => {
/// @type.node source=run type=((int32) => void) => void
/// @type.node type=void
/// @resolution.name source=run target=run
/// @resolution.call parameters=((int32) => void) arguments=(provided(argument) as (int32) => void) return=void kind=symbol target=run
/// @type.symbol symbol=symbol4 type=Function<(<error>, <error>), void, "readonly">
/// @type.node type=Function<(<error>, <error>), void, "readonly">
/// @type.symbol symbol=symbol4.value source=value type=<error>
/// @type.symbol symbol=symbol4.extra source=extra type=<error>

    value;
    /// @type.node source=value type=<error>
    /// @resolution.name source=value target=symbol4.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol4.value

    extra;
    /// @type.node source=extra type=<error>
    /// @resolution.name source=extra target=symbol4.extra
    /// @resolution.place source=extra placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=extra root=symbol4.extra

});
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Function<(value: _, extra: _), void, \"readonly\">' is not assignable to parameter of type '(value: int32) => void'"
/// @diagnostic.label line=4 column=5 span="(value, extra) => {\n    value;\n    extra;\n}" line_source="run((value, extra) => {"
/// @diagnostic.related line=4 column=1 span="run((value, extra) => {\n    value;\n    extra;\n})" line_source="run((value, extra) => {" message="in this call"
"#,
    );
}

/// Keep a written closure parameter type over the contextual one and reject a mismatch.
#[test]
fn test_keep_a_written_closure_parameter_type_over_the_contextual_one() {
    let session = TestSession::single(
        r#"
declare function run(callback: (value: int32 | string) => void): void;

run((value: int32 | string) => {
    value;
});
run((value: boolean) => {
    value;
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function run(callback: (value: int32 | string) => void): void;

run((value: int32 | string): void => {
    value;
});
run((value: boolean): void => {
    value;
});

=== dir ===
declare function run(callback: (value: int32 | string) => void): void;
/// @type.symbol symbol=run source="declare function run(callback: (value: int32 | string) => void): void" type=((int32 | string) => void) => void
/// @type.symbol symbol=run.value source="value: int32 | string" type=int32 | string

run((value: int32 | string) => {
/// @type.node source=run type=((int32 | string) => void) => void
/// @type.node type=void
/// @resolution.name source=run target=run
/// @resolution.call parameters=((int32 | string) => void) arguments=(provided(argument) as (int32 | string) => void) return=void kind=symbol target=run
/// @type.symbol symbol=symbol4 type=Function<(int32 | string,), void, "readonly">
/// @type.node type=Function<(int32 | string,), void, "readonly">
/// @type.symbol symbol=symbol4.value source="value: int32 | string" type=int32 | string

    value;
    /// @type.node source=value type=int32 | string
    /// @resolution.name source=value target=symbol4.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol4.value

});
run((value: boolean) => {
/// @type.node source=run type=((int32 | string) => void) => void
/// @type.node type=void
/// @resolution.name source=run target=run
/// @resolution.call parameters=((int32 | string) => void) arguments=(provided(argument) as (int32 | string) => void) return=void kind=symbol target=run
/// @type.symbol symbol=symbol6 type=Function<(boolean,), void, "readonly">
/// @type.node type=Function<(boolean,), void, "readonly">
/// @type.symbol symbol=symbol6.value source="value: boolean" type=boolean

    value;
    /// @type.node source=value type=boolean
    /// @resolution.name source=value target=symbol6.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol6.value

});
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type 'Function<(value: boolean,), void, \"readonly\">' is not assignable to parameter of type '(value: int32 | string) => void'"
/// @diagnostic.label line=7 column=5 span="(value: boolean) => {\n    value;\n}" line_source="run((value: boolean) => {"
/// @diagnostic.related line=7 column=1 span="run((value: boolean) => {\n    value;\n})" line_source="run((value: boolean) => {" message="in this call"
"#,
    );
}

/// Union the result of a closure that returns from more than one branch.
#[test]
fn test_union_the_return_statements_of_a_closure() {
    let session = TestSession::single(
        r#"
declare const flag: boolean;

const choose = () => {
    if (flag) {
        return "text";
    }

    return 1;
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare const flag: boolean;

const choose: () => string | int64 = (): string | int64 => {
    if (flag) {
        return "text" as string | int64;
    }

    return 1 as string | int64;
};

=== dir ===
declare const flag: boolean;
/// @type.symbol symbol=flag source=flag type=boolean
/// @resolution.pattern source=flag kind=binding target=flag

const choose = () => {
/// @type.symbol symbol=choose source=choose type=Function<(), string | int64, "readonly">
/// @resolution.pattern source=choose kind=binding target=choose
/// @type.symbol symbol=symbol2 type=Function<(), string | int64, "readonly">
/// @type.node type=Function<(), string | int64, "readonly">

    if (flag) {
    /// @type.node source=flag type=boolean
    /// @resolution.name source=flag target=flag
    /// @resolution.place source=flag placement="local" lifetime="static" access="immutable"
    /// @resolution.access source=flag root=flag

        return "text";
        /// @type.node source="\"text\"" type="text"

    }

    return 1;
    /// @type.node source=1 type=1

};
"#,
        r#"

"#,
    );
}

/// Take the contextual signature through an optional callback type.
#[test]
fn test_take_the_contextual_signature_through_an_optional_callback_type() {
    let session = TestSession::single(
        r#"
type Handler = ((value: int32) => void) | undefined;

const handler: Handler = (value) => {
    value;
};
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Handler = ((value: int32) => void) | undefined;

const handler: Handler = ((value: int32): void => {
    value;
}) as Handler;

=== dir ===
type Handler = ((value: int32) => void) | undefined;
/// @type.symbol symbol=Handler source="type Handler = ((value: int32) => void) | undefined" type=(int32) => void | undefined
/// @definition.type symbol=Handler source="type Handler = ((value: int32) => void) | undefined" value=(int32) => void | undefined
/// @type.symbol symbol=Handler.value source="value: int32" type=int32

const handler: Handler = (value) => {
/// @type.symbol symbol=handler source=handler type=Handler
/// @resolution.pattern source=handler kind=binding target=handler
/// @resolution.name source=Handler target=Handler
/// @type.symbol symbol=symbol3 type=Function<(int32,), void, "readonly">
/// @type.node type=Function<(int32,), void, "readonly">
/// @type.symbol symbol=symbol3.value source=value type=int32

    value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=symbol3.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol3.value

};
"#,
        r#"

"#,
    );
}

/// Type a closure parameter from a type variable an earlier argument already fixed.
#[test]
fn test_type_a_closure_parameter_from_an_earlier_fixed_argument() {
    let session = TestSession::single(
        r#"
declare function withValue<T>(value: T, callback: (value: T) => void): void;

withValue("ready", (value) => {
    value;
});
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function withValue<T>(value: T, callback: (value: T) => void): void;

withValue<string>("ready", (value: string): void => {
    value;
});

=== dir ===
declare function withValue<T>(value: T, callback: (value: T) => void): void;
/// @generic.template symbol=withValue parameters=(T)
/// @type.symbol symbol=withValue source="declare function withValue<T>(value: T, callback: (value: T) => void): void" type=<T>(T, (T) => void) => void
/// @type.symbol symbol=withValue.T source=T type=T
/// @resolution.name source=T target=withValue.T
/// @type.symbol symbol=withValue.value#2 source="value: T" type=T
/// @resolution.name source=T target=withValue.T

withValue("ready", (value) => {
/// @type.node source=withValue type=(string, (string) => void) => void
/// @type.node type=void
/// @resolution.name source=withValue target=withValue
/// @resolution.call parameters=(string, (string) => void) arguments=(provided("ready") as string, provided(argument) as (string) => void) return=void kind=symbol target=withValue instance=withValue<string>
/// @generic.instantiation id=withValue<string> template=withValue arguments=(string)
/// @type.node source="\"ready\"" type="ready"
/// @type.symbol symbol=symbol6 type=Function<(string,), void, "readonly">
/// @type.node type=Function<(string,), void, "readonly">
/// @type.symbol symbol=symbol6.value source=value type=string

    value;
    /// @type.node source=value type=string
    /// @resolution.name source=value target=symbol6.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=symbol6.value

});
"#,
        r#"

"#,
    );
}
