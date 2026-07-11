use crate::tests::{DirRows, TestSession};

#[test]
fn test_instanceof_reduces_to_boolean() {
    let session = TestSession::single(
        r#"
class User {}
declare const value: unknown;

const ok = value instanceof User;
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}
declare const value: unknown;

const ok: boolean = value instanceof User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

const ok = value instanceof User;
/// @type.symbol symbol=ok source=ok type=boolean
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @resolution.guard source="value instanceof User" kind=instanceof value=unknown target=User target_type=User predicate="unknown is subtype(User)" narrowed=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User
"#,
    );
}

#[test]
fn test_instanceof_narrows_positive_branch_to_class_arm() {
    let session = TestSession::single(
        r#"
declare class User {
    name: string;
}

declare class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
    value.name satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class User {
    name: string;
}

declare class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
    value.name satisfies string;
}

=== checked ===
declare class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

declare class Team {
/// @type.symbol symbol=Team type=Team
/// @definition.class symbol=Team
/// @definition.field symbol=Team.title source="title: string" key=title type=string

    title: string;
    /// @type.symbol symbol=Team.title source="title: string" type=string

}

declare const value: User | Team;
/// @type.symbol symbol=value source=value type=User | Team
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

if (value instanceof User) {
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=User | Team
/// @resolution.name source=value target=value
/// @resolution.guard source="value instanceof User" kind=instanceof value=User | Team target=User target_type=User predicate="User | Team is subtype(User)" narrowed=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User

    value.name satisfies string;
    /// @type.node source="value.name satisfies string" type=string
    /// @type.node source=value type=User
    /// @type.node source=value.name type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.name receiver=User kind=symbol target=User.name

}
"#,
    );
}

#[test]
fn test_instanceof_narrows_negative_branch_by_removing_class_arm() {
    let session = TestSession::single(
        r#"
declare class User {
    name: string;
}

declare class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
} else {
    value.title satisfies string;
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
declare class User {
    name: string;
}

declare class Team {
    title: string;
}

declare const value: User | Team;

if (value instanceof User) {
} else {
    value.title satisfies string;
}

=== checked ===
declare class User {
/// @type.symbol symbol=User type=User
/// @definition.class symbol=User
/// @definition.field symbol=User.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=User.name source="name: string" type=string

}

declare class Team {
/// @type.symbol symbol=Team type=Team
/// @definition.class symbol=Team
/// @definition.field symbol=Team.title source="title: string" key=title type=string

    title: string;
    /// @type.symbol symbol=Team.title source="title: string" type=string

}

declare const value: User | Team;
/// @type.symbol symbol=value source=value type=User | Team
/// @resolution.name source=User target=User
/// @resolution.name source=Team target=Team

if (value instanceof User) {
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=User | Team
/// @resolution.name source=value target=value
/// @resolution.guard source="value instanceof User" kind=instanceof value=User | Team target=User target_type=User predicate="User | Team is subtype(User)" narrowed=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User

} else {
    value.title satisfies string;
    /// @type.node source="value.title satisfies string" type=string
    /// @type.node source=value type=Team
    /// @type.node source=value.title type=string
    /// @resolution.name source=value target=value
    /// @resolution.member source=value.title receiver=Team kind=symbol target=Team.title

}
"#,
    );
}

#[test]
fn test_instanceof_rejects_non_class_right_hand_side() {
    let session = TestSession::single(
        r#"
interface Named {
    name: string;
}

declare const value: unknown;

const ok = value instanceof Named;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Named {
    name: string;
}

declare const value: unknown;

const ok = value instanceof Named;

=== checked ===
interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named
/// @definition.field symbol=Named.name source="name: string" key=name type=string

    name: string;
    /// @type.symbol symbol=Named.name source="name: string" type=string

}

declare const value: unknown;
/// @type.symbol symbol=value source=value type=unknown

const ok = value instanceof Named;
/// @type.symbol symbol=ok source=ok type=<error>
/// @type.node source="value instanceof Named" type=<error>
/// @type.node source=value type=unknown
/// @resolution.name source=value target=value
/// @type.node source=Named type=Named
/// @resolution.name source=Named target=Named
"#,
        r#"
/// @diagnostic.error code=EC317 message="right-hand side of 'instanceof' must be a class"
/// @diagnostic.label line=8 column=29 span="Named" line_source="const ok = value instanceof Named;"
"#,
    );
}

#[test]
fn test_instanceof_rejects_impossible_primitive_check() {
    let session = TestSession::single(
        r#"
class User {}
declare const value: string;

const ok = value instanceof User;
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class User {}
declare const value: string;

const ok: boolean = value instanceof User;

=== checked ===
class User {}
/// @type.symbol symbol=User source="class User {}" type=User
/// @definition.class symbol=User source="class User {}"

declare const value: string;
/// @type.symbol symbol=value source=value type=string

const ok = value instanceof User;
/// @type.symbol symbol=ok source=ok type=boolean
/// @type.node source="value instanceof User" type=boolean
/// @type.node source=value type=string
/// @resolution.name source=value target=value
/// @resolution.guard source="value instanceof User" kind=instanceof value=string target=User target_type=User predicate="string is subtype(User)" narrowed=User
/// @type.node source=User type=User
/// @resolution.name source=User target=User
"#,
        r#"
/// @diagnostic.error code=EC318 message="type 'string' can never be an instance of 'User'"
/// @diagnostic.label line=5 column=12 span="value" line_source="const ok = value instanceof User;"
"#,
    );
}

#[test]
fn test_instanceof_narrows_generic_value_to_intersection() {
    let session = TestSession::single(
        r#"
class Deferred<T> {
    then(callback: (value: T) => void): void {}
}

function adopt<T>(value: T): void {
    if (value instanceof Deferred) {
        value.then((value) => {});
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Deferred<out T> {
    then(callback: (arg0: T) => void): void {}
}

function adopt<T>(value: T): void {
    if (value instanceof Deferred) {
        value.then((value): void => {});
    }
}

=== checked ===
class Deferred<T> {
/// @generic.template symbol=Deferred parameters=(out T#1)
/// @type.symbol symbol=Deferred type=Deferred
/// @definition.class symbol=Deferred template=(out T#1)
/// @definition.method symbol=Deferred.then source="then(callback: (value: T) => void): void {}" slot=then type=(this: Deferred<T#1>, Function<(T#1,), void>) => void
/// @type.symbol symbol=Deferred.T source=T type=T#1

    then(callback: (value: T) => void): void {}
    /// @type.symbol symbol=Deferred.then source="then(callback: (value: T) => void): void {}" type=(this: Deferred<T#1>, Function<(T#1,), void>) => void
    /// @type.symbol symbol=Deferred.then.callback source="callback: (value: T) => void" type=Function<(T#1,), void>
    /// @resolution.name source=T target=Deferred.T

}

function adopt<T>(value: T): void {
/// @generic.template symbol=adopt parameters=(T#2)
/// @type.symbol symbol=adopt type=<T#2>(T#2) => void
/// @type.symbol symbol=adopt.T source=T type=T#2
/// @type.symbol symbol=adopt.value source="value: T" type=T#2
/// @resolution.name source=T target=adopt.T

    if (value instanceof Deferred) {
    /// @type.node source="value instanceof Deferred" type=boolean
    /// @type.node source=value type=T#2
    /// @resolution.name source=value target=adopt.value
    /// @resolution.guard source="value instanceof Deferred" kind=instanceof value=T#2 target=Deferred target_type=Deferred<*> predicate="T#2 is subtype(Deferred<*>)" narrowed=Deferred<*>
    /// @type.node source=Deferred type=Deferred
    /// @resolution.name source=Deferred target=Deferred

        value.then((value) => {});
        /// @type.node source="value.then((value) => {})" type=void
        /// @type.node source=value type=T#2 & Deferred<*>
        /// @type.node source=value.then type=(this: Deferred<*>, Function<(*,), void>) => void
        /// @resolution.name source=value target=adopt.value
        /// @resolution.member source=value.then receiver=T#2 & Deferred<*> kind=symbol target=Deferred.then
        /// @resolution.call source="value.then((value) => {})" parameters=(Function<(*,), void>) arguments=(provided((value) => {}) as Function<(*,), void>) return=void kind=symbol target=Deferred.then receiver=T#2 & Deferred<*> instance=Deferred<*>.then
        /// @generic.instance source="value.then((value) => {})" id=Deferred<*>.then
        /// @generic.instance source=value id=Deferred<*>
        /// @generic.instance source=value.then id=Deferred<*>
        /// @type.symbol symbol=adopt.symbol10 source="(value) => {}" type=Function<(*,), void>
        /// @type.node source="(value) => {}" type=Function<(*,), void>
        /// @type.symbol symbol=adopt.symbol10.value source=value type=*

    }
}

/// @generic.instance id=Deferred<*> template=Deferred arguments=(*)
/// @generic.instance id=Deferred<*>.then template=Deferred.then arguments=(*)
/// @generic.instance id=Deferred<T#1> template=Deferred arguments=(T#1)
"#,
    );
}

#[test]
fn test_instanceof_narrows_generic_union_arm() {
    let session = TestSession::single(
        r#"
class Deferred<T> {
    then(callback: (value: T) => void): void {}
}

function adopt<T>(value: T | Deferred<T>): void {
    if (value instanceof Deferred) {
        value.then((value) => {});
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
class Deferred<out T> {
    then(callback: (arg0: T) => void): void {}
}

function adopt<T>(value: T | Deferred<T>): void {
    if (value instanceof Deferred) {
        value.then((value): void => {});
    }
}

=== checked ===
class Deferred<T> {
/// @generic.template symbol=Deferred parameters=(out T#1)
/// @type.symbol symbol=Deferred type=Deferred
/// @definition.class symbol=Deferred template=(out T#1)
/// @definition.method symbol=Deferred.then source="then(callback: (value: T) => void): void {}" slot=then type=(this: Deferred<T#1>, Function<(T#1,), void>) => void
/// @type.symbol symbol=Deferred.T source=T type=T#1

    then(callback: (value: T) => void): void {}
    /// @type.symbol symbol=Deferred.then source="then(callback: (value: T) => void): void {}" type=(this: Deferred<T#1>, Function<(T#1,), void>) => void
    /// @type.symbol symbol=Deferred.then.callback source="callback: (value: T) => void" type=Function<(T#1,), void>
    /// @resolution.name source=T target=Deferred.T

}

function adopt<T>(value: T | Deferred<T>): void {
/// @generic.template symbol=adopt parameters=(T#2)
/// @type.symbol symbol=adopt type=<T#2>(T#2 | Deferred<T#2>) => void
/// @type.symbol symbol=adopt.T source=T type=T#2
/// @type.symbol symbol=adopt.value source="value: T | Deferred<T>" type=T#2 | Deferred<T#2>
/// @resolution.name source=T target=adopt.T
/// @resolution.name source=Deferred target=Deferred
/// @resolution.name source=T target=adopt.T

    if (value instanceof Deferred) {
    /// @type.node source="value instanceof Deferred" type=boolean
    /// @type.node source=value type=T#2 | Deferred<T#2>
    /// @resolution.name source=value target=adopt.value
    /// @resolution.guard source="value instanceof Deferred" kind=instanceof value=T#2 | Deferred<T#2> target=Deferred target_type=Deferred<*> predicate="T#2 | Deferred<T#2> is subtype(Deferred<*>)" narrowed=Deferred<*>
    /// @generic.instance source=value id=Deferred<T#2>
    /// @type.node source=Deferred type=Deferred
    /// @resolution.name source=Deferred target=Deferred

        value.then((value) => {});
        /// @type.node source="value.then((value) => {})" type=void
        /// @type.node source=value type=T#2 & Deferred<*>
        /// @type.node source=value.then type=(this: Deferred<*>, Function<(*,), void>) => void
        /// @resolution.name source=value target=adopt.value
        /// @resolution.member source=value.then receiver=T#2 & Deferred<*> kind=symbol target=Deferred.then
        /// @resolution.call source="value.then((value) => {})" parameters=(Function<(*,), void>) arguments=(provided((value) => {}) as Function<(*,), void>) return=void kind=symbol target=Deferred.then receiver=T#2 & Deferred<*> instance=Deferred<*>.then
        /// @generic.instance source="value.then((value) => {})" id=Deferred<*>.then
        /// @generic.instance source=value id=Deferred<*>
        /// @generic.instance source=value.then id=Deferred<*>
        /// @type.symbol symbol=adopt.symbol10 source="(value) => {}" type=Function<(*,), void>
        /// @type.node source="(value) => {}" type=Function<(*,), void>
        /// @type.symbol symbol=adopt.symbol10.value source=value type=*

    }
}

/// @generic.instance id=Deferred<*> template=Deferred arguments=(*)
/// @generic.instance id=Deferred<*>.then template=Deferred.then arguments=(*)
/// @generic.instance id=Deferred<T#1> template=Deferred arguments=(T#1)
/// @generic.instance id=Deferred<T#2> template=Deferred arguments=(T#2)
"#,
    );
}
