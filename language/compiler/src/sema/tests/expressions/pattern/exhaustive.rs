use crate::tests::{DirRows, TestSession};

#[test]
fn test_newtype_union_matches_exhaustively_through_its_backing() {
    let session = TestSession::single(
        r#"
struct Ok<T> {
    value: T;
}

struct Err<E> {
    error: E;
}

newtype Outcome<T, E> = Ok<T> | Err<E>;

function unwrapOr<T, E>(outcome: Outcome<T, E>, fallback: T): T {
    match (outcome) {
        Ok { value } => value
        Err { error } => fallback
    }
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Ok<out T> {
    value: T;
}

struct Err<out E> {
    error: E;
}

newtype Outcome<out T, out E> = Ok<T> | Err<E>;

function unwrapOr<T, E>(outcome: Outcome<T, E>, fallback: T): T {
    match (outcome) {
        Ok { value } => value
        Err { error } => fallback
    }
}

=== dir ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(out T#2, out E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @generic.instance id=Err<E#2> template=Err arguments=(E#2)
/// @generic.instance id=Ok<T#2> template=Ok arguments=(T#2)
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) backing=Ok<T#2> | Err<E#2> constructors=[<T#2, E#2>(Ok<T#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Err<E#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Ok<T#2> | Err<E#2>) => Outcome<T#2, E#2>]
/// @type.symbol symbol=Outcome.T source=T type=T#2
/// @type.symbol symbol=Outcome.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Outcome.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Outcome.E

function unwrapOr<T, E>(outcome: Outcome<T, E>, fallback: T): T {
/// @generic.template symbol=unwrapOr parameters=(T#3, E#3)
/// @type.symbol symbol=unwrapOr type=<T#3, E#3>(Outcome<T#3, E#3>, T#3) => T#3
/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
/// @type.symbol symbol=unwrapOr.T source=T type=T#3
/// @type.symbol symbol=unwrapOr.E source=E type=E#3
/// @type.symbol symbol=unwrapOr.outcome source="outcome: Outcome<T, E>" type=Outcome<T#3, E#3>
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=T target=unwrapOr.T
/// @resolution.name source=E target=unwrapOr.E
/// @type.symbol symbol=unwrapOr.fallback source="fallback: T" type=T#3
/// @resolution.name source=T target=unwrapOr.T
/// @resolution.name source=T target=unwrapOr.T

    match (outcome) {
    /// @type.node type=T#3
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @type.node source=outcome type=Outcome<T#3, E#3>
    /// @resolution.name source=outcome target=unwrapOr.outcome
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=unwrapOr.outcome

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Outcome, Ok<T#3> | Err<E#3>), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=unwrapOr
        /// @generic.instantiation id=Ok<T#3> template=Ok arguments=(T#3) owner=unwrapOr
        /// @type.symbol symbol=unwrapOr.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrapOr.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=unwrapOr.value

        Err { error } => fallback
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error }" kind=nominal_object adjustments=(newtype.payload(Outcome, Ok<T#3> | Err<E#3>), union.payload(Ok<T#3> | Err<E#3>, Err<E#3>, Err<E#3>)) target=Err instance=Err<E#3> fields={ Err.error }
        /// @generic.instantiation id=Err<E#3> template=Err arguments=(E#3) owner=unwrapOr
        /// @type.symbol symbol=unwrapOr.error source=error type=E#3
        /// @type.node source=fallback type=T#3
        /// @resolution.name source=fallback target=unwrapOr.fallback
        /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fallback root=unwrapOr.fallback

    }
}
"#,
    );
}

#[test]
fn test_newtype_union_match_reports_the_uncovered_arm() {
    let session = TestSession::single(
        r#"
struct Ok<T> {
    value: T;
}

struct Err<E> {
    error: E;
}

newtype Outcome<T, E> = Ok<T> | Err<E>;

function unwrap<T, E>(outcome: Outcome<T, E>): T {
    match (outcome) {
        Ok { value } => value
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
struct Ok<out T> {
    value: T;
}

struct Err<out E> {
    error: E;
}

newtype Outcome<out T, out E> = Ok<T> | Err<E>;

function unwrap<T, E>(outcome: Outcome<T, E>): T {
    match (outcome) {
        Ok { value } => value
    }
}

=== dir ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(out T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(out T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(out E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(out E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(out T#2, out E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(out T#2, out E#2) backing=Ok<T#2> | Err<E#2> constructors=[<T#2, E#2>(Ok<T#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Err<E#2>) => Outcome<T#2, E#2>, <T#2, E#2>(Ok<T#2> | Err<E#2>) => Outcome<T#2, E#2>]
/// @type.symbol symbol=Outcome.T source=T type=T#2
/// @type.symbol symbol=Outcome.E source=E type=E#2
/// @resolution.name source=Ok target=Ok
/// @resolution.name source=T target=Outcome.T
/// @resolution.name source=Err target=Err
/// @resolution.name source=E target=Outcome.E

function unwrap<T, E>(outcome: Outcome<T, E>): T {
/// @generic.template symbol=unwrap parameters=(T#3, E#3)
/// @type.symbol symbol=unwrap type=<T#3, E#3>(Outcome<T#3, E#3>) => T#3
/// @type.symbol symbol=unwrap.T source=T type=T#3
/// @type.symbol symbol=unwrap.E source=E type=E#3
/// @type.symbol symbol=unwrap.outcome source="outcome: Outcome<T, E>" type=Outcome<T#3, E#3>
/// @resolution.name source=Outcome target=Outcome
/// @resolution.name source=T target=unwrap.T
/// @resolution.name source=E target=unwrap.E
/// @resolution.name source=T target=unwrap.T

    match (outcome) {
    /// @type.node type=T#3
    /// @resolution.coverage exhaustive=false disjoint=true
    /// @type.node source=outcome type=Outcome<T#3, E#3>
    /// @resolution.name source=outcome target=unwrap.outcome
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=unwrap.outcome

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object adjustments=(newtype.payload(Outcome, Ok<T#3> | Err<E#3>), union.payload(Ok<T#3> | Err<E#3>, Ok<T#3>, Ok<T#3>)) target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instantiation id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3) owner=unwrap
        /// @generic.instantiation id=Ok<T#3> template=Ok arguments=(T#3) owner=unwrap
        /// @type.symbol symbol=unwrap.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=value root=unwrap.value

    }
}
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: 'Err<E>' is not covered"
/// @diagnostic.label line=13 column=5 span="match" line_source="match (outcome) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}

/// Tuple arms that split one finite field between them cover the whole tuple.
#[test]
fn test_tuple_match_covers_through_combined_arms() {
    let session = TestSession::single(
        r#"
function finish(value: (int32, boolean)): int32 {
    return match (value) {
        (left, true) => left
        (right, false) => right
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function finish(value: (int32, boolean)): int32 {
    return match (value) {
        (left, true) => left
        (right, false) => right
    };
}

=== dir ===
function finish(value: (int32, boolean)): int32 {
/// @type.symbol symbol=finish type=((int32, boolean)) => int32
/// @type.symbol symbol=finish.value source="value: (int32, boolean)" type=(int32, boolean)

    return match (value) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=value target=finish.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=finish.value

        (left, true) => left
        /// @resolution.pattern source=(left, true) kind=tuple fields=(finish.left, true)
        /// @type.symbol symbol=finish.left source=left type=int32
        /// @resolution.pattern source=left kind=binding target=finish.left
        /// @resolution.pattern source=true kind=literal value=true
        /// @resolution.name source=left target=finish.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=left root=finish.left

        (right, false) => right
        /// @resolution.pattern source=(right, false) kind=tuple fields=(finish.right, false)
        /// @type.symbol symbol=finish.right source=right type=int32
        /// @resolution.pattern source=right kind=binding target=finish.right
        /// @resolution.pattern source=false kind=literal value=false
        /// @resolution.name source=right target=finish.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=right root=finish.right

    };
}
"#,
        r#"

"#,
    );
}

/// A tuple match names the missing combination when one finite case stays uncovered.
#[test]
fn test_tuple_match_reports_the_uncovered_combination() {
    let session = TestSession::single(
        r#"
function finish(value: (int32, boolean)): int32 {
    return match (value) {
        (left, true) => left
        (right, true) => right
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function finish(value: (int32, boolean)): int32 {
    return match (value) {
        (left, true) => left
        (right, true) => right
    };
}

=== dir ===
function finish(value: (int32, boolean)): int32 {
/// @type.symbol symbol=finish type=((int32, boolean)) => int32
/// @type.symbol symbol=finish.value source="value: (int32, boolean)" type=(int32, boolean)

    return match (value) {
    /// @resolution.coverage exhaustive=false disjoint=false
    /// @resolution.name source=value target=finish.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=finish.value

        (left, true) => left
        /// @resolution.pattern source=(left, true) kind=tuple fields=(finish.left, true)
        /// @type.symbol symbol=finish.left source=left type=int32
        /// @resolution.pattern source=left kind=binding target=finish.left
        /// @resolution.pattern source=true kind=literal value=true
        /// @resolution.name source=left target=finish.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=left root=finish.left

        (right, true) => right
        /// @resolution.pattern source=(right, true) kind=tuple fields=(finish.right, true)
        /// @type.symbol symbol=finish.right source=right type=int32
        /// @resolution.pattern source=right kind=binding target=finish.right
        /// @resolution.pattern source=true kind=literal value=true
        /// @resolution.name source=right target=finish.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=right root=finish.right

    };
}
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '(int32, false)' is not covered"
/// @diagnostic.label line=3 column=12 span="match" line_source="return match (value) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}

/// Nested tuple arms cover the outer tuple once their fields expand into the columns.
#[test]
fn test_nested_tuple_match_covers_through_expanded_columns() {
    let session = TestSession::single(
        r#"
function pick(value: ((int32, boolean), string)): int32 {
    return match (value) {
        ((left, true), first) => left
        ((right, false), second) => right
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function pick(value: ((int32, boolean), string)): int32 {
    return match (value) {
        ((left, true), first) => left
        ((right, false), second) => right
    };
}

=== dir ===
function pick(value: ((int32, boolean), string)): int32 {
/// @type.symbol symbol=pick type=(((int32, boolean), string)) => int32
/// @type.symbol symbol=pick.value source="value: ((int32, boolean), string)" type=((int32, boolean), string)

    return match (value) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=value target=pick.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pick.value

        ((left, true), first) => left
        /// @resolution.pattern source=((left, true), first) kind=tuple fields=(pattern, pick.first)
        /// @resolution.pattern source=(left, true) kind=tuple fields=(pick.left, true)
        /// @type.symbol symbol=pick.left source=left type=int32
        /// @resolution.pattern source=left kind=binding target=pick.left
        /// @resolution.pattern source=true kind=literal value=true
        /// @type.symbol symbol=pick.first source=first type=string
        /// @resolution.pattern source=first kind=binding target=pick.first
        /// @resolution.name source=left target=pick.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=left root=pick.left

        ((right, false), second) => right
        /// @resolution.pattern source=((right, false), second) kind=tuple fields=(pattern, pick.second)
        /// @resolution.pattern source=(right, false) kind=tuple fields=(pick.right, false)
        /// @type.symbol symbol=pick.right source=right type=int32
        /// @resolution.pattern source=right kind=binding target=pick.right
        /// @resolution.pattern source=false kind=literal value=false
        /// @type.symbol symbol=pick.second source=second type=string
        /// @resolution.pattern source=second kind=binding target=pick.second
        /// @resolution.name source=right target=pick.right
        /// @resolution.place source=right placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=right root=pick.right

    };
}
"#,
        r#"

"#,
    );
}

/// Tuple arms cover a union-typed field case by case.
#[test]
fn test_tuple_match_covers_a_union_element_field() {
    let session = TestSession::single(
        r#"
function label(value: (int32, "on" | "off")): int32 {
    return match (value) {
        (first, "on") => first
        (second, "off") => second
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function label(value: (int32, "on" | "off")): int32 {
    return match (value) {
        (first, "on") => first
        (second, "off") => second
    };
}

=== dir ===
function label(value: (int32, "on" | "off")): int32 {
/// @type.symbol symbol=label type=((int32, "on" | "off")) => int32
/// @type.symbol symbol=label.value source="value: (int32, \"on\" | \"off\")" type=(int32, "on" | "off")

    return match (value) {
    /// @resolution.coverage exhaustive=true disjoint=true
    /// @resolution.name source=value target=label.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=label.value

        (first, "on") => first
        /// @resolution.pattern source=(first, "on") kind=tuple fields=(label.first, "on")
        /// @type.symbol symbol=label.first source=first type=int32
        /// @resolution.pattern source=first kind=binding target=label.first
        /// @resolution.pattern source="\"on\"" kind=literal value="on"
        /// @resolution.name source=first target=label.first
        /// @resolution.place source=first placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=first root=label.first

        (second, "off") => second
        /// @resolution.pattern source=(second, "off") kind=tuple fields=(label.second, "off")
        /// @type.symbol symbol=label.second source=second type=int32
        /// @resolution.pattern source=second kind=binding target=label.second
        /// @resolution.pattern source="\"off\"" kind=literal value="off"
        /// @resolution.name source=second target=label.second
        /// @resolution.place source=second placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=second root=label.second

    };
}
"#,
        r#"

"#,
    );
}

/// A nested tuple failure names the outer element whole in its witness.
#[test]
fn test_nested_tuple_match_reports_the_outer_uncovered_element() {
    let session = TestSession::single(
        r#"
function pick(value: ((int32, boolean), string)): int32 {
    return match (value) {
        ((left, true), first) => left
    };
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
function pick(value: ((int32, boolean), string)): int32 {
    return match (value) {
        ((left, true), first) => left
    };
}

=== dir ===
function pick(value: ((int32, boolean), string)): int32 {
/// @type.symbol symbol=pick type=(((int32, boolean), string)) => int32
/// @type.symbol symbol=pick.value source="value: ((int32, boolean), string)" type=((int32, boolean), string)

    return match (value) {
    /// @resolution.coverage exhaustive=false disjoint=true
    /// @resolution.name source=value target=pick.value
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=pick.value

        ((left, true), first) => left
        /// @resolution.pattern source=((left, true), first) kind=tuple fields=(pattern, pick.first)
        /// @resolution.pattern source=(left, true) kind=tuple fields=(pick.left, true)
        /// @type.symbol symbol=pick.left source=left type=int32
        /// @resolution.pattern source=left kind=binding target=pick.left
        /// @resolution.pattern source=true kind=literal value=true
        /// @type.symbol symbol=pick.first source=first type=string
        /// @resolution.pattern source=first kind=binding target=pick.first
        /// @resolution.name source=left target=pick.left
        /// @resolution.place source=left placement="local" lifetime="frame" access="immutable"
        /// @resolution.access source=left root=pick.left

    };
}
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: '((int32, boolean), string)' is not covered"
/// @diagnostic.label line=3 column=12 span="match" line_source="return match (value) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}
