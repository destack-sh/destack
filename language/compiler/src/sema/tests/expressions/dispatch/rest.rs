use crate::tests::{DirRows, TestSession};

/// Fixed-array rest annotations report a declaration error.
#[test]
fn test_declare_fixed_array_rest_parameter() {
    let session = TestSession::single(
        r#"
declare function collect(...values: [int32; 2]): void;
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function collect(...values: [int32; 2]): void;

=== dir ===
declare function collect(...values: [int32; 2]): void;
/// @type.symbol symbol=collect source="declare function collect(...values: [int32; 2]): void" type=(...FixedArray<int32, 2>) => void
"#, r#"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type FixedArray<int32, 2> must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=2 column=29 span="values" line_source="declare function collect(...values: [int32; 2]): void;"
"#);
}

/// A borrowed fixed-array alias is invalid as a rest parameter.
#[test]
fn test_call_fixed_array_rest_parameter() {
    let session = TestSession::single(
        r#"
type Pair = [int32; 2];

declare function collect(...values: &readonly Pair): void;

collect(1, 2);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
type Pair = [int32; 2];

declare function collect<'a>(...values: &readonly Pair): void;

collect(1, 2);

=== dir ===
type Pair = [int32; 2];
/// @type.symbol symbol=Pair source="type Pair = [int32; 2]" type=FixedArray<int32, 2>
/// @definition.type symbol=Pair source="type Pair = [int32; 2]" value=FixedArray<int32, 2>

declare function collect(...values: &readonly Pair): void;
/// @generic.template symbol=collect parameters=('a)
/// @type.symbol symbol=collect source="declare function collect(...values: &readonly Pair): void" type=<collect.'a>(...&collect.'a readonly Pair) => void
/// @resolution.name source=Pair target=Pair

collect(1, 2);
/// @resolution.name source=collect target=collect
/// @resolution.rejected source="collect(1, 2)"
"#,
        r#"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type &'a readonly Pair must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=4 column=29 span="values" line_source="declare function collect(...values: &readonly Pair): void;"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('1', '2')"
/// @diagnostic.label line=6 column=1 span="collect(1, 2)" line_source="collect(1, 2);"
/// @diagnostic.note message="the candidate '<'a>(values: ...&'a readonly Pair) => void' does not apply"
"#,
    );
}

/// A tuple's variadic tail requires a supported rest collection.
#[test]
fn test_declare_fixed_array_tuple_rest() {
    let session = TestSession::single(
        r#"
declare function collect(...values: (string, ...[int32; 2])): void;
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function collect(...values: (string, ...[int32; 2])): void;

=== dir ===
declare function collect(...values: (string, ...[int32; 2])): void;
/// @type.symbol symbol=collect source="declare function collect(...values: (string, ...[int32; 2])): void" type=(string, ...FixedArray<int32, 2>) => void
"#,
        r#"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type (string, ...FixedArray<int32, 2>) must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=2 column=29 span="values" line_source="declare function collect(...values: (string, ...[int32; 2])): void;"
"#,
    );
}

/// A tuple rest preserves its fixed prefix and variadic tail.
#[test]
fn test_call_variadic_tuple_parameter() {
    let session = TestSession::single(
        r#"
declare function collect(...values: (string, ...int32[])): void;

collect("Ada", 1, 2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
declare function collect(...values: (string, ...int32[])): void;

collect("Ada", 1, 2);

=== dir ===
declare function collect(...values: (string, ...int32[])): void;
/// @type.symbol symbol=collect source="declare function collect(...values: (string, ...int32[])): void" type=(string, ...int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

collect("Ada", 1, 2);
/// @resolution.name source=collect target=collect
/// @resolution.call source="collect(\"Ada\", 1, 2)" parameters=(string, int32[]) arguments=(provided("Ada") as string, rest(provided(1) as int32, provided(2) as int32) pack=arrayFromOwnedSlice as int32) return=void kind=symbol target=collect
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
"#,
    );
}

/// A captured function parameter sequence can declare another function type.
#[test]
fn test_infer_function_parameters() {
    let session = TestSession::single(
        r#"
type Rebuild<T> = T extends (...values: infer P) => infer R ? (...values: P) => R : never;

type Callback = Rebuild<(name: string, count?: int32) => boolean>;

declare const callback: Callback;

const result = callback("Ada", 2);
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
type Rebuild<T> = T extends (...values: infer P) => infer R ? (...values: P) => R : never;

type Callback = Rebuild<(name: string, count?: int32) => boolean>;

declare const callback: Callback;

const result: boolean = callback("Ada", 2 as int32 | undefined);

=== dir ===
type Rebuild<T> = T extends (...values: infer P) => infer R ? (...values: P) => R : never;
/// @generic.template symbol=Rebuild parameters=(T)
/// @type.symbol symbol=Rebuild type=T extends (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => infer R ? (...Rebuild.P) => Rebuild.R : never
/// @definition.type symbol=Rebuild template=(T) value=T extends (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => infer R ? (...Rebuild.P) => Rebuild.R : never
/// @type.symbol symbol=Rebuild.T source=T type=T
/// @resolution.name source=T target=Rebuild.T
/// @type.symbol symbol=Rebuild.values#1 source="...values: infer P" type=infer P extends (readonly ...ReadonlyArray<unknown>,)
/// @type.symbol symbol=Rebuild.values#2 source="...values: P" type=Rebuild.P
/// @resolution.name source=P target=Rebuild.P
/// @resolution.name source=R target=Rebuild.R

type Callback = Rebuild<(name: string, count?: int32) => boolean>;
/// @type.symbol symbol=Callback source="type Callback = Rebuild<(name: string, count?: int32) => boolean>" type=(string, int32 | undefined?) => boolean
/// @definition.type symbol=Callback source="type Callback = Rebuild<(name: string, count?: int32) => boolean>" value=Rebuild<(string, int32 | undefined?) => boolean>
/// @resolution.name source=Rebuild target=Rebuild
/// @type.symbol symbol=Callback.name source="name: string" type=string
/// @type.symbol symbol=Callback.count source="count?: int32" type=int32 | undefined

declare const callback: Callback;
/// @type.symbol symbol=callback source=callback type=Callback
/// @resolution.pattern source=callback kind=binding target=callback
/// @resolution.name source=Callback target=Callback

const result = callback("Ada", 2);
/// @type.symbol symbol=result source=result type=boolean
/// @resolution.pattern source=result kind=binding target=result
/// @resolution.name source=callback target=callback
/// @resolution.call source="callback(\"Ada\", 2)" parameters=(string, int32 | undefined) arguments=(provided("Ada") as string, provided(2) as int32 | undefined) return=boolean kind=expression target=expression
/// @resolution.place source=callback placement="local" lifetime="static" access="immutable"
/// @resolution.access source=callback root=callback
"#);
}

/// A captured constructor parameter sequence can declare another constructor type.
#[test]
fn test_infer_constructor_parameters() {
    let session = TestSession::single(
        r#"
class User {
    constructor(name: string) {}
}

type Construct<T> = T extends new (...values: infer P) => infer R ? new (...values: P) => R : never;

type Constructor = Construct<typeof User>;

declare const constructor: Constructor;

const instance = new constructor("Ada");
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
class User {
    constructor(name: string) {}
}

type Construct<T> = T extends new (...values: infer P) => infer R ? new (...values: P) => R : never;

type Constructor = Construct<typeof User>;

declare const constructor: Constructor;

const instance: User = new constructor("Ada");

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

type Construct<T> = T extends new (...values: infer P) => infer R ? new (...values: P) => R : never;
/// @generic.template symbol=Construct parameters=(T)
/// @type.symbol symbol=Construct type=T extends new (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => infer R ? new (...Construct.P) => Construct.R : never
/// @definition.type symbol=Construct template=(T) value=T extends new (...infer P extends (readonly ...ReadonlyArray<unknown>,)) => infer R ? new (...Construct.P) => Construct.R : never
/// @type.symbol symbol=Construct.T source=T type=T
/// @resolution.name source=T target=Construct.T
/// @type.symbol symbol=Construct.values#1 source="...values: infer P" type=infer P extends (readonly ...ReadonlyArray<unknown>,)
/// @type.symbol symbol=Construct.values#2 source="...values: P" type=Construct.P
/// @resolution.name source=P target=Construct.P
/// @resolution.name source=R target=Construct.R

type Constructor = Construct<typeof User>;
/// @type.symbol symbol=Constructor source="type Constructor = Construct<typeof User>" type=new (string) => User
/// @definition.type symbol=Constructor source="type Constructor = Construct<typeof User>" value=Construct<typeof User>
/// @resolution.name source=Construct target=Construct
/// @resolution.name source=User target=User

declare const constructor: Constructor;
/// @type.symbol symbol=constructor source=constructor type=Constructor
/// @resolution.pattern source=constructor kind=binding target=constructor
/// @resolution.name source=Constructor target=Constructor

const instance = new constructor("Ada");
/// @type.symbol symbol=instance source=instance type=User
/// @resolution.pattern source=instance kind=binding target=instance
/// @resolution.call source="new constructor(\"Ada\")" parameters=(string) arguments=(provided("Ada") as string) return=User kind=expression target=expression
/// @resolution.name source=constructor target=constructor
/// @resolution.place source=constructor placement="local" lifetime="static" access="immutable"
/// @resolution.access source=constructor root=constructor
"#);
}

/// A captured tuple rest can declare a function's remaining parameters.
#[test]
fn test_infer_tuple_rest() {
    let session = TestSession::single(
        r#"
type Tail<T> = T extends (unknown, ...infer P) ? (...values: P) => void : never;

type Remaining = Tail<(boolean, string, int32)>;

declare const remaining: Remaining;

remaining("Ada", 2);
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
type Tail<T> = T extends (unknown, ...infer P) ? (...values: P) => void : never;

type Remaining = Tail<(boolean, string, int32)>;

declare const remaining: Remaining;

remaining("Ada", 2);

=== dir ===
type Tail<T> = T extends (unknown, ...infer P) ? (...values: P) => void : never;
/// @generic.template symbol=Tail parameters=(T)
/// @type.symbol symbol=Tail source="type Tail<T> = T extends (unknown, ...infer P) ? (...values: P) => void : never" type=T extends (unknown, ...infer P extends (readonly ...ReadonlyArray<unknown>,)) ? (...Tail.P) => void : never
/// @definition.type symbol=Tail source="type Tail<T> = T extends (unknown, ...infer P) ? (...values: P) => void : never" template=(T) value=T extends (unknown, ...infer P extends (readonly ...ReadonlyArray<unknown>,)) ? (...Tail.P) => void : never
/// @type.symbol symbol=Tail.T source=T type=T
/// @resolution.name source=T target=Tail.T
/// @type.symbol symbol=Tail.values source="...values: P" type=Tail.P
/// @resolution.name source=P target=Tail.P

type Remaining = Tail<(boolean, string, int32)>;
/// @type.symbol symbol=Remaining source="type Remaining = Tail<(boolean, string, int32)>" type=(string, int32) => void
/// @definition.type symbol=Remaining source="type Remaining = Tail<(boolean, string, int32)>" value=Tail<(boolean, string, int32)>
/// @resolution.name source=Tail target=Tail

declare const remaining: Remaining;
/// @type.symbol symbol=remaining source=remaining type=Remaining
/// @resolution.pattern source=remaining kind=binding target=remaining
/// @resolution.name source=Remaining target=Remaining

remaining("Ada", 2);
/// @resolution.name source=remaining target=remaining
/// @resolution.call source="remaining(\"Ada\", 2)" parameters=(string, int32) arguments=(provided("Ada") as string, provided(2) as int32) return=void kind=expression target=expression
/// @resolution.place source=remaining placement="local" lifetime="static" access="immutable"
/// @resolution.access source=remaining root=remaining
"#);
}

/// Rest annotations are checked on declarations even when they have no callers.
#[test]
fn test_declare_scalar_rest_parameters() {
    let session = TestSession::single(
        r#"
type Callback = (...values: string) => void;

class Logger {
    constructor(...values: int32) {}

    write(...values: boolean): void {}
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
type Callback = (...values: string) => void;

class Logger {
    constructor(...values: int32) {}

    write(...values: boolean): void {}
}

=== dir ===
type Callback = (...values: string) => void;
/// @type.symbol symbol=Callback source="type Callback = (...values: string) => void" type=(...string) => void
/// @definition.type symbol=Callback source="type Callback = (...values: string) => void" value=(...string) => void
/// @type.symbol symbol=Callback.values source="...values: string" type=string

class Logger {
/// @type.symbol symbol=Logger type=typeof Logger
/// @definition.class symbol=Logger
/// @definition.method symbol=Logger.constructor source="constructor(...values: int32) {}" slot=constructor role=constructor type=(this: &'managed Logger, ...int32) => Logger
/// @definition.method symbol=Logger.write source="write(...values: boolean): void {}" slot=write type=(this: Logger, ...boolean) => void

    constructor(...values: int32) {}
    /// @type.symbol symbol=Logger.constructor source="constructor(...values: int32) {}" type=(this: &'managed Logger, ...int32) => Logger
    /// @type.symbol symbol=Logger.constructor.this type=&'managed Logger
    /// @type.symbol symbol=Logger.constructor.values source="...values: int32" type=int32

    write(...values: boolean): void {}
    /// @type.symbol symbol=Logger.write source="write(...values: boolean): void {}" type=(this: Logger, ...boolean) => void
    /// @type.symbol symbol=Logger.write.this type=Logger
    /// @type.symbol symbol=Logger.write.values source="...values: boolean" type=boolean

}
"#, r#"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type string must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=2 column=21 span="values" line_source="type Callback = (...values: string) => void;"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type int32 must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=5 column=20 span="values" line_source="constructor(...values: int32) {}"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type boolean must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=7 column=14 span="values" line_source="write(...values: boolean): void {}"
"#);
}

/// A bottom rest type admits no argument list, including an empty one.
#[test]
fn test_call_bottom_rest_parameter() {
    let session = TestSession::single(
        r#"
declare function impossible(...values: never): void;

impossible();
impossible(1);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function impossible(...values: never): void;

impossible();
impossible(1);

=== dir ===
declare function impossible(...values: never): void;
/// @type.symbol symbol=impossible source="declare function impossible(...values: never): void" type=(...never) => void

impossible();
/// @resolution.name source=impossible target=impossible
/// @resolution.rejected source=impossible()

impossible(1);
/// @resolution.name source=impossible target=impossible
/// @resolution.rejected source=impossible(1)
"#, r#"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments (none)"
/// @diagnostic.label line=4 column=1 span="impossible()" line_source="impossible();"
/// @diagnostic.note message="the candidate '(values: ...never) => void' does not apply"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('1')"
/// @diagnostic.label line=5 column=1 span="impossible(1)" line_source="impossible(1);"
/// @diagnostic.note message="the candidate '(values: ...never) => void' does not apply"
"#);
}

/// Scalar annotations cannot describe a rest parameter's argument sequence.
#[test]
fn test_call_scalar_rest_parameter() {
    let session = TestSession::single(
        r#"
declare function collect(...values: int32): void;

collect();
collect(1, 2);
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function collect(...values: int32): void;

collect();
collect(1, 2);

=== dir ===
declare function collect(...values: int32): void;
/// @type.symbol symbol=collect source="declare function collect(...values: int32): void" type=(...int32) => void

collect();
/// @resolution.name source=collect target=collect
/// @resolution.rejected source=collect()

collect(1, 2);
/// @resolution.name source=collect target=collect
/// @resolution.rejected source="collect(1, 2)"
"#, r#"
/// @diagnostic.error id=invalid-rest-parameter message="rest parameter type int32 must be a dynamic array, slice, or tuple"
/// @diagnostic.label line=2 column=29 span="values" line_source="declare function collect(...values: int32): void;"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments (none)"
/// @diagnostic.label line=4 column=1 span="collect()" line_source="collect();"
/// @diagnostic.note message="the candidate '(values: ...int32) => void' does not apply"
/// @diagnostic.error id=no-matching-call message="no overload matches arguments ('1', '2')"
/// @diagnostic.label line=5 column=1 span="collect(1, 2)" line_source="collect(1, 2);"
/// @diagnostic.note message="the candidate '(values: ...int32) => void' does not apply"
"#);
}

/// Array and slice rest parameters accept individual arguments of their element type.
#[test]
fn test_call_collection_rest_parameters() {
    let session = TestSession::single(
        r#"
declare function array(...values: int32[]): void;

declare function slice(...values: &readonly [int32]): void;

array(1, 2);
slice(1, 2);
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
declare function array(...values: int32[]): void;

declare function slice<'a>(...values: &readonly [int32]): void;

array(1, 2);
slice<"frame">(1, 2);

=== dir ===
declare function array(...values: int32[]): void;
/// @type.symbol symbol=array source="declare function array(...values: int32[]): void" type=(...int32[]) => void
/// @generic.instance id=Array<int32> template=Array arguments=(int32)
/// @generic.instance id=sliceAssumeInit<MaybeUninit<int32>> template=sliceAssumeInit arguments=(MaybeUninit<int32>)
/// @generic.instance id=sliceUninit<MaybeUninit<int32>> template=sliceUninit arguments=(MaybeUninit<int32>)

declare function slice(...values: &readonly [int32]): void;
/// @generic.template symbol=slice parameters=('a)
/// @type.symbol symbol=slice source="declare function slice(...values: &readonly [int32]): void" type=<slice.'a>(...&slice.'a readonly Slice<int32>) => void

array(1, 2);
/// @resolution.name source=array target=array
/// @resolution.call source="array(1, 2)" parameters=(int32[]) arguments=(rest(provided(1) as int32, provided(2) as int32) pack=arrayFromOwnedSlice as int32) return=void kind=symbol target=array
/// @generic.instantiation id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)
/// @generic.instance id=arrayFromOwnedSlice<int32> template=arrayFromOwnedSlice arguments=(int32)

slice(1, 2);
/// @resolution.name source=slice target=slice
/// @resolution.call source="slice(1, 2)" parameters=(&'frame readonly Slice<int32>) arguments=(rest(provided(1) as int32, provided(2) as int32) as int32) return=void regions=("frame") kind=symbol target=slice instance="slice<\"frame\">"
/// @generic.instantiation id="slice<\"frame\">" template=slice arguments=("frame")
/// @generic.instance id="slice<\"frame\">" template=slice arguments=("frame")
"#);
}
