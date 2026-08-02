use crate::tests::{DirRows, TestSession};

#[test]
fn test_duplicate_interface_extension_reports_conflict() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

extension of User implements Show {
    show(): string {
        return "debug";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

extension of User implements Show {
    show(): string {
        return "debug";
    }
}

=== checked ===
newtype interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: this) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: this) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show#1 slot=show type=<show#1.'a>(this: &show#1.'a exclusive this) => string
/// @definition.implementation symbol=<module>#2 requirement=Show.show target=show#1
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show#1 parent=template#1 parameters=('a)
    /// @type.symbol symbol=show#1 type=<show#1.'a>(this: &show#1.'a exclusive this) => string

        return "user";
    }
}

extension of User implements Show {
/// @definition.extension symbol=<module>#3 form=local target=User
/// @definition.implements symbol=<module>#3 source=Show target=Show
/// @definition.method symbol=show#2 slot=show type=<show#2.'a>(this: &show#2.'a exclusive this) => string
/// @definition.implementation symbol=<module>#3 requirement=Show.show target=show#2
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show#2 parent=template#2 parameters=('a)
    /// @type.symbol symbol=show#2 type=<show#2.'a>(this: &show#2.'a exclusive this) => string

        return "debug";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Show' for type 'User'"
/// @diagnostic.label line=14 column=14 span="User" line_source="extension of User implements Show {"
/// @diagnostic.related line=8 column=14 span="User" line_source="extension of User implements Show {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_extension_implements_clause_requires_members() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {}

=== checked ===
newtype interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: this) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: this) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {}
/// @definition.extension symbol=<module>#2 source="extension of User implements Show {}" form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=8 column=30 span="Show" line_source="extension of User implements Show {}"
"#,
    );
}

#[test]
fn test_extension_implements_clause_requires_inherited_members() {
    let session = TestSession::single(
        r#"
newtype interface Named {
    name(): string;
}

newtype interface Show extends Named {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Named {
    name(): string;
}

newtype interface Show extends Named {
    show(): string;
}

struct User {}

extension of User implements Show {
    show(): string {
        return "user";
    }
}

=== checked ===
newtype interface Named {
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named nominal=true
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: this) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: this) => string

}

newtype interface Show extends Named {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
/// @definition.extends symbol=Show source=Named target=Named
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: this) => string
/// @resolution.name source=Named target=Named

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: this) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a exclusive this) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show parent=template#2 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a exclusive this) => string

        return "user";
    }
}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=12 column=30 span="Show" line_source="extension of User implements Show {"
"#,
    );
}

#[test]
fn test_extension_implementation_satisfies_inherited_interface_relation() {
    let session = TestSession::single(
        r#"
newtype interface PartialEqual<T = this> {
    equal(other: T): boolean;
}

newtype interface Equal<T = this> extends PartialEqual<T> {}

struct Badge {}

extension of Badge implements Equal<Badge> {
    equal(other: Badge): boolean {
        true
    }
}

function compare<T: PartialEqual<T>>(left: T, right: T): boolean {
    left.equal(right)
}

const ok = compare(Badge {}, Badge {});
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface PartialEqual<in T = this> {
    equal(other: T): boolean;
}

newtype interface Equal<in T = this> extends PartialEqual<T> {}

struct Badge {}

extension of Badge implements Equal<Badge> {
    equal(other: Badge): boolean {
        true
    }
}

function compare<T: PartialEqual<T>>(left: T, right: T): boolean {
    left.equal<T>(right)
}

const ok: boolean = compare<Badge>(Badge {}, Badge {});

=== checked ===
newtype interface PartialEqual<T = this> {
/// @generic.template symbol=PartialEqual parameters=(in T#1 = this)
/// @type.symbol symbol=PartialEqual type=PartialEqual
/// @definition.interface symbol=PartialEqual template=(in T#1 = this) nominal=true
/// @definition.where symbol=PartialEqual relation=satisfies left=this right=PartialEqual<T#1>
/// @definition.method symbol=PartialEqual.equal source="equal(other: T): boolean" slot=equal type=(this: this, T#1) => boolean
/// @type.symbol symbol=PartialEqual.T source="T = this" type=T#1

    equal(other: T): boolean;
    /// @type.symbol symbol=PartialEqual.equal source="equal(other: T): boolean" type=(this: this, T#1) => boolean
    /// @type.symbol symbol=PartialEqual.equal.other source="other: T" type=T#1
    /// @resolution.name source=T target=PartialEqual.T

}

newtype interface Equal<T = this> extends PartialEqual<T> {}
/// @generic.template symbol=Equal parameters=(in T#2 = this)
/// @type.symbol symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal
/// @definition.interface symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" template=(in T#2 = this) nominal=true
/// @definition.where symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" relation=satisfies left=this right=Equal<T#2>
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

struct Badge {}
/// @type.symbol symbol=Badge source="struct Badge {}" type=Badge
/// @definition.struct symbol=Badge source="struct Badge {}"

extension of Badge implements Equal<Badge> {
/// @definition.extension symbol=<module>#2 form=local target=Badge
/// @definition.implements symbol=<module>#2 source=Equal<Badge> target=Equal<Badge>
/// @definition.method symbol=equal slot=equal type=<equal.'a>(this: &equal.'a exclusive this, Badge) => boolean
/// @definition.implementation symbol=<module>#2 requirement=PartialEqual.equal target=equal
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=Badge target=Badge

    equal(other: Badge): boolean {
    /// @generic.template symbol=equal parent=template#2 parameters=('a)
    /// @type.symbol symbol=equal type=<equal.'a>(this: &equal.'a exclusive this, Badge) => boolean
    /// @type.symbol symbol=equal.other source="other: Badge" type=Badge
    /// @resolution.name source=Badge target=Badge

        true
    }
}

function compare<T: PartialEqual<T>>(left: T, right: T): boolean {
/// @generic.template symbol=compare parameters=(T#3: PartialEqual<T#3>)
/// @type.symbol symbol=compare type=<T#3: PartialEqual<T#3>>(T#3, T#3) => boolean
/// @type.symbol symbol=compare.T source="T: PartialEqual<T>" type=T#3
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=compare.T
/// @type.symbol symbol=compare.left source="left: T" type=T#3
/// @resolution.name source=T target=compare.T
/// @type.symbol symbol=compare.right source="right: T" type=T#3
/// @resolution.name source=T target=compare.T

    left.equal(right)
    /// @resolution.name source=left target=compare.left
    /// @resolution.member source=left.equal receiver=T#3 type=(this: T#3, T#3) => boolean kind=symbol target_receiver=T#3 target=PartialEqual.equal
    /// @resolution.call source=left.equal(right) parameters=(T#3) arguments=(provided(right) as T#3) return=boolean kind=symbol target=PartialEqual.equal receiver=T#3 instance=PartialEqual<T#3>.equal
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=compare.left
    /// @generic.instance source=left.equal(right) id=PartialEqual<T#3>.equal
    /// @resolution.name source=right target=compare.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=compare.right

}

const ok = compare(Badge {}, Badge {});
/// @type.symbol symbol=ok source=ok type=boolean
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=compare target=compare
/// @resolution.call source="compare(Badge {}, Badge {})" parameters=(Badge, Badge) arguments=(provided(Badge {}) as Badge, provided(Badge {}) as Badge) return=boolean kind=symbol target=compare instance=compare<Badge>
/// @generic.instance source="compare(Badge {}, Badge {})" id=compare<Badge>
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=Badge target=Badge

/// @generic.instance id=PartialEqual<T#3>.equal template=PartialEqual.equal arguments=(T#3)
/// @generic.instance id=compare<Badge> template=compare arguments=(Badge)
"#,
    );
}

#[test]
fn test_extension_implements_clause_requires_interface() {
    let session = TestSession::single(
        r#"
struct User {}
struct NotInterface {}

extension of User implements NotInterface {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
struct NotInterface {}

extension of User implements NotInterface {}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

struct NotInterface {}
/// @type.symbol symbol=NotInterface source="struct NotInterface {}" type=NotInterface
/// @definition.struct symbol=NotInterface source="struct NotInterface {}"

extension of User implements NotInterface {}
/// @definition.extension symbol=<module>#2 source="extension of User implements NotInterface {}" form=local target=User
/// @resolution.name source=User target=User
/// @resolution.name source=NotInterface target=NotInterface
"#,
        r#"
/// @diagnostic.error id=implementation-target-not-interface message="type 'User' can only implement interfaces, not 'NotInterface'"
/// @diagnostic.label line=5 column=30 span="NotInterface" line_source="extension of User implements NotInterface {}"
"#,
    );
}

#[test]
fn test_extension_implements_rejects_type_alias_interface() {
    let session = TestSession::single(
        r#"
struct User {}
interface Show {
    show(): string;
}
type Alias = Show;

extension of User implements Alias {
    show(): string {
        return "";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
interface Show {
    show(): string;
}
type Alias = Show;

extension of User implements Alias {
    show(): string {
        return "";
    }
}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: this) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: this) => string

}
type Alias = Show;
/// @type.symbol symbol=Alias source="type Alias = Show" type=Show
/// @definition.type symbol=Alias source="type Alias = Show" value=Show
/// @resolution.name source=Show target=Show

extension of User implements Alias {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a exclusive this) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Alias target=Alias

    show(): string {
    /// @generic.template symbol=show parent=template#1 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a exclusive this) => string

        return "";
    }
}
"#,
        r#"
/// @diagnostic.error id=implementation-target-not-interface message="type 'User' can only implement interfaces, not 'Alias'"
/// @diagnostic.label line=8 column=30 span="Alias" line_source="extension of User implements Alias {"
"#,
    );
}

#[test]
fn test_extension_implements_rejects_union_interface() {
    let session = TestSession::single(
        r#"
struct User {}
interface Show {
    show(): string;
}
interface Debug {
    debug(): string;
}

extension of User implements Show | Debug {
    show(): string {
        return "";
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
interface Show {
    show(): string;
}
interface Debug {
    debug(): string;
}

extension of User implements Show | Debug {
    show(): string {
        return "";
    }
}

=== checked ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

interface Show {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: this) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: this) => string

}
interface Debug {
/// @type.symbol symbol=Debug type=Debug
/// @definition.interface symbol=Debug
/// @definition.method symbol=Debug.debug source="debug(): string" slot=debug type=(this: this) => string

    debug(): string;
    /// @type.symbol symbol=Debug.debug source="debug(): string" type=(this: this) => string

}

extension of User implements Show | Debug {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a exclusive this) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
/// @resolution.name source=Debug target=Debug

    show(): string {
    /// @generic.template symbol=show parent=template#2 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a exclusive this) => string

        return "";
    }
}
"#,
        r#"
/// @diagnostic.error id=implementation-target-not-interface message="type 'User' can only implement interfaces, not 'Show | Debug'"
/// @diagnostic.label line=10 column=35 span="|" line_source="extension of User implements Show | Debug {"
"#,
    );
}

#[test]
fn test_primitive_extension_projects_its_associated_type() {
    let session = TestSession::single(
        r#"
interface Doubling {
    type Output;

    double(): this.Output;
}

extension of int32 implements Doubling {
    type Output = int32;

    double(): this.Output {
        todo("double")
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Doubling {
    type Output;

    double(): this.Output;
}

extension of int32 implements Doubling {
    type Output = int32;

    double(): this.Output {
        todo("double" as string | undefined)
    }
}

=== checked ===
interface Doubling {
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling
/// @definition.associated.type symbol=Doubling.Output source="type Output" key=Output
/// @definition.method symbol=Doubling.double source="double(): this.Output" slot=double type=(this: this) => this.Output

    type Output;

    double(): this.Output;
    /// @type.symbol symbol=Doubling.double source="double(): this.Output" type=(this: this) => this.Output

}

extension of int32 implements Doubling {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Doubling target="Doubling<type Output = int32>"
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=double slot=double type=<double.'a>(this: &double.'a exclusive this) => this.Output
/// @definition.implementation symbol=<module>#2 requirement=Doubling.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=Doubling.double target=double
/// @resolution.name source=Doubling target=Doubling

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    double(): this.Output {
    /// @generic.template symbol=double parent=template#1 parameters=('a)
    /// @type.symbol symbol=double type=<double.'a>(this: &double.'a exclusive this) => this.Output

        todo("double")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"double\")" parameters=(string | undefined) arguments=(provided("double") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}
"#);
}

#[test]
fn test_extension_implements_inherited_interface_with_associated_argument() {
    let session = TestSession::single(
        r#"
interface Source<T> {
    static from(value: T): this;
}

interface Carrier extends Source<this.Error> {
    type Error;
}

newtype Result<T, E> = T | E;

extension<T, E> of Result<T, E> implements Source<E>, Carrier {
    type Error = E;

    static from(value: E): Result<T, E> {
        todo("from")
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Source<in T> {
    static from(value: T): this;
}

interface Carrier extends Source<this.Error> {
    type Error;
}

newtype Result<out T, out E> = T | E;

extension<T, E> of Result<T, E> implements Source<E>, Carrier {
    type Error = E;

    static from(value: E): Result<T, E> {
        todo("from" as string | undefined)
    }
}

=== checked ===
interface Source<T> {
/// @generic.template symbol=Source parameters=(in T#1)
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source template=(in T#1)
/// @definition.where symbol=Source relation=satisfies left=this right=Source<T#1>
/// @definition.method symbol=Source.from source="static from(value: T): this" slot=from static=true type=(T#1) => this
/// @type.symbol symbol=Source.T source=T type=T#1

    static from(value: T): this;
    /// @type.symbol symbol=Source.from source="static from(value: T): this" type=(T#1) => this
    /// @type.symbol symbol=Source.from.value source="value: T" type=T#1
    /// @resolution.name source=T target=Source.T

}

interface Carrier extends Source<this.Error> {
/// @type.symbol symbol=Carrier type=Carrier
/// @definition.interface symbol=Carrier
/// @definition.extends symbol=Carrier source=Source<this.Error> target=Source<this.Error>
/// @definition.associated.type symbol=Carrier.Error source="type Error" key=Error
/// @resolution.name source=Source target=Source

    type Error;
}

newtype Result<T, E> = T | E;
/// @generic.template symbol=Result parameters=(out T#2, out E#1)
/// @type.symbol symbol=Result source="newtype Result<T, E> = T | E" type=Result
/// @definition.newtype symbol=Result source="newtype Result<T, E> = T | E" template=(out T#2, out E#1) backing=T#2 | E#1
/// @type.symbol symbol=Result.T source=T type=T#2
/// @type.symbol symbol=Result.E source=E type=E#1
/// @resolution.name source=T target=Result.T
/// @resolution.name source=E target=Result.E

extension<T, E> of Result<T, E> implements Source<E>, Carrier {
/// @generic.template symbol=<module>#2 parameters=(T#3, E#2)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#3, E#2>
/// @definition.implements symbol=<module>#2 source=Carrier target="Carrier<type Error = E#2>"
/// @definition.implements symbol=<module>#2 source=Source<E> target=Source<E#2>
/// @definition.associated.type symbol=Error source="type Error = E" key=Error value=E#2
/// @definition.method symbol=from slot=from static=true type=(E#2) => Result<T#3, E#2>
/// @definition.implementation symbol=<module>#2 requirement=Carrier.Error target=Error
/// @definition.implementation symbol=<module>#2 requirement=Source.from target=from
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=E source=E type=E#2
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E
/// @resolution.name source=Source target=Source
/// @resolution.name source=E target=E
/// @resolution.name source=Carrier target=Carrier

    type Error = E;
    /// @type.symbol symbol=Error source="type Error = E" type=E#2
    /// @resolution.name source=E target=E

    static from(value: E): Result<T, E> {
    /// @type.symbol symbol=from type=(E#2) => Result<T#3, E#2>
    /// @type.symbol symbol=from.value source="value: E" type=E#2
    /// @resolution.name source=E target=E
    /// @resolution.name source=Result target=Result
    /// @resolution.name source=T target=T
    /// @resolution.name source=E target=E

        todo("from")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"from\")" parameters=(string | undefined) arguments=(provided("from") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

/// @generic.instance id="Result<T#3, E#2>" template=Result arguments=(T#3, E#2)
"#,
    );
}

#[test]
fn test_borrowed_receiver_satisfies_interface_method() {
    let session = TestSession::single(
        r#"
interface Halving {
    type Output;

    halve(): this.Output;
}

extension of int32 implements Halving {
    type Output = int32;

    halve(&readonly this): this.Output {
        todo("halve")
    }
}
"#,
    );

    session.assert_dir_checked("main.ds", DirRows::checked(), r#"
=== annotated ===
interface Halving {
    type Output;

    halve(): this.Output;
}

extension of int32 implements Halving {
    type Output = int32;

    halve(&readonly this): this.Output {
        todo("halve" as string | undefined)
    }
}

=== checked ===
interface Halving {
/// @type.symbol symbol=Halving type=Halving
/// @definition.interface symbol=Halving
/// @definition.associated.type symbol=Halving.Output source="type Output" key=Output
/// @definition.method symbol=Halving.halve source="halve(): this.Output" slot=halve type=(this: this) => this.Output

    type Output;

    halve(): this.Output;
    /// @type.symbol symbol=Halving.halve source="halve(): this.Output" type=(this: this) => this.Output

}

extension of int32 implements Halving {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Halving target="Halving<type Output = int32>"
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=halve slot=halve type=<halve.'a>(this: &halve.'a readonly this) => this.Output
/// @definition.implementation symbol=<module>#2 requirement=Halving.Output target=Output
/// @definition.implementation symbol=<module>#2 requirement=Halving.halve target=halve
/// @resolution.name source=Halving target=Halving

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    halve(&readonly this): this.Output {
    /// @generic.template symbol=halve parent=template#1 parameters=('a)
    /// @type.symbol symbol=halve type=<halve.'a>(this: &halve.'a readonly this) => this.Output
    /// @type.symbol symbol=halve.this source="&readonly this" type=&halve.'a readonly this

        todo("halve")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"halve\")" parameters=(string | undefined) arguments=(provided("halve") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}
"#);
}

#[test]
fn test_ambiguous_associated_types_qualify_by_interface() {
    let session = TestSession::single(
        r#"
interface Reading {
    type Output;

    read(): this.Output;
}

interface Writing {
    type Output;

    write(): this.Output;
}

struct Cell {
    value: int32;
}

extension of Cell implements Reading {
    type Output = int32;

    read(): this.Output {
        todo("read")
    }
}

extension of Cell implements Writing {
    type Output = float64;

    write(): this.Output {
        todo("write")
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Reading {
    type Output;

    read(): this.Output;
}

interface Writing {
    type Output;

    write(): this.Output;
}

struct Cell {
    value: int32;
}

extension of Cell implements Reading {
    type Output = int32;

    read(): this.Output {
        todo("read" as string | undefined)
    }
}

extension of Cell implements Writing {
    type Output = float64;

    write(): this.Output {
        todo("write" as string | undefined)
    }
}

=== checked ===
interface Reading {
/// @type.symbol symbol=Reading type=Reading
/// @definition.interface symbol=Reading
/// @definition.associated.type symbol=Reading.Output source="type Output" key=Output
/// @definition.method symbol=Reading.read source="read(): this.Output" slot=read type=(this: this) => this.Output

    type Output;

    read(): this.Output;
    /// @type.symbol symbol=Reading.read source="read(): this.Output" type=(this: this) => this.Output

}

interface Writing {
/// @type.symbol symbol=Writing type=Writing
/// @definition.interface symbol=Writing
/// @definition.associated.type symbol=Writing.Output source="type Output" key=Output
/// @definition.method symbol=Writing.write source="write(): this.Output" slot=write type=(this: this) => this.Output

    type Output;

    write(): this.Output;
    /// @type.symbol symbol=Writing.write source="write(): this.Output" type=(this: this) => this.Output

}

struct Cell {
/// @type.symbol symbol=Cell type=Cell
/// @definition.struct symbol=Cell
/// @definition.field symbol=Cell.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Cell.value source="value: int32" type=int32

}

extension of Cell implements Reading {
/// @definition.extension symbol=<module>#2 form=local target=Cell
/// @definition.implements symbol=<module>#2 source=Reading target="Reading<type Output = int32>"
/// @definition.associated.type symbol=Output#1 source="type Output = int32" key=Output value=int32
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a exclusive this) => this.Output
/// @definition.implementation symbol=<module>#2 requirement=Reading.Output target=Output#1
/// @definition.implementation symbol=<module>#2 requirement=Reading.read target=read
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Reading target=Reading

    type Output = int32;
    /// @type.symbol symbol=Output#1 source="type Output = int32" type=int32

    read(): this.Output {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a exclusive this) => this.Output

        todo("read")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"read\")" parameters=(string | undefined) arguments=(provided("read") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

extension of Cell implements Writing {
/// @definition.extension symbol=<module>#3 form=local target=Cell
/// @definition.implements symbol=<module>#3 source=Writing target="Writing<type Output = float64>"
/// @definition.associated.type symbol=Output#2 source="type Output = float64" key=Output value=float64
/// @definition.method symbol=write slot=write type=<write.'a>(this: &write.'a exclusive this) => this.Output
/// @definition.implementation symbol=<module>#3 requirement=Writing.Output target=Output#2
/// @definition.implementation symbol=<module>#3 requirement=Writing.write target=write
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Writing target=Writing

    type Output = float64;
    /// @type.symbol symbol=Output#2 source="type Output = float64" type=float64

    write(): this.Output {
    /// @generic.template symbol=write parent=template#3 parameters=('a)
    /// @type.symbol symbol=write type=<write.'a>(this: &write.'a exclusive this) => this.Output

        todo("write")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"write\")" parameters=(string | undefined) arguments=(provided("write") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}
"#,
    );
}
#[test]
fn test_extension_implements_one_interface_at_many_instantiations() {
    let session = TestSession::single(
        r#"
interface Emits<in out T> {}

struct Channel {}

extension of Channel implements Emits<int32>, Emits<string> {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Emits<in out T> {}

struct Channel {}

extension of Channel implements Emits<int32>, Emits<string> {}

=== checked ===
interface Emits<in out T> {}
/// @generic.template symbol=Emits parameters=(in out T)
/// @type.symbol symbol=Emits source="interface Emits<in out T> {}" type=Emits
/// @definition.interface symbol=Emits source="interface Emits<in out T> {}" template=(in out T)
/// @definition.where symbol=Emits source="interface Emits<in out T> {}" relation=satisfies left=this right=Emits<T>
/// @type.symbol symbol=Emits.T source="in out T" type=T

struct Channel {}
/// @type.symbol symbol=Channel source="struct Channel {}" type=Channel
/// @definition.struct symbol=Channel source="struct Channel {}"

extension of Channel implements Emits<int32>, Emits<string> {}
/// @definition.extension symbol=<module>#2 source="extension of Channel implements Emits<int32>, Emits<string> {}" form=local target=Channel
/// @definition.implements symbol=<module>#2 source=Emits<int32> target=Emits<int32>
/// @definition.implements symbol=<module>#2 source=Emits<string> target=Emits<string>
/// @resolution.name source=Channel target=Channel
/// @resolution.name source=Emits target=Emits
/// @resolution.name source=Emits target=Emits
"#,
        r#""#,
    );
}

#[test]
fn test_generic_member_binds_its_parameter_to_satisfy_the_interface() {
    let session = TestSession::single(
        r#"
interface Eq<T> {
    equals(other: &readonly T): boolean;
}

interface Has<T> {
    has(value: &readonly T): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T: Eq<T>> of Pack<T> implements Has<T> {
    has<Q: Eq<Q>>(value: &readonly Q): boolean {
        false
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Eq<in T> {
    equals(other: &'a readonly T): boolean;
}

interface Has<in T> {
    has(value: &'a readonly T): boolean;
}

struct Pack<out T> {
    value: T;
}

export extension<T: Eq<T>> of Pack<T> implements Has<T> {
    has<Q: Eq<Q>>(value: &'a readonly Q): boolean {
        false
    }
}

=== checked ===
interface Eq<T> {
/// @generic.template symbol=Eq parameters=(in T#1)
/// @type.symbol symbol=Eq type=Eq
/// @definition.interface symbol=Eq template=(in T#1)
/// @definition.where symbol=Eq relation=satisfies left=this right=Eq<T#1>
/// @definition.method symbol=Eq.equals source="equals(other: &readonly T): boolean" slot=equals type=<Eq.equals.'a>(this: this, &Eq.equals.'a readonly T#1) => boolean
/// @type.symbol symbol=Eq.T source=T type=T#1

    equals(other: &readonly T): boolean;
    /// @generic.template symbol=Eq.equals parent=template#0 parameters=('a)
    /// @type.symbol symbol=Eq.equals source="equals(other: &readonly T): boolean" type=<Eq.equals.'a>(this: this, &Eq.equals.'a readonly T#1) => boolean
    /// @type.symbol symbol=Eq.equals.other source="other: &readonly T" type=&Eq.equals.'a readonly T#1
    /// @resolution.name source=T target=Eq.T

}

interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#2)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#2)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#2>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(this: this, &Has.has.'a readonly T#2) => boolean
/// @type.symbol symbol=Has.T source=T type=T#2

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(this: this, &Has.has.'a readonly T#2) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=&Has.has.'a readonly T#2
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#3)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#3)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#3
/// @type.symbol symbol=Pack.T source=T type=T#3

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#3
    /// @resolution.name source=T target=Pack.T

}

export extension<T: Eq<T>> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#4: Eq<T#4>)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#4>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has<T#4>
/// @definition.method symbol=has slot=has type=<Q: Eq<Q>, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a readonly Q) => boolean
/// @definition.implementation symbol=<module>#2 requirement=Has.has target=has
/// @type.symbol symbol=T source="T: Eq<T>" type=T#4
/// @resolution.name source=Eq target=Eq
/// @resolution.name source=T target=T
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Eq<Q>>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=(Q: Eq<Q>, 'a, 'b)
    /// @type.symbol symbol=has type=<Q: Eq<Q>, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a readonly Q) => boolean
    /// @type.symbol symbol=has.Q source="Q: Eq<Q>" type=Q
    /// @resolution.name source=Eq target=Eq
    /// @resolution.name source=Q target=has.Q
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=&has.'a readonly Q
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}
"#,
        r#""#,
    );
}

#[test]
fn test_generic_member_with_an_unsatisfied_bound_rejects_the_interface() {
    let session = TestSession::single(
        r#"
interface Marker {}

interface Has<T> {
    has(value: &readonly T): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q: Marker>(value: &readonly Q): boolean {
        false
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Marker {}

interface Has<in T> {
    has(value: &'a readonly T): boolean;
}

struct Pack<out T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q: Marker>(value: &'a readonly Q): boolean {
        false
    }
}

=== checked ===
interface Marker {}
/// @type.symbol symbol=Marker source="interface Marker {}" type=Marker
/// @definition.interface symbol=Marker source="interface Marker {}"

interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#1)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#1)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(this: this, &Has.has.'a readonly T#1) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(this: this, &Has.has.'a readonly T#1) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=&Has.has.'a readonly T#1
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#2)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#2)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Pack.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#2
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#3>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has<T#3>
/// @definition.method symbol=has slot=has type=<Q: Marker, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a readonly Q) => boolean
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Marker>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=(Q: Marker, 'a, 'b)
    /// @type.symbol symbol=has type=<Q: Marker, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a readonly Q) => boolean
    /// @type.symbol symbol=has.Q source="Q: Marker" type=Q
    /// @resolution.name source=Marker target=Marker
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=&has.'a readonly Q
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}
"#,
        r#"/// @diagnostic.error id=interface-not-implemented message="type 'Pack<T>' does not implement interface 'Has<T>'"
/// @diagnostic.label line=12 column=43 span="Has" line_source="export extension<T> of Pack<T> implements Has<T> {"
"#,
    );
}

#[test]
fn test_generic_member_with_a_stronger_access_rejects_the_interface() {
    let session = TestSession::single(
        r#"
interface Has<T> {
    has(value: &readonly T): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q>(value: &exclusive Q): boolean {
        false
    }
}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked().with_reference_types(),
        r#"
=== annotated ===
interface Has<in T> {
    has(value: &'a readonly T): boolean;
}

struct Pack<out T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q>(value: &'a exclusive Q): boolean {
        false
    }
}

=== checked ===
interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#1)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#1)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(this: this, &Has.has.'a readonly T#1) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#0 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(this: this, &Has.has.'a readonly T#1) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=&Has.has.'a readonly T#1
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(out T#2)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(out T#2)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Pack.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#2
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#3>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has<T#3>
/// @definition.method symbol=has slot=has type=<Q, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a exclusive Q) => boolean
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q>(value: &exclusive Q): boolean {
    /// @generic.template symbol=has parent=template#2 parameters=(Q, 'a, 'b)
    /// @type.symbol symbol=has type=<Q, has.'a, has.'b>(this: &has.'b exclusive this, &has.'a exclusive Q) => boolean
    /// @type.symbol symbol=has.Q source=Q type=Q
    /// @type.symbol symbol=has.value source="value: &exclusive Q" type=&has.'a exclusive Q
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}
"#,
        r#"/// @diagnostic.error id=interface-not-implemented message="type 'Pack<T>' does not implement interface 'Has<T>'"
/// @diagnostic.label line=10 column=43 span="Has" line_source="export extension<T> of Pack<T> implements Has<T> {"
"#,
    );
}

#[test]
fn test_borrowed_entry_iterables_conform_through_elided_lifetimes() {
    let session = TestSession::single(
        r#"
import { Iterable, Iterator } from "destack:iter";
import { Access, WithAccess } from "destack:memory";
import { todo } from "destack:error";

export struct Entry<K, V> {
    key: K;
    value: V;
}

export class Bag<K, V> {
    keys: K[] = [];
    values: V[] = [];
}

export extension<K, V> of Bag<K, V>
    implements
        Iterable<(K, V)>,
        Iterable<Entry<&readonly K, &V>> {
    iterator(): Iterator<(K, V)> {
        todo("Bag.iterator")
    }

    iterator<comptime A: Access = "readonly">(
        this: WithAccess<&Bag<K, V>, A>,
    ): Iterator<Entry<&readonly K, WithAccess<&V, A>>> {
        todo("Bag.iterator")
    }
}
"#,
    );

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "destack:error";
import { Iterable, Iterator } from "destack:iter";
import { Access, WithAccess } from "destack:memory";

export struct Entry<out K, out V> {
    key: K;
    value: V;
}

export class Bag<in out K, in out V> {
    keys: K[] = [];
    values: V[] = [];
}

export extension<K, V, 'a, 'b> of Bag<K, V>
    implements
        Iterable<(K, V)>,
        Iterable<Entry<&readonly K, &V>> {
    iterator(): Iterator<(K, V)> {
        todo("Bag.iterator" as string | undefined)
    }

    iterator<comptime A: Access = "readonly">(
        this: WithAccess<&Bag<K, V>, A>,
    ): Iterator<Entry<&'a readonly K, WithAccess<&'a V, A>>> {
        todo("Bag.iterator" as string | undefined)
    }
}

=== checked ===
import { Iterable, Iterator } from "destack:iter";
import { Access, WithAccess } from "destack:memory";
import { todo } from "destack:error";

export struct Entry<K, V> {
/// @generic.template symbol=Entry parameters=(out K#1, out V#1)
/// @type.symbol symbol=Entry type=Entry
/// @definition.struct symbol=Entry template=(out K#1, out V#1)
/// @definition.field symbol=Entry.key source="key: K" key=key type=K#1
/// @definition.field symbol=Entry.value source="value: V" key=value type=V#1
/// @type.symbol symbol=Entry.K source=K type=K#1
/// @type.symbol symbol=Entry.V source=V type=V#1

    key: K;
    /// @type.symbol symbol=Entry.key source="key: K" type=K#1
    /// @resolution.name source=K target=Entry.K

    value: V;
    /// @type.symbol symbol=Entry.value source="value: V" type=V#1
    /// @resolution.name source=V target=Entry.V

}

export class Bag<K, V> {
/// @generic.template symbol=Bag parameters=(in out K#2, in out V#2)
/// @type.symbol symbol=Bag type=Bag
/// @definition.class symbol=Bag template=(in out K#2, in out V#2)
/// @definition.field symbol=Bag.keys source="keys: K[] = []" key=keys type=Array<K#2>
/// @definition.field symbol=Bag.values source="values: V[] = []" key=values type=Array<V#2>
/// @type.symbol symbol=Bag.K source=K type=K#2
/// @type.symbol symbol=Bag.V source=V type=V#2

    keys: K[] = [];
    /// @type.symbol symbol=Bag.keys source="keys: K[] = []" type=Array<K#2>
    /// @resolution.name source=K target=Bag.K

    values: V[] = [];
    /// @type.symbol symbol=Bag.values source="values: V[] = []" type=Array<V#2>
    /// @resolution.name source=V target=Bag.V

}

export extension<K, V> of Bag<K, V>
/// @generic.template symbol=<module>#2 parameters=(K#3, V#3, 'a, 'b)
/// @definition.extension symbol=<module>#2 form=exported target=Bag<K#3, V#3>
/// @definition.implements symbol=<module>#2 source="Iterable<(K, V)>" target="iter.iterator.Iterable<(K#3, V#3)>"
/// @definition.implements symbol=<module>#2 source="Iterable<Entry<&readonly K, &V>>" target="iter.iterator.Iterable<Entry<&<module>#2.'a readonly K#3, &<module>#2.'b V#3>>"
/// @definition.method symbol=iterator#1 slot=iterator type=(this: this) => iter.iterator.Iterator<(K#3, V#3)>
/// @definition.method symbol=iterator#2 slot=iterator type=<comptime A: memory.access.Access = "readonly", iterator#2.'a>(this: memory.type.WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>) => iter.iterator.Iterator<Entry<&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>>>
/// @definition.implementation symbol=<module>#2 requirement=iter.iterator.Iterable.iterator target=iterator#2
/// @type.symbol symbol=K source=K type=K#3
/// @type.symbol symbol=V source=V type=V#3
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=K target=K
/// @resolution.name source=V target=V

    implements
        Iterable<(K, V)>,
        /// @resolution.name source=Iterable target=iter.iterator.Iterable
        /// @resolution.name source=K target=K
        /// @resolution.name source=V target=V

        Iterable<Entry<&readonly K, &V>> {
        /// @resolution.name source=Iterable target=iter.iterator.Iterable
        /// @resolution.name source=Entry target=Entry
        /// @resolution.name source=K target=K
        /// @resolution.name source=V target=V

    iterator(): Iterator<(K, V)> {
    /// @type.symbol symbol=iterator#1 type=(this: this) => iter.iterator.Iterator<(K#3, V#3)> reduced=(this: this) => iter.iterator.Iterator<(K#3, V#3), void>
    /// @resolution.name source=Iterator target=iter.iterator.Iterator
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Bag.iterator")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Bag.iterator\")" parameters=(string | undefined) arguments=(provided("Bag.iterator") as string | undefined) return=never kind=symbol target=error.panic.todo

    }

    iterator<comptime A: Access = "readonly">(
    /// @generic.template symbol=iterator#2 parent=template#2 parameters=(comptime A: memory.access.Access = "readonly", 'a)
    /// @type.symbol symbol=iterator#2 type=<comptime A: memory.access.Access = "readonly", iterator#2.'a>(this: memory.type.WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>) => iter.iterator.Iterator<Entry<&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>>> reduced=<comptime A: memory.access.Access = "readonly", iterator#2.'a>(this: Borrowed<Bag<K#3, V#3>, iterator#2.'a, A>) => iter.iterator.Iterator<Entry<&iterator#2.'a readonly K#3, Borrowed<V#3, iterator#2.'a, A>>, void>
    /// @type.symbol symbol=iterator.A source="comptime A: Access = \"readonly\"" type=A
    /// @resolution.name source=Access target=memory.access.Access

        this: WithAccess<&Bag<K, V>, A>,
        /// @type.symbol symbol=iterator.this#2 source="this: WithAccess<&Bag<K, V>, A>" type=memory.type.WithAccess<&iterator#2.'a Bag<K#3, V#3>, A> reduced=Borrowed<Bag<K#3, V#3>, iterator#2.'a, A>
        /// @resolution.name source=WithAccess target=memory.type.WithAccess
        /// @resolution.name source=Bag target=Bag
        /// @resolution.name source=K target=K
        /// @resolution.name source=V target=V
        /// @resolution.name source=A target=iterator.A

    ): Iterator<Entry<&readonly K, WithAccess<&V, A>>> {
    /// @resolution.name source=Iterator target=iter.iterator.Iterator
    /// @resolution.name source=Entry target=Entry
    /// @resolution.name source=K target=K
    /// @resolution.name source=WithAccess target=memory.type.WithAccess
    /// @resolution.name source=V target=V
    /// @resolution.name source=A target=iterator.A

        todo("Bag.iterator")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"Bag.iterator\")" parameters=(string | undefined) arguments=(provided("Bag.iterator") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

/// @generic.instance id="Bag<K#3, V#3>" template=Bag arguments=(K#3, V#3)
/// @generic.instance id="Entry<&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>>" template=Entry arguments=(&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>)
/// @generic.instance id="iter.iterator.Iterator<(K#3, V#3)>" template=iter.iterator.Iterator arguments=((K#3, V#3))
/// @generic.instance id="iter.iterator.Iterator<Entry<&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>>>" template=iter.iterator.Iterator arguments=(Entry<&iterator#2.'a readonly K#3, memory.type.WithAccess<&iterator#2.'a V#3, A>>)
/// @generic.instance id="memory.type.WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>" template=memory.type.WithAccess arguments=(&iterator#2.'a Bag<K#3, V#3>, A)
/// @generic.instance id="memory.type.WithAccess<&iterator#2.'a V#3, A>" template=memory.type.WithAccess arguments=(&iterator#2.'a V#3, A)
"#,
    );
}
