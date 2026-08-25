use crate::tests::{DirRows, TestSession};

#[test]
fn test_narrow_borrowed_union_parameter_in_chained_callback() {
    let session = TestSession::single(
        r#"
function firstPositive(values: (int32 | undefined)[]): int32 | undefined {
    values.iterator().map((value) => value).find((value) => value !== undefined && value > 0)
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function firstPositive(values: (int32 | undefined)[]): int32 | undefined {
    values.iterator<int32 | undefined, "local">().map<int32 | undefined, int32 | undefined>(
        (value: int32 | undefined): int32 | undefined => value,
    ).find<int32 | undefined, int32 | undefined, int32 | undefined, Iterator<int32 | undefined>>(
        (value: Borrowed<int32 | undefined, 'a & P1, "readonly">): boolean =>
            value !== (undefined as int32 | undefined) && (value as int32) > 0,
    )
}

=== dir ===
function firstPositive(values: (int32 | undefined)[]): int32 | undefined {
/// @type.symbol symbol=firstPositive type=(int32 | undefined[]) => int32 | undefined
/// @generic.instance id="Array<int32 | undefined>" template=Array arguments=(int32 | undefined)
/// @generic.instance id="MaybeUninit<int32 | undefined>" template=MaybeUninit arguments=(int32 | undefined)
/// @generic.instance id="new<MaybeUninit<int32 | undefined>>" template=new arguments=(MaybeUninit<int32 | undefined>)
/// @type.symbol symbol=firstPositive.values source="values: (int32 | undefined)[]" type=int32 | undefined[]

    values.iterator().map((value) => value).find((value) => value !== undefined && value > 0)
    /// @resolution.name source=values target=firstPositive.values
    /// @resolution.member source="values.iterator().map((value) => value).find" receiver=MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined> type=(this: MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined>, Function<(Borrowed<int32 | undefined, type_expression.'a & type_expression.P1, "readonly">, isize), boolean>) => int32 | undefined kind=symbol target_receiver=MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined> target=Iterator.find
    /// @resolution.member source=values.iterator receiver=int32 | undefined[] type=<iterator#2.P0: Place>(this: Managed<int32 | undefined[], iterator#2.P0>) => Iterator<int32 | undefined> kind=symbol target_receiver=int32 | undefined[] target=iterator#2
    /// @resolution.member source=values.iterator().map receiver=Iterator<int32 | undefined> type=<Iterator.map.U>(this: Iterator<int32 | undefined>, Function<(int32 | undefined, isize), Iterator.map.U>) => MapIterator<Iterator<int32 | undefined>, int32 | undefined, Iterator.map.U> kind=symbol target_receiver=Iterator<int32 | undefined> dispatch=dynamic constraint=Iterator<int32 | undefined> target=Iterator.map
    /// @resolution.call parameters=(Function<(Borrowed<int32 | undefined, type_expression.'a & type_expression.P1, "readonly">, isize), boolean>) arguments=(provided((value) => value !== undefined && value > 0) as Function<(Borrowed<int32 | undefined, type_expression.'a & type_expression.P1, "readonly">, isize), boolean>) return=int32 | undefined kind=symbol target=Iterator.find receiver=MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined> instance="MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined>.<extension#3>.find"
    /// @resolution.call source="values.iterator().map((value) => value)" parameters=(Function<(int32 | undefined, isize), int32 | undefined>) arguments=(provided((value) => value) as Function<(int32 | undefined, isize), int32 | undefined>) return=MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined> kind=dynamic target=Iterator.map receiver=Iterator<int32 | undefined> constraint=Iterator<int32 | undefined> generic_arguments=(int32 | undefined, int32 | undefined)
    /// @resolution.call source=values.iterator() parameters=() return=Iterator<int32 | undefined> kind=symbol target=iterator#2 receiver=int32 | undefined[] instance="Array<int32 | undefined>.<extension#3>.iterator#2<\"local\">"
    /// @resolution.place source=values placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=values root=firstPositive.values
    /// @generic.instantiation id="Iterator.find<int32 | undefined, int32 | undefined, int32 | undefined, Iterator<int32 | undefined>>" template=Iterator.find arguments=(int32 | undefined, int32 | undefined, int32 | undefined, Iterator<int32 | undefined>)
    /// @generic.instantiation id="Iterator.map<int32 | undefined>" template=Iterator.map arguments=(int32 | undefined)
    /// @generic.instantiation id="iterator#2<int32 | undefined, \"local\">" template=iterator#2 arguments=(int32 | undefined, "local")
    /// @generic.instantiation id="iterator#2<int32 | undefined>" template=iterator#2 arguments=(int32 | undefined)
    /// @generic.instance id="DropIterator<Iterator<int32 | undefined>, int32 | undefined>" template=DropIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="DropWhileIterator<Iterator<int32 | undefined>, int32 | undefined>" template=DropWhileIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="EnumeratedIterator<Iterator<int32 | undefined>, int32 | undefined>" template=EnumeratedIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="FilterIterator<Iterator<int32 | undefined>, int32 | undefined>" template=FilterIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="InspectIterator<Iterator<int32 | undefined>, int32 | undefined>" template=InspectIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="Iterator.find<int32 | undefined, int32 | undefined, int32 | undefined, Iterator<int32 | undefined>>" template=Iterator.find arguments=(int32 | undefined, int32 | undefined, int32 | undefined, Iterator<int32 | undefined>)
    /// @generic.instance id="Iterator<int32 | undefined>" template=Iterator arguments=(int32 | undefined)
    /// @generic.instance id="IteratorResult<int32 | undefined, Iterator<int32 | undefined>.Return>" template=IteratorResult arguments=(int32 | undefined, Iterator<int32 | undefined>.Return)
    /// @generic.instance id="IteratorResult<int32 | undefined, void>" template=IteratorResult arguments=(int32 | undefined, void)
    /// @generic.instance id="IteratorReturn<Iterator<int32 | undefined>.Return>" template=IteratorReturn arguments=(Iterator<int32 | undefined>.Return)
    /// @generic.instance id="IteratorYield<int32 | undefined>" template=IteratorYield arguments=(int32 | undefined)
    /// @generic.instance id="MapIterator<Iterator<int32 | undefined>, int32 | undefined, int32 | undefined>" template=MapIterator arguments=(Iterator<int32 | undefined>, int32 | undefined, int32 | undefined)
    /// @generic.instance id="PeekableIterator<Iterator<int32 | undefined>, int32 | undefined>" template=PeekableIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="PlaceOf<DropWhileIterator<Iterator<int32 | undefined>, int32 | undefined>>" template=PlaceOf arguments=(DropWhileIterator<Iterator<int32 | undefined>, int32 | undefined>)
    /// @generic.instance id="PlaceOf<FilterIterator<Iterator<int32 | undefined>, int32 | undefined>>" template=PlaceOf arguments=(FilterIterator<Iterator<int32 | undefined>, int32 | undefined>)
    /// @generic.instance id="PlaceOf<InspectIterator<Iterator<int32 | undefined>, int32 | undefined>>" template=PlaceOf arguments=(InspectIterator<Iterator<int32 | undefined>, int32 | undefined>)
    /// @generic.instance id="PlaceOf<TakeWhileIterator<Iterator<int32 | undefined>, int32 | undefined>>" template=PlaceOf arguments=(TakeWhileIterator<Iterator<int32 | undefined>, int32 | undefined>)
    /// @generic.instance id="TakeIterator<Iterator<int32 | undefined>, int32 | undefined>" template=TakeIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="TakeWhileIterator<Iterator<int32 | undefined>, int32 | undefined>" template=TakeWhileIterator arguments=(Iterator<int32 | undefined>, int32 | undefined)
    /// @generic.instance id="iterator#2<int32 | undefined, \"local\">" template=iterator#2 arguments=(int32 | undefined, "local")
    /// @generic.instance id=IteratorReturn<void> template=IteratorReturn arguments=(void)
    /// @type.symbol symbol=firstPositive.symbol3 source="(value) => value" type=Function<(int32 | undefined,), int32 | undefined>
    /// @type.symbol symbol=firstPositive.symbol3.value source=value type=int32 | undefined
    /// @resolution.name source=value target=firstPositive.symbol3.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=firstPositive.symbol3.value
    /// @type.symbol symbol=firstPositive.symbol5 source="(value) => value !== undefined && value > 0" type=Function<(Borrowed<int32 | undefined, type_expression.'a & type_expression.P1, "readonly">,), boolean>
    /// @type.symbol symbol=firstPositive.symbol5.value source=value type=Borrowed<int32 | undefined, type_expression.'a & type_expression.P1, "readonly">
    /// @resolution.name source=value target=firstPositive.symbol5.value
    /// @resolution.operator source="value !== undefined && value > 0" type=boolean operator="&&" kind=builtin operands=[value !== undefined as boolean families=(boolean), value > 0 as boolean families=(boolean)]
    /// @resolution.operator source="value !== undefined" type=boolean operator="!==" kind=builtin operands=[value as int32 | undefined families=(integer | undefined), undefined as int32 | undefined families=(integer | undefined)]
    /// @resolution.place source=value placement=type_expression.P1 lifetime=type_expression.'a access="readonly"
    /// @resolution.access source=value root=firstPositive.symbol5.value
    /// @resolution.name source=value target=firstPositive.symbol5.value
    /// @resolution.operator source="value > 0" type=boolean operator=">" kind=builtin operands=[value as int32 families=(integer), 0 as int32 families=(integer)]
    /// @resolution.place source=value placement=type_expression.P1 lifetime=type_expression.'a access="readonly"
    /// @resolution.access source=value root=firstPositive.symbol5.value

}
"#,
    );
}

#[test]
fn test_function_type_parameter_records_its_declared_symbol() {
    let session = TestSession::single(
        r#"
function schedule(callback: (ready: boolean) => void): void {}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
function schedule(callback: (arg0: boolean) => void): void {}

=== dir ===
function schedule(callback: (ready: boolean) => void): void {}
/// @type.symbol symbol=schedule source="function schedule(callback: (ready: boolean) => void): void {}" type=(Function<(boolean,), void>) => void
/// @type.symbol symbol=schedule.callback source="callback: (ready: boolean) => void" type=Function<(boolean,), void>
/// @type.symbol symbol=schedule.ready source="ready: boolean" type=boolean
"#);
}

#[test]
fn test_free_function_call_selects_function_symbol() {
    let session = TestSession::single(
        r#"
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value = add(1, 2);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types()
            ,
        r#"
=== annotated ===
function add(left: int32, right: int32): int32 {
    return left + right;
}

const value: int32 = add(1, 2);

=== dir ===
function add(left: int32, right: int32): int32 {
/// @type.symbol symbol=add type=(int32, int32) => int32
/// @type.symbol symbol=add.left source="left: int32" type=int32
/// @type.symbol symbol=add.right source="right: int32" type=int32

    return left + right;
    /// @type.node source="left + right" type=int32
    /// @resolution.name source=left target=add.left
    /// @resolution.operator source="left + right" type=int32 operator="+" kind=builtin operands=[left as int32 families=(integer), right as int32 families=(integer)]
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=add.left
    /// @resolution.name source=right target=add.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=add.right

}

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=symbol target=add
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#);
}

#[test]
fn test_imported_function_call_selects_exported_symbol() {
    let compiler = TestSession::builder()
        .module(
            "math.ds",
            r#"
export function add(left: int32, right: int32): int32 {
    return left + right;
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { add } from "./math.ds";

const value = add(1, 2);
"#,
        )
        .build();

    compiler.assert_dir(
        "main.ds",
        DirRows::checked()
            .with_node_types()
            .without_reference_types(),
        r#"
=== annotated ===
import { add } from "./math.ds";

const value: int32 = add(1, 2);

=== dir ===
import { add } from "./math.ds";

const value = add(1, 2);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="add(1, 2)" type=int32
/// @resolution.name source=add target=math.add
/// @resolution.call source="add(1, 2)" parameters=(int32, int32) arguments=(provided(1) as int32, provided(2) as int32) return=int32 kind=symbol target=math.add
/// @type.node source=1 type=1
/// @type.node source=2 type=2
"#,
    );
}

/// Reject an optional-parameter function where a required-parameter function type is expected.
///
/// An optional-parameter callback stores its parameter as a union, so passing it into a
/// required-parameter function type changes the interior representation.
/// A synthesized thunk coercion is the designed path to make this flow again.
#[test]
fn test_optional_parameter_function_needs_a_thunk_for_required_targets() {
    let session = TestSession::single(
        r#"
function source(value?: unknown): void {}
declare function use(callback: (value: unknown) => void): void;

use(source);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function source(value?: unknown): void {}
declare function use(callback: (arg0: unknown) => void): void;

use(source);

=== dir ===
function source(value?: unknown): void {}
/// @type.symbol symbol=source source="function source(value?: unknown): void {}" type=(unknown | undefined?) => void
/// @type.symbol symbol=source.value source="value?: unknown" type=unknown | undefined

declare function use(callback: (value: unknown) => void): void;
/// @type.symbol symbol=use source="declare function use(callback: (value: unknown) => void): void" type=(Function<(unknown,), void>) => void
/// @type.symbol symbol=use.callback source="callback: (value: unknown) => void" type=Function<(unknown,), void>
/// @type.symbol symbol=use.value source="value: unknown" type=unknown

use(source);
/// @type.node source=use type=(Function<(unknown,), void>) => void
/// @type.node source=use(source) type=void
/// @resolution.name source=use target=use
/// @resolution.call source=use(source) parameters=(Function<(unknown,), void>) arguments=(provided(source) as Function<(unknown,), void>) return=void kind=symbol target=use
/// @type.node source=source type=(unknown | undefined?) => void
/// @resolution.name source=source target=source
/// @resolution.function source=source type=(unknown | undefined?) => void target=source
"#,
        r#"
/// @diagnostic.error id=argument-not-assignable message="argument of type '(value: unknown | undefined) => void' is not assignable to parameter of type '(value: unknown) => void'"
/// @diagnostic.label line=5 column=5 span="source" line_source="use(source);"
/// @diagnostic.related line=5 column=1 span="use(source)" line_source="use(source);" message="in this call"
"#,
    );
}

#[test]
fn test_callback_can_ignore_contextual_parameter() {
    let session = TestSession::single(
        r#"
declare function map<T>(callback: (value: unknown) => T): T;

const value = map(() => 1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare function map<T>(callback: (arg0: unknown) => T): T;

const value: int64 = map<int64>((): int64 => 1);

=== dir ===
declare function map<T>(callback: (value: unknown) => T): T;
/// @generic.template symbol=map parameters=(T)
/// @type.symbol symbol=map source="declare function map<T>(callback: (value: unknown) => T): T" type=<T>(Function<(unknown,), T>) => T
/// @type.symbol symbol=map.T source=T type=T
/// @type.symbol symbol=map.callback source="callback: (value: unknown) => T" type=Function<(unknown,), T>
/// @type.symbol symbol=map.value source="value: unknown" type=unknown
/// @resolution.name source=T target=map.T
/// @resolution.name source=T target=map.T

const value = map(() => 1);
/// @type.symbol symbol=value source=value type=int64
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source="map(() => 1)" type=int64
/// @type.node source=map type=(Function<(unknown,), int64>) => int64
/// @resolution.name source=map target=map
/// @resolution.call source="map(() => 1)" parameters=(Function<(unknown,), int64>) arguments=(provided(() => 1) as Function<(unknown,), int64>) return=int64 kind=symbol target=map instance=map<int64>
/// @generic.instantiation id=map<int64> template=map arguments=(int64)
/// @generic.instance id=map<int64> template=map arguments=(int64)
/// @type.symbol symbol=symbol5 source="() => 1" type=Function<(), int64>
/// @type.node source="() => 1" type=Function<(), int64>
/// @type.node source=1 type=1
"#,
    );
}

/// Omit a defaulted parameter at the call site while keeping its exact type inside the body.
#[test]
fn test_defaulted_parameter_may_be_omitted_at_the_call() {
    let session = TestSession::single(
        r#"
function greet(name: string = "world"): string {
    name
}

let short = greet();
let long = greet("compiler");
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function greet(name: string = "world"): string {
    name
}

let short: string = greet();
let long: string = greet("compiler");

=== dir ===
function greet(name: string = "world"): string {
/// @type.symbol symbol=greet type=(string?) => string
/// @type.symbol symbol=greet.name source="name: string = \"world\"" type=string
/// @type.node source="\"world\"" type="world"

    name
    /// @type.node source=name type=string
    /// @resolution.name source=name target=greet.name
    /// @resolution.place source=name placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=name root=greet.name

}

let short = greet();
/// @type.symbol symbol=short source=short type=string
/// @resolution.pattern source=short kind=binding target=short
/// @type.node source=greet type=(string?) => string
/// @type.node source=greet() type=string
/// @resolution.name source=greet target=greet
/// @resolution.call source=greet() parameters=(string) arguments=(omitted as string) return=string kind=symbol target=greet

let long = greet("compiler");
/// @type.symbol symbol=long source=long type=string
/// @resolution.pattern source=long kind=binding target=long
/// @type.node source="greet(\"compiler\")" type=string
/// @type.node source=greet type=(string?) => string
/// @resolution.name source=greet target=greet
/// @resolution.call source="greet(\"compiler\")" parameters=(string) arguments=(provided("compiler") as string) return=string kind=symbol target=greet
/// @type.node source="\"compiler\"" type="compiler"
"#,
    );
}
