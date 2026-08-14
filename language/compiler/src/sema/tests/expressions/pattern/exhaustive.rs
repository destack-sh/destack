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

    session.assert_dir_checked(
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

=== checked ===
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

function unwrapOr<T, E>(outcome: Outcome<T, E>, fallback: T): T {
/// @generic.template symbol=unwrapOr parameters=(T#3, E#3)
/// @type.symbol symbol=unwrapOr type=<T#3, E#3>(Outcome<T#3, E#3>, T#3) => T#3
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
    /// @type.node source=outcome type=Outcome<T#3, E#3>
    /// @resolution.name source=outcome target=unwrapOr.outcome
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=unwrapOr.outcome
    /// @generic.instance source=outcome id="Outcome<T#3, E#3>"

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instance source="Ok { value }" id=Ok<T#3>
        /// @type.symbol symbol=unwrapOr.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrapOr.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=unwrapOr.value

        Err { error } => fallback
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error }" kind=nominal_object target=Err instance=Err<E#3> fields={ Err.error }
        /// @generic.instance source="Err { error }" id=Err<E#3>
        /// @type.symbol symbol=unwrapOr.error source=error type=E#3
        /// @type.node source=fallback type=T#3
        /// @resolution.name source=fallback target=unwrapOr.fallback
        /// @resolution.place source=fallback placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=fallback root=unwrapOr.fallback

    }
}

/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
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

    session.assert_dir_checked_and_diagnostics(
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

=== checked ===
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
    /// @type.node source=outcome type=Outcome<T#3, E#3>
    /// @resolution.name source=outcome target=unwrap.outcome
    /// @resolution.place source=outcome placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=outcome root=unwrap.outcome
    /// @generic.instance source=outcome id="Outcome<T#3, E#3>"

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instance source="Ok { value }" id=Ok<T#3>
        /// @type.symbol symbol=unwrap.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrap.value
        /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=value root=unwrap.value

    }
}

/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
"#,
        r#"
/// @diagnostic.error id=non-exhaustive-pattern message="match is not exhaustive: 'Err<E>' is not covered"
/// @diagnostic.label line=13 column=5 span="match (outcome) {\n        Ok { value } => value\n    }" line_source="match (outcome) {"
/// @diagnostic.help message="cover the remaining values or add a wildcard '_' arm"
"#,
    );
}
