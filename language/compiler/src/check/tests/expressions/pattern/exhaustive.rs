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

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
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
} as T

=== checked ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(T#2, E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(T#2, E#2) value=Ok<T#2> | Err<E#2>
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
    /// @type.node type=T#3 | T#3
    /// @type.node source=outcome type=Outcome<T#3, E#3>
    /// @resolution.name source=outcome target=unwrapOr.outcome
    /// @generic.instance source=outcome id="Outcome<T#3, E#3>"

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instance source="Ok { value }" id=Ok<T#3>
        /// @type.symbol symbol=unwrapOr.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrapOr.value

        Err { error } => fallback
        /// @resolution.name source=Err target=Err
        /// @resolution.pattern source="Err { error }" kind=nominal_object target=Err instance=Err<E#3> fields={ Err.error }
        /// @generic.instance source="Err { error }" id=Err<E#3>
        /// @type.symbol symbol=unwrapOr.error source=error type=E#3
        /// @type.node source=fallback type=T#3
        /// @resolution.name source=fallback target=unwrapOr.fallback

    }
}

/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Err<E#3> template=Err arguments=(E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
"#,
        r#""#,
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

=== checked ===
struct Ok<T> {
/// @generic.template symbol=Ok parameters=(T#1)
/// @type.symbol symbol=Ok type=Ok
/// @definition.struct symbol=Ok template=(T#1)
/// @definition.field symbol=Ok.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Ok.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Ok.value source="value: T" type=T#1
    /// @resolution.name source=T target=Ok.T

}

struct Err<E> {
/// @generic.template symbol=Err parameters=(E#1)
/// @type.symbol symbol=Err type=Err
/// @definition.struct symbol=Err template=(E#1)
/// @definition.field symbol=Err.error source="error: E" key=error type=E#1
/// @type.symbol symbol=Err.E source=E type=E#1

    error: E;
    /// @type.symbol symbol=Err.error source="error: E" type=E#1
    /// @resolution.name source=E target=Err.E

}

newtype Outcome<T, E> = Ok<T> | Err<E>;
/// @generic.template symbol=Outcome parameters=(T#2, E#2)
/// @type.symbol symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" type=Outcome
/// @definition.newtype symbol=Outcome source="newtype Outcome<T, E> = Ok<T> | Err<E>" template=(T#2, E#2) value=Ok<T#2> | Err<E#2>
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
    /// @generic.instance source=outcome id="Outcome<T#3, E#3>"

        Ok { value } => value
        /// @resolution.name source=Ok target=Ok
        /// @resolution.pattern source="Ok { value }" kind=nominal_object target=Ok instance=Ok<T#3> fields={ Ok.value }
        /// @generic.instance source="Ok { value }" id=Ok<T#3>
        /// @type.symbol symbol=unwrap.value source=value type=T#3
        /// @type.node source=value type=T#3
        /// @resolution.name source=value target=unwrap.value

    }
}

/// @generic.instance id="Outcome<T#3, E#3>" template=Outcome arguments=(T#3, E#3)
/// @generic.instance id=Ok<T#3> template=Ok arguments=(T#3)
"#,
        r#"
/// @diagnostic.error code=EC403 message="match is not exhaustive: 'Err<E>' is not covered"
/// @diagnostic.label line=13 column=11 span="(outcome) {\n        Ok { value } => value\n    }" line_source="match (outcome) {"
"#,
    );
}

#[test]
fn test_tagged_variant_object_pattern_binds_the_payload() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Edge<T> =
    | { kind: "bounded"; limit: T }
    | { kind: "open" };

function limitOr<T>(edge: Edge<T>, fallback: T): T {
    match (edge) {
        Edge.Bounded { limit } => limit
        Edge.Open => fallback
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Edge<T> = { kind: "bounded"; limit: T } | { kind: "open" };

function limitOr<T>(edge: Edge<T>, fallback: T): T {
    match (edge) {
        Edge.Bounded { limit } => limit
        Edge.Open => fallback
    }
}

=== checked ===
@derive(Tagged)
/// @generic.template symbol=Edge parameters=(T#1)
/// @type.symbol symbol=Edge type=Edge
/// @type.symbol symbol=Edge.Bounded type=Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge template=(T#1) value={ kind: "bounded"; limit: T#1 } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded
/// @definition.variant symbol=Edge.Open key=Open
/// @resolution.name source=derive target=decorator.derive.derive

newtype Edge<T> =
/// @type.symbol symbol=Edge.T source=T type=T#1

    | { kind: "bounded"; limit: T }
    /// @resolution.name source=T target=Edge.T

    | { kind: "open" };

function limitOr<T>(edge: Edge<T>, fallback: T): T {
/// @generic.template symbol=limitOr parameters=(T#2)
/// @type.symbol symbol=limitOr type=<T#2>(Edge<T#2>, T#2) => T#2
/// @type.symbol symbol=limitOr.T source=T type=T#2
/// @type.symbol symbol=limitOr.edge source="edge: Edge<T>" type=Edge<T#2>
/// @resolution.name source=Edge target=Edge
/// @resolution.name source=T target=limitOr.T
/// @type.symbol symbol=limitOr.fallback source="fallback: T" type=T#2
/// @resolution.name source=T target=limitOr.T
/// @resolution.name source=T target=limitOr.T

    match (edge) {
    /// @type.node type=T#2
    /// @type.node source=edge type=Edge<T#2>
    /// @resolution.name source=edge target=limitOr.edge
    /// @generic.instance source=edge id=Edge<T#2>

        Edge.Bounded { limit } => limit
        /// @resolution.name source=Edge.Bounded target=Edge
        /// @resolution.pattern source="Edge.Bounded { limit }" kind=variant predicate="variant.tag(\"bounded\") is \"bounded\"" projection="variant.payload(Edge.Bounded<T#2>, { limit: T#2 })" payload=object fields={ limit }
        /// @generic.instance source="Edge.Bounded { limit }" id=Edge<T#2>
        /// @type.symbol symbol=limitOr.limit source=limit type=T#2
        /// @type.node source=limit type=T#2
        /// @resolution.name source=limit target=limitOr.limit

        Edge.Open => fallback
        /// @type.node source=Edge type=Edge
        /// @type.node source=Edge.Open type=Edge.Open
        /// @resolution.name source=Edge target=Edge
        /// @resolution.member source=Edge.Open receiver=Edge kind=symbol target=Edge.Open
        /// @resolution.pattern source=Edge.Open kind=variant predicate="variant.tag(\"open\") is \"open\"" projection="variant.payload(Edge.Open<T#2>, {})" fields=()
        /// @generic.instance source=Edge.Open id=Edge<T#1>
        /// @generic.instance source=Edge.Open id=Edge<T#2>
        /// @type.node source=fallback type=T#2
        /// @resolution.name source=fallback target=limitOr.fallback

    }
}

/// @generic.instance id=Edge<T#1> template=Edge arguments=(T#1)
/// @generic.instance id=Edge<T#2> template=Edge arguments=(T#2)
"#,
        r#""#,
    );
}

#[test]
fn test_tagged_variant_object_pattern_joins_payload_from_union_instances() {
    let session = TestSession::single(
        r#"
@derive(Tagged)
newtype Edge<T> =
    | { kind: "bounded"; limit: T }
    | { kind: "open" };

declare const edge: Edge<string> | Edge<int32>;

const value: string = match (edge) {
    Edge.Bounded { limit } => limit
    Edge.Open => ""
};
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
@derive(Tagged)
newtype Edge<T> = { kind: "bounded"; limit: T } | { kind: "open" };

declare const edge: Edge<string> | Edge<int32>;

const value: string = match (edge) {
    Edge.Bounded { limit } => limit
    Edge.Open => ""
};

=== checked ===
@derive(Tagged)
/// @generic.template symbol=Edge parameters=(T)
/// @type.symbol symbol=Edge type=Edge
/// @type.symbol symbol=Edge.Bounded type=Edge.Bounded
/// @type.symbol symbol=Edge.Open type=Edge.Open
/// @definition.newtype symbol=Edge template=(T) value={ kind: "bounded"; limit: T } | { kind: "open" }
/// @definition.variant symbol=Edge.Bounded key=Bounded
/// @definition.variant symbol=Edge.Open key=Open
/// @resolution.name source=derive target=decorator.derive.derive

newtype Edge<T> =
/// @type.symbol symbol=Edge.T source=T type=T

    | { kind: "bounded"; limit: T }
    /// @resolution.name source=T target=Edge.T

    | { kind: "open" };

declare const edge: Edge<string> | Edge<int32>;
/// @type.symbol symbol=edge source=edge type=Edge<string> | Edge<int32>
/// @resolution.name source=Edge target=Edge
/// @resolution.name source=Edge target=Edge

const value: string = match (edge) {
/// @type.symbol symbol=value source=value type=string
/// @type.node type=string | int32
/// @type.node source=edge type=Edge<string> | Edge<int32>
/// @resolution.name source=edge target=edge
/// @generic.instance source=edge id=Edge<int32>
/// @generic.instance source=edge id=Edge<string>

    Edge.Bounded { limit } => limit
    /// @resolution.name source=Edge.Bounded target=Edge
    /// @resolution.pattern source="Edge.Bounded { limit }" kind=variant predicate="variant.tag(\"bounded\") is \"bounded\"" projection="variant.payload(Edge.Bounded, { limit: string } | { limit: int32 })" payload=object fields={ limit }
    /// @type.symbol symbol=limit source=limit type=string | int32
    /// @type.node source=limit type=string | int32
    /// @resolution.name source=limit target=limit

    Edge.Open => ""
    /// @type.node source=Edge type=Edge
    /// @type.node source=Edge.Open type=Edge.Open
    /// @resolution.name source=Edge target=Edge
    /// @resolution.member source=Edge.Open receiver=Edge kind=symbol target=Edge.Open
    /// @resolution.pattern source=Edge.Open kind=variant predicate="variant.tag(\"open\") is \"open\"" projection="variant.payload(Edge.Open, {})" fields=()
    /// @generic.instance source=Edge.Open id=Edge<T>
    /// @type.node source="\"\"" type=""

};

/// @generic.instance id=Edge<T> template=Edge arguments=(T)
/// @generic.instance id=Edge<int32> template=Edge arguments=(int32)
/// @generic.instance id=Edge<string> template=Edge arguments=(string)
"#,
        r#"
/// @diagnostic.error code=EC200 message="type 'string | int32' is not assignable to type 'string'"
/// @diagnostic.label line=9 column=29 span="(edge) {\n    Edge.Bounded { limit } => limit\n    Edge.Open => \"\"\n}" line_source="const value: string = match (edge) {"
"#,
    );
}
