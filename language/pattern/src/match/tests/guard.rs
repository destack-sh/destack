use crate::tests::TestMatcher;

/// Match direct calls that resolve to the selected declaration.
#[test]
fn test_match_symbol_guard() {
    TestMatcher::new(
        "$CALLEE($VALUE)",
        r#"
function fetch(value: string): string {
    return value;
}

const request = fetch;

fetch("first");
request("second");
"#,
    )
    .guard("$CALLEE == fetch")
    .assert(
        r#"
function fetch(value: string): string {
    return value;
}

const request = fetch;

fetch("first");
^^^^^^^^^^^^^^ match CALLEE.node="fetch" VALUE.node="\"first\""
request("second");
"#,
    );
}

/// Reject the same structural call shape when its callee resolves elsewhere.
#[test]
fn test_reject_other_symbol() {
    TestMatcher::new(
        "$CALLEE($VALUE)",
        r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

fetch("first");
send("second");
"#,
    )
    .guard("$CALLEE == fetch")
    .assert(
        r#"
function fetch(value: string): string {
    return value;
}

function send(value: string): string {
    return value;
}

fetch("first");
^^^^^^^^^^^^^^ match CALLEE.node="fetch" VALUE.node="\"first\""
send("second");
"#,
    );
}

/// Resolve qualified predicate symbols through an imported module namespace.
#[test]
fn test_match_imported_namespace_symbol() {
    TestMatcher::new(
        "$CALLEE($VALUE)",
        r#"
import * as myPackage from "./package.tspp";

const request = myPackage.net.fetch;

myPackage.net.fetch("first");
request("second");
"#,
    )
    .file(
        "package.tspp",
        r#"
export * as net from "./net.tspp";
"#,
    )
    .file(
        "net.tspp",
        r#"
export function fetch(value: string): string {
    return value;
}
"#,
    )
    .guard("$CALLEE == myPackage.net.fetch")
    .assert(
        r#"
import * as myPackage from "./package.tspp";

const request = myPackage.net.fetch;

myPackage.net.fetch("first");
^^^^^^^^^^^^^^^^^^^^^^^^^^^^ match CALLEE.node="myPackage.net.fetch" VALUE.node="\"first\""
request("second");
"#,
    );
}

/// Match values whose final checked type is assignable to a scalar target.
#[test]
fn test_match_assignable_guard() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
function consume(value: string): void {}

const text = "hello";

consume(text);
consume("world");
"#,
    )
    .guard("$VALUE satisfies string")
    .assert(
        r#"
function consume(value: string): void {}

const text = "hello";

consume(text);
^^^^^^^^^^^^^ match VALUE.node="text"
consume("world");
^^^^^^^^^^^^^^^^ match VALUE.node="\"world\""
"#,
    );
}

/// Require every member of a checked union to satisfy the target union.
#[test]
fn test_match_union_assignable_guard() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
function consume(value: string | int32): void {}

function inspect(value: string | int32): void {
    consume(value);
}
"#,
    )
    .guard("$VALUE satisfies string | int32")
    .assert(
        r#"
function consume(value: string | int32): void {}

function inspect(value: string | int32): void {
    consume(value);
    ^^^^^^^^^^^^^^ match VALUE.node="value"
}
"#,
    );
}

/// Reject a union source when any member falls outside the target.
#[test]
fn test_reject_partial_union_assignability() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
function consume(value: string | int32): void {}

function inspect(value: string | int32): void {
    consume(value);
}
"#,
    )
    .guard("$VALUE satisfies string")
    .assert(
        r#"
function consume(value: string | int32): void {}

function inspect(value: string | int32): void {
    consume(value);
}
"#,
    );
}

/// Match a checked nominal reference against its lexical declaration.
#[test]
fn test_match_nominal_assignable_guard() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
class User {}

function consume(value: User): void {}

function inspect(user: User): void {
    consume(user);
}
"#,
    )
    .guard("$VALUE satisfies User")
    .assert(
        r#"
class User {}

function consume(value: User): void {}

function inspect(user: User): void {
    consume(user);
    ^^^^^^^^^^^^^ match VALUE.node="user"
}
"#,
    );
}

/// Relate generic arguments by the declaration's checked covariant parameter.
#[test]
fn test_match_covariant_generic_guard() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
struct Box<out T> {
    value: T;
}

function consume(value: unknown): void {}

declare const box: Box<"hello">;

consume(box);
"#,
    )
    .guard("$VALUE satisfies Box<string>")
    .assert(
        r#"
struct Box<out T> {
    value: T;
}

function consume(value: unknown): void {}

declare const box: Box<"hello">;

consume(box);
^^^^^^^^^^^^ match VALUE.node="box"
"#,
    );
}

/// Consume the covariant parameter variance committed by the checker.
#[test]
fn test_match_derived_generic_variance() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
struct Box<T> {
    value: T;
}

function consume(value: unknown): void {}

declare const box: Box<"hello">;

consume(box);
"#,
    )
    .guard("$VALUE satisfies Box<string>")
    .assert(
        r#"
struct Box<T> {
    value: T;
}

function consume(value: unknown): void {}

declare const box: Box<"hello">;

consume(box);
^^^^^^^^^^^^ match VALUE.node="box"
"#,
    );
}

/// Require both argument directions for a checked invariant parameter.
#[test]
fn test_reject_invariant_generic_widening() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
struct Cell<in out T> {
    value: T;
}

function consume(value: unknown): void {}

declare const cell: Cell<"hello">;

consume(cell);
"#,
    )
    .guard("$VALUE satisfies Cell<string>")
    .assert(
        r#"
struct Cell<in out T> {
    value: T;
}

function consume(value: unknown): void {}

declare const cell: Cell<"hello">;

consume(cell);
"#,
    );
}

/// Reverse the argument relation for a checked contravariant parameter.
#[test]
fn test_match_contravariant_generic_guard() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
interface Sink<in T> {
    write(value: T): void;
}

function consume(value: unknown): void {}

declare const sink: Sink<string>;

consume(sink);
"#,
    )
    .guard(r#"$VALUE satisfies Sink<"hello">"#)
    .assert(
        r#"
interface Sink<in T> {
    write(value: T): void;
}

function consume(value: unknown): void {}

declare const sink: Sink<string>;

consume(sink);
^^^^^^^^^^^^^ match VALUE.node="sink"
"#,
    );
}

/// Match values whose checked static scalar satisfies an ordered comparison.
#[test]
fn test_match_static_guard() {
    TestMatcher::new(
        "consume($COUNT)",
        r#"
function consume(count: int32): void {}

const count = 1;

consume(count);
consume(0);
"#,
    )
    .guard("$COUNT > 0")
    .assert(
        r#"
function consume(count: int32): void {}

const count = 1;

consume(count);
^^^^^^^^^^^^^^ match COUNT.node="count"
consume(0);
"#,
    );
}

/// Resolve a predicate constant in the candidate root's lexical scope.
#[test]
fn test_match_static_reference_guard() {
    TestMatcher::new(
        "consume($COUNT)",
        r#"
function consume(count: int32): void {}

const minimum = 0;
const count = 1;

consume(count);
consume(0);
"#,
    )
    .guard("$COUNT > minimum")
    .assert(
        r#"
function consume(count: int32): void {}

const minimum = 0;
const count = 1;

consume(count);
^^^^^^^^^^^^^^ match COUNT.node="count"
consume(0);
"#,
    );
}

/// Compose symbol, type, and static-value conditions in one guard.
#[test]
fn test_match_composed_guard() {
    TestMatcher::new(
        "$CALLEE($VALUE, $COUNT)",
        r#"
function consume(value: string, count: int32): void {}

const text = "hello";
const count = 1;

consume(text, count);
consume(text, 0);
"#,
    )
    .guard("$CALLEE == consume && $VALUE satisfies string && $COUNT > 0")
    .assert(
        r#"
function consume(value: string, count: int32): void {}

const text = "hello";
const count = 1;

consume(text, count);
^^^^^^^^^^^^^^^^^^^^ match CALLEE.node="consume" VALUE.node="text" COUNT.node="count"
consume(text, 0);
"#,
    );
}

/// Evaluate name scalars through boolean negation and disjunction.
#[test]
fn test_match_name_guard() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
user.name
user.email
"#,
    )
    .guard("!false && ($MEMBER == \"name\" || false)")
    .assert(
        r#"
user.name
^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
user.email
"#,
    );
}

/// Treat values from different equality domains as unequal.
#[test]
fn test_match_heterogeneous_inequality() {
    TestMatcher::new(
        "$CALLEE($VALUE)",
        r#"
function fetch(value: string): string {
    return value;
}

fetch("value");
"#,
    )
    .guard("$CALLEE != null")
    .assert(
        r#"
function fetch(value: string): string {
    return value;
}

fetch("value");
^^^^^^^^^^^^^^ match CALLEE.node="fetch" VALUE.node="\"value\""
"#,
    );
}

/// Evaluate every equality operator over one name binding.
#[test]
fn test_match_equality_operators() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
user.name
user.email
"#,
    )
    .guard(
        r#"$MEMBER == "name" && $MEMBER === "name" && $MEMBER != "email" && $MEMBER !== "email""#,
    )
    .assert(
        r#"
user.name
^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
user.email
"#,
    );
}

/// Evaluate every ordered comparison over checked static values.
#[test]
fn test_match_order_operators() {
    TestMatcher::new(
        "consume($COUNT)",
        r#"
function consume(count: int32): void {}

consume(0);
consume(1);
consume(2);
consume(3);
"#,
    )
    .guard("$COUNT > 0 && $COUNT >= 1 && $COUNT < 3 && $COUNT <= 2")
    .assert(
        r#"
function consume(count: int32): void {}

consume(0);
consume(1);
^^^^^^^^^^ match COUNT.node="1"
consume(2);
^^^^^^^^^^ match COUNT.node="2"
consume(3);
"#,
    );
}

/// Evaluate a direct metavariable condition by binding presence.
#[test]
fn test_match_binding_condition() {
    TestMatcher::new(
        "consume($VALUE)",
        r#"
consume(first)
consume(second)
"#,
    )
    .guard("$VALUE")
    .assert(
        r#"
consume(first)
^^^^^^^^^^^^^^ match VALUE.node="first"
consume(second)
^^^^^^^^^^^^^^^ match VALUE.node="second"
"#,
    );
}

/// Conjoin separately authored predicates in tree order.
#[test]
fn test_match_multiple_predicates() {
    TestMatcher::new(
        "$OBJECT.$MEMBER",
        r#"
user.name
user.email
"#,
    )
    .guard("$MEMBER != \"email\"")
    .guard("$MEMBER == \"name\"")
    .assert(
        r#"
user.name
^^^^^^^^^ match OBJECT.node="user" MEMBER.name="name"
user.email
"#,
    );
}
