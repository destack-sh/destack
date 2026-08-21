use crate::tests::{DirRows, TestSession};

#[test]
fn test_transfer_owned_operands_into_managed_operator_parameters() {
    let session = TestSession::single(
        r#"
declare function fresh(): ^string;
declare const text: string;

const managedLeft = text + fresh();
const ownedLeft = fresh() + text;
const bothOwned = fresh() + fresh();
const same = fresh() == text;

function append(target: string): string {
    let result = target;
    result += fresh();
    return result;
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_coercion(),
        r#"
=== annotated ===
declare function fresh(): ^string;
declare const text: string;

const managedLeft: string = (text + fresh()) as string;
const ownedLeft: string = (fresh() + text) as string;
const bothOwned: string = (fresh() + fresh()) as string;
const same: boolean = (fresh() as string) == text;

function append(target: string): string {
    let result: string = target;
    (result += fresh()) as string;
    return result;
}

=== dir ===
declare function fresh(): ^string;
/// @type.symbol symbol=fresh source="declare function fresh(): ^string" type=() => Owned<string>

declare const text: string;
/// @type.symbol symbol=text source=text type=string
/// @resolution.pattern source=text kind=binding target=text

const managedLeft = text + fresh();
/// @type.symbol symbol=managedLeft source=managedLeft type=string
/// @resolution.pattern source=managedLeft kind=binding target=managedLeft
/// @resolution.name source=text target=text
/// @resolution.operator source="text + fresh()" type=Owned<string> operator="+" kind=call parameters=(string) arguments=(provided(fresh()) as string) return=Owned<string> kind=symbol target=string.string.add receiver=string adjustments=(borrow(&'static readonly string))
/// @resolution.place source=text placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=text root=text
/// @coercion.node source="text + fresh()" from=Owned<string> adjustments=[{ kind: manage, target: string }] origin=implicit
/// @resolution.name source=fresh target=fresh
/// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh

const ownedLeft = fresh() + text;
/// @type.symbol symbol=ownedLeft source=ownedLeft type=string
/// @resolution.pattern source=ownedLeft kind=binding target=ownedLeft
/// @resolution.name source=fresh target=fresh
/// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh
/// @resolution.operator source="fresh() + text" type=Owned<string> operator="+" kind=call parameters=(string) arguments=(provided(text) as string) return=Owned<string> kind=symbol target=string.string.add receiver=Owned<string> adjustments=(borrow(&'frame readonly Owned<string>))
/// @coercion.node source="fresh() + text" from=Owned<string> adjustments=[{ kind: manage, target: string }] origin=implicit
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=text root=text

const bothOwned = fresh() + fresh();
/// @type.symbol symbol=bothOwned source=bothOwned type=string
/// @resolution.pattern source=bothOwned kind=binding target=bothOwned
/// @resolution.name source=fresh target=fresh
/// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh
/// @resolution.operator source="fresh() + fresh()" type=Owned<string> operator="+" kind=call parameters=(string) arguments=(provided(fresh()) as string) return=Owned<string> kind=symbol target=string.string.add receiver=Owned<string> adjustments=(borrow(&'frame readonly Owned<string>))
/// @coercion.node source="fresh() + fresh()" from=Owned<string> adjustments=[{ kind: manage, target: string }] origin=implicit
/// @resolution.name source=fresh target=fresh
/// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh

const same = fresh() == text;
/// @type.symbol symbol=same source=same type=boolean
/// @resolution.pattern source=same kind=binding target=same
/// @resolution.name source=fresh target=fresh
/// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh
/// @resolution.operator source="fresh() == text" type=boolean operator="==" kind=builtin operands=[fresh() as string families=(string), text as string families=(string)]
/// @coercion.node source=fresh() from=Owned<string> adjustments=[{ kind: manage, target: string }] origin=implicit
/// @resolution.name source=text target=text
/// @resolution.place source=text placement="local" lifetime="static" access="exclusive"
/// @resolution.access source=text root=text

function append(target: string): string {
/// @type.symbol symbol=append type=(string) => string
/// @type.symbol symbol=append.target source="target: string" type=string

    let result = target;
    /// @type.symbol symbol=append.result source=result type=string
    /// @resolution.pattern source=result kind=binding target=append.result
    /// @resolution.name source=target target=append.target
    /// @resolution.access source=target root=append.target

    result += fresh();
    /// @resolution.name source=result target=append.result
    /// @resolution.operator source="result += fresh()" type=Owned<string> operator="+" kind=call parameters=(string) arguments=(provided(fresh()) as string) return=Owned<string> kind=symbol target=string.string.add receiver=string adjustments=(borrow(&'frame readonly string))
    /// @resolution.pattern.assign source=result kind=place
    /// @resolution.assignment source=result read=binding(append.result) write=binding(append.result) type=string
    /// @resolution.access source=result root=append.result
    /// @coercion.node source="result += fresh()" from=Owned<string> adjustments=[{ kind: manage, target: string }] origin=implicit
    /// @resolution.name source=fresh target=fresh
    /// @resolution.call source=fresh() parameters=() return=Owned<string> kind=symbol target=fresh

    return result;
    /// @resolution.name source=result target=append.result
    /// @resolution.place source=result placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=result root=append.result

}
"#,
        r#"
"#,
    );
}
