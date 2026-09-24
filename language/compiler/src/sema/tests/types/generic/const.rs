use crate::tests::{DirRows, TestSession};

#[test]
fn test_static_value_argument_specializes_array_length() {
    let session = TestSession::single(
        r#"
function take<const N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes = take<4>([1, 2, 3, 4]);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function take<const N: uint>(value: [uint8; N]): [uint8; N] {
    return value;
}

const bytes: [uint8; 4] = take<4>([1, 2, 3, 4]);

=== dir ===
function take<const N: uint>(value: [uint8; N]): [uint8; N] {
/// @generic.template symbol=take parameters=(const N: uint64)
/// @type.symbol symbol=take type=<const N: uint64>(FixedArray<uint8, N>) => FixedArray<uint8, N>
/// @type.symbol symbol=take.N source="const N: uint" type=N
/// @type.symbol symbol=take.value source="value: [uint8; N]" type=FixedArray<uint8, N>
/// @resolution.name source=N target=take.N
/// @resolution.name source=N target=take.N

    return value;
    /// @type.node source=value type=FixedArray<uint8, N>
    /// @resolution.name source=value target=take.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=take.value

}

const bytes = take<4>([1, 2, 3, 4]);
/// @type.symbol symbol=bytes source=bytes type=FixedArray<uint8, 4>
/// @resolution.pattern source=bytes kind=binding target=bytes
/// @type.node source="take<4>([1, 2, 3, 4])" type=FixedArray<uint8, 4>
/// @type.node source=take type=(FixedArray<uint8, 4>) => FixedArray<uint8, 4>
/// @resolution.name source=take target=take
/// @resolution.call source="take<4>([1, 2, 3, 4])" parameters=(FixedArray<uint8, 4>) arguments=(provided([1, 2, 3, 4]) as FixedArray<uint8, 4>) return=FixedArray<uint8, 4> kind=symbol target=take instance=take<4>
/// @generic.instantiation id=take<4> template=take arguments=(4)
/// @generic.instance id=take<4> template=take arguments=(4)
/// @type.node source=[1, 2, 3, 4] type=FixedArray<uint8, 4>
/// @type.node source=1 type=1
/// @type.node source=2 type=2
/// @type.node source=3 type=3
/// @type.node source=4 type=4
"#,
    );
}

#[test]
fn test_defaulted_static_value_argument_uses_literal_default() {
    let session = TestSession::single(
        r#"
function choose<const Flag: boolean = true>(value: int32): int32 {
    return value;
}

const value = choose(1);
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
function choose<const Flag: boolean = true>(value: int32): int32 {
    return value;
}

const value: int32 = choose<true>(1);

=== dir ===
function choose<const Flag: boolean = true>(value: int32): int32 {
/// @generic.template symbol=choose parameters=(const Flag: boolean = true)
/// @type.symbol symbol=choose type=<const Flag: boolean = true>(int32) => int32
/// @type.symbol symbol=choose.Flag source="const Flag: boolean = true" type=Flag
/// @type.symbol symbol=choose.value source="value: int32" type=int32

    return value;
    /// @type.node source=value type=int32
    /// @resolution.name source=value target=choose.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=choose.value

}

const value = choose(1);
/// @type.symbol symbol=value source=value type=int32
/// @resolution.pattern source=value kind=binding target=value
/// @type.node source=choose type=(int32) => int32
/// @type.node source=choose(1) type=int32
/// @resolution.name source=choose target=choose
/// @resolution.call source=choose(1) parameters=(int32) arguments=(provided(1) as int32) return=int32 kind=symbol target=choose instance=choose<true>
/// @generic.instantiation id=choose<true> template=choose arguments=(true)
/// @generic.instance id=choose<true> template=choose arguments=(true)
/// @type.node source=1 type=1
"#,
    );
}

#[test]
fn test_function_type_const_generic_binds_return_type() {
    let session = TestSession::single(
        r#"
type Read = <const N: uint>() => [uint8; N];

declare const read: Read;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_statics(),
        r#"
=== annotated ===
type Read = <const N: uint>() => [uint8; N];

declare const read: Read;

=== dir ===
type Read = <const N: uint>() => [uint8; N];
/// @type.symbol symbol=Read source="type Read = <const N: uint>() => [uint8; N]" type=<const N: uint64>() => FixedArray<uint8, N>
/// @definition.type symbol=Read source="type Read = <const N: uint>() => [uint8; N]" value=<const N: uint64>() => FixedArray<uint8, N>
/// @generic.template source=type_expression parameters=(const N: uint64)
/// @type.symbol symbol=Read.N source="const N: uint" type=N
/// @resolution.name source=N target=Read.N

declare const read: Read;
/// @type.symbol symbol=read source=read type=Read
/// @resolution.pattern source=read kind=binding target=read
/// @resolution.name source=Read target=Read
"#,
    );
}

#[test]
fn test_static_literal_arguments_specialize_aliases() {
    let session = TestSession::single(
        r#"
type Tagged<const Tag: string> = { tag: Tag };
type Flagged<const Config: { name: string; enabled: boolean }> = Config;

declare const tagged: Tagged<"alpha">;
declare const flagged: Flagged<{ name: "search"; enabled: true }>;
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
type Tagged<const Tag: string> = { tag: Tag };
type Flagged<const Config: { name: string; enabled: boolean }> = Config;

declare const tagged: Tagged<"alpha">;
declare const flagged: Flagged<{ name: "search"; enabled: true }>;

=== dir ===
type Tagged<const Tag: string> = { tag: Tag };
/// @generic.template symbol=Tagged parameters=(const Tag: string)
/// @type.symbol symbol=Tagged source="type Tagged<const Tag: string> = { tag: Tag }" type={ tag: Tag }
/// @definition.type symbol=Tagged source="type Tagged<const Tag: string> = { tag: Tag }" template=(const Tag: string) value={ tag: Tag }
/// @type.symbol symbol=Tagged.Tag source="const Tag: string" type=Tag
/// @type.symbol symbol=Tagged.tag source="tag: Tag" type=Tag
/// @resolution.name source=Tag target=Tagged.Tag

type Flagged<const Config: { name: string; enabled: boolean }> = Config;
/// @generic.template symbol=Flagged parameters=(const Config: { name: string; enabled: boolean })
/// @type.symbol symbol=Flagged source="type Flagged<const Config: { name: string; enabled: boolean }> = Config" type=Config
/// @definition.type symbol=Flagged source="type Flagged<const Config: { name: string; enabled: boolean }> = Config" template=(const Config: { name: string; enabled: boolean }) value=Config
/// @type.symbol symbol=Flagged.Config source="const Config: { name: string; enabled: boolean }" type=Config
/// @type.symbol symbol=Flagged.name source="name: string" type=string
/// @type.symbol symbol=Flagged.enabled source="enabled: boolean" type=boolean
/// @resolution.name source=Config target=Flagged.Config

declare const tagged: Tagged<"alpha">;
/// @type.symbol symbol=tagged source=tagged type=Tagged<"alpha">
/// @resolution.pattern source=tagged kind=binding target=tagged
/// @generic.instance id="Tagged<\"alpha\">" template=Tagged arguments=("alpha")
/// @resolution.name source=Tagged target=Tagged

declare const flagged: Flagged<{ name: "search"; enabled: true }>;
/// @type.symbol symbol=flagged source=flagged type=Flagged<{ name: "search"; enabled: true }>
/// @resolution.pattern source=flagged kind=binding target=flagged
/// @resolution.name source=Flagged target=Flagged
/// @type.symbol symbol=name source="name: \"search\"" type="search"
/// @type.symbol symbol=enabled source="enabled: true" type=true
"#,
    );
}

#[test]
fn test_reject_a_comparison_between_usize_and_int64() {
    let session = TestSession::single(
        r#"
function f(a: usize, b: int64): boolean {
    a < b
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function f(a: usize, b: int64): boolean {
    a < b
}

=== dir ===
function f(a: usize, b: int64): boolean {
/// @type.symbol symbol=f type=(usize, int64) => boolean
/// @type.symbol symbol=f.a source="a: usize" type=usize
/// @type.symbol symbol=f.b source="b: int64" type=int64

    a < b
    /// @resolution.name source=a target=f.a
    /// @resolution.rejected source="a < b"
    /// @resolution.place source=a placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=a root=f.a
    /// @resolution.name source=b target=f.b
    /// @resolution.place source=b placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=b root=f.b

}
"#,
        r#"
/// @diagnostic.error id=no-matching-operator message="operator '<' is not defined for 'usize' and 'int64'"
/// @diagnostic.label line=3 column=7 span="<" line_source="a < b"
"#,
    );
}

#[test]
fn test_assign_through_a_mutable_generic_slice_index() {
    let session = TestSession::single(
        r#"
function put<T>(destination: &[T], value: T): void {
    let lane: isize = 0;

    destination[lane] = value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function put<T, 'a>(destination: &'a [T], value: T): void {
    let lane: isize = 0;

    destination[lane] = value;
}

=== dir ===
function put<T>(destination: &[T], value: T): void {
/// @generic.template symbol=put parameters=(T, 'a)
/// @type.symbol symbol=put type=<T, put.'a>(&put.'a Slice<T>, T) => void
/// @type.symbol symbol=put.T source=T type=T
/// @type.symbol symbol=put.destination source="destination: &[T]" type=&put.'a Slice<T>
/// @resolution.name source=T target=put.T
/// @type.symbol symbol=put.value source="value: T" type=T
/// @resolution.name source=T target=put.T

    let lane: isize = 0;
    /// @type.symbol symbol=put.lane source=lane type=isize
    /// @resolution.pattern source=lane kind=binding target=put.lane

    destination[lane] = value;
    /// @resolution.name source=destination target=put.destination
    /// @resolution.place source=destination placement=put.'a lifetime=put.'a access="mutable"
    /// @resolution.access source=destination root=put.destination
    /// @resolution.pattern.assign source=destination[lane] kind=place
    /// @resolution.assignment source=destination[lane] write="indexSet#1(parameters=(isize, T), arguments=(provided(lane) as isize, supplied(0) as T), return=void, regions=(put.'a))" type=T
    /// @generic.instantiation id="indexSet#1<T, put.'a>" template=indexSet#1 arguments=(T, put.'a) owner=put
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="indexSet#1<T, put.'a>" template=indexSet#1 arguments=(T, put.'a)
    /// @generic.instance id="panic<\"bound0\" & \"local\">" template=panic arguments=("bound0" & "local")
    /// @generic.instance id="size<T, put.'a>" template=size arguments=(T, put.'a)
    /// @generic.instance id="sliceLength<T, put.'a>" template=sliceLength arguments=(T, put.'a)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id="unsafeSet<T, put.'a>" template=unsafeSet arguments=(T, put.'a)
    /// @resolution.name source=lane target=put.lane
    /// @resolution.place source=lane placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=lane root=put.lane
    /// @resolution.name source=value target=put.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=put.value

}
"#,
    );
}

#[test]
fn test_assign_through_a_mutable_int32_slice_index() {
    let session = TestSession::single(
        r#"
function put(destination: &[int32], value: int32): void {
    let lane: isize = 0;

    destination[lane] = value;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function put<'a>(destination: &'a [int32], value: int32): void {
    let lane: isize = 0;

    destination[lane] = value;
}

=== dir ===
function put(destination: &[int32], value: int32): void {
/// @generic.template symbol=put parameters=('a)
/// @type.symbol symbol=put type=<put.'a>(&put.'a Slice<int32>, int32) => void
/// @type.symbol symbol=put.destination source="destination: &[int32]" type=&put.'a Slice<int32>
/// @type.symbol symbol=put.value source="value: int32" type=int32

    let lane: isize = 0;
    /// @type.symbol symbol=put.lane source=lane type=isize
    /// @resolution.pattern source=lane kind=binding target=put.lane

    destination[lane] = value;
    /// @resolution.name source=destination target=put.destination
    /// @resolution.place source=destination placement=put.'a lifetime=put.'a access="mutable"
    /// @resolution.access source=destination root=put.destination
    /// @resolution.pattern.assign source=destination[lane] kind=place
    /// @resolution.assignment source=destination[lane] write="indexSet#1(parameters=(isize, int32), arguments=(provided(lane) as isize, supplied(0) as int32), return=void, regions=(put.'a))" type=int32
    /// @generic.instantiation id="indexSet#1<int32, put.'a>" template=indexSet#1 arguments=(int32, put.'a)
    /// @generic.instance id="Cast.truncate<isize, usize>" template=Cast.truncate arguments=(isize, usize)
    /// @generic.instance id="Cast.truncate<usize, isize>" template=Cast.truncate arguments=(usize, isize)
    /// @generic.instance id="indexSet#1<int32, put.'a>" template=indexSet#1 arguments=(int32, put.'a)
    /// @generic.instance id="panic<\"bound0\" & \"local\">" template=panic arguments=("bound0" & "local")
    /// @generic.instance id="size<int32, put.'a>" template=size arguments=(int32, put.'a)
    /// @generic.instance id="sliceLength<int32, put.'a>" template=sliceLength arguments=(int32, put.'a)
    /// @generic.instance id="truncateInt<isize, usize>" template=truncateInt arguments=(isize, usize)
    /// @generic.instance id="truncateInt<usize, isize>" template=truncateInt arguments=(usize, isize)
    /// @generic.instance id="unsafeSet<int32, put.'a>" template=unsafeSet arguments=(int32, put.'a)
    /// @generic.instance id=Slice<int32> template=Slice arguments=(int32)
    /// @resolution.name source=lane target=put.lane
    /// @resolution.place source=lane placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=lane root=put.lane
    /// @resolution.name source=value target=put.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=put.value

}
"#,
    );
}

/// Const parameters, const blocks, and const functions check together.
#[test]
fn test_check_const_parameter_block_and_function_declarations() {
    let session = TestSession::single(
        r#"struct Lane<const Width: usize> {
    const {}
    data: [uint8; Width];
}

const function double(value: usize): usize {
    return value * 2;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::none(),
        r#"
=== annotated ===
struct Lane<const Width: usize> {
    const {}
    data: [uint8; Width];
}

const function double(value: usize): usize {
    return value * 2;
}

=== dir ===
struct Lane<const Width: usize> {
    const {}
    data: [uint8; Width];
}

const function double(value: usize): usize {
    return value * 2;
}
"#,
        r#"
"#,
    );
}
