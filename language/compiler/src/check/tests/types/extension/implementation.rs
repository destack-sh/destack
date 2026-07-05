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
/// @generic.template symbol=Show parameters=()
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=() nominal=true
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show#1 slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show#1 type=(this: User) => string

        return "user";
    }
}

extension of User implements Show {
/// @definition.extension symbol=<module>#3 form=local target=User
/// @definition.implements symbol=<module>#3 source=Show target=Show
/// @definition.method symbol=show#2 slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show#2 type=(this: User) => string

        return "debug";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC604 message="conflicting implementations of interface 'Show' for type 'User'"
/// @diagnostic.label line=14 column=1 span="extension of User implements Show {\n    show(): string {\n        return \"debug\";\n    }\n}" line_source="extension of User implements Show {"
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
/// @generic.template symbol=Show parameters=()
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=() nominal=true
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

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
/// @diagnostic.error code=EC203 message="type 'User' does not implement interface 'Show'"
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
/// @generic.template symbol=Named parameters=()
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named template=() nominal=true
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: Named) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: Named) => string

}

newtype interface Show extends Named {
/// @generic.template symbol=Show parameters=()
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=() nominal=true
/// @definition.extends symbol=Show source=Named target=Named
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string
/// @resolution.name source=Named target=Named

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "user";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC203 message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=12 column=30 span="Show" line_source="extension of User implements Show {"
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
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'NotInterface'"
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
/// @generic.template symbol=Show parameters=()
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=()
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}
type Alias = Show;
/// @type.symbol symbol=Alias source="type Alias = Show" type=Show
/// @definition.type symbol=Alias source="type Alias = Show" value=Show
/// @resolution.name source=Show target=Show

extension of User implements Alias {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Alias target=Alias

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'Alias'"
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
/// @generic.template symbol=Show parameters=()
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=()
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}
interface Debug {
/// @generic.template symbol=Debug parameters=()
/// @type.symbol symbol=Debug type=Debug
/// @definition.interface symbol=Debug template=()
/// @definition.method symbol=Debug.debug source="debug(): string" slot=debug type=(this: Debug) => string

    debug(): string;
    /// @type.symbol symbol=Debug.debug source="debug(): string" type=(this: Debug) => string

}

extension of User implements Show | Debug {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=(this: User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
/// @resolution.name source=Debug target=Debug

    show(): string {
    /// @type.symbol symbol=show type=(this: User) => string

        return "";
    }
}
"#,
        r#"
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'Show | Debug'"
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

    double(): int32.Output {
        todo("double" as string | undefined)
    }
}

=== checked ===
interface Doubling {
/// @generic.template symbol=Doubling parameters=()
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling template=()
/// @definition.associated.type symbol=Doubling.Output source="type Output" key=Output
/// @definition.method symbol=Doubling.double source="double(): this.Output" slot=double type=(this: Doubling) => this.Output

    type Output;

    double(): this.Output;
    /// @type.symbol symbol=Doubling.double source="double(): this.Output" type=(this: Doubling) => this.Output

}

extension of int32 implements Doubling {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Doubling target=Doubling
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=double slot=double type=(this: int32) => int32.Output
/// @resolution.name source=Doubling target=Doubling

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    double(): this.Output {
    /// @type.symbol symbol=double type=(this: int32) => int32.Output reduced=(this: int32) => int32

        todo("double")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"double\")" parameters=(string | undefined) arguments=(provided("double") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}
"#);
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

    halve(&readonly this): Borrowed<int32, L0, "readonly">.Output {
        todo("halve" as string | undefined)
    }
}

=== checked ===
interface Halving {
/// @generic.template symbol=Halving parameters=()
/// @type.symbol symbol=Halving type=Halving
/// @definition.interface symbol=Halving template=()
/// @definition.associated.type symbol=Halving.Output source="type Output" key=Output
/// @definition.method symbol=Halving.halve source="halve(): this.Output" slot=halve type=(this: Halving) => this.Output

    type Output;

    halve(): this.Output;
    /// @type.symbol symbol=Halving.halve source="halve(): this.Output" type=(this: Halving) => this.Output

}

extension of int32 implements Halving {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Halving target=Halving
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=halve slot=halve type=<comptime halve.L0: Lifetime>(this: Borrowed<int32, halve.L0, "readonly">) => Borrowed<int32, halve.L0, "readonly">.Output
/// @resolution.name source=Halving target=Halving

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    halve(&readonly this): this.Output {
    /// @generic.template symbol=halve parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=halve type=<comptime halve.L0: Lifetime>(this: Borrowed<int32, halve.L0, "readonly">) => Borrowed<int32, halve.L0, "readonly">.Output reduced=<comptime halve.L0: Lifetime>(this: Borrowed<int32, halve.L0, "readonly">) => int32
    /// @type.symbol symbol=halve.this source="&readonly this" type=Borrowed<this, halve.L0, "readonly">

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

    read(): Cell.Output {
        todo("read" as string | undefined)
    }
}

extension of Cell implements Writing {
    type Output = float64;

    write(): Cell.Output {
        todo("write" as string | undefined)
    }
}

=== checked ===
interface Reading {
/// @generic.template symbol=Reading parameters=()
/// @type.symbol symbol=Reading type=Reading
/// @definition.interface symbol=Reading template=()
/// @definition.associated.type symbol=Reading.Output source="type Output" key=Output
/// @definition.method symbol=Reading.read source="read(): this.Output" slot=read type=(this: Reading) => this.Output

    type Output;

    read(): this.Output;
    /// @type.symbol symbol=Reading.read source="read(): this.Output" type=(this: Reading) => this.Output

}

interface Writing {
/// @generic.template symbol=Writing parameters=()
/// @type.symbol symbol=Writing type=Writing
/// @definition.interface symbol=Writing template=()
/// @definition.associated.type symbol=Writing.Output source="type Output" key=Output
/// @definition.method symbol=Writing.write source="write(): this.Output" slot=write type=(this: Writing) => this.Output

    type Output;

    write(): this.Output;
    /// @type.symbol symbol=Writing.write source="write(): this.Output" type=(this: Writing) => this.Output

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
/// @definition.implements symbol=<module>#2 source=Reading target=Reading
/// @definition.associated.type symbol=Output#1 source="type Output = int32" key=Output value=int32
/// @definition.method symbol=read slot=read type=(this: Cell) => Cell.Output
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Reading target=Reading

    type Output = int32;
    /// @type.symbol symbol=Output#1 source="type Output = int32" type=int32

    read(): this.Output {
    /// @type.symbol symbol=read type=(this: Cell) => Cell.Output reduced=(this: Cell) => int32

        todo("read")
        /// @resolution.name source=todo target=error.panic.todo
        /// @resolution.call source="todo(\"read\")" parameters=(string | undefined) arguments=(provided("read") as string | undefined) return=never kind=symbol target=error.panic.todo

    }
}

extension of Cell implements Writing {
/// @definition.extension symbol=<module>#3 form=local target=Cell
/// @definition.implements symbol=<module>#3 source=Writing target=Writing
/// @definition.associated.type symbol=Output#2 source="type Output = float64" key=Output value=float64
/// @definition.method symbol=write slot=write type=(this: Cell) => Cell.Output
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Writing target=Writing

    type Output = float64;
    /// @type.symbol symbol=Output#2 source="type Output = float64" type=float64

    write(): this.Output {
    /// @type.symbol symbol=write type=(this: Cell) => Cell.Output reduced=(this: Cell) => float64

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
interface Emits<T> {}

struct Channel {}

extension of Channel implements Emits<int32>, Emits<string> {}
"#,
    );

    session.assert_dir_checked_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Emits<T> {}

struct Channel {}

extension of Channel implements Emits<int32>, Emits<string> {}

=== checked ===
interface Emits<T> {}
/// @generic.template symbol=Emits parameters=(T)
/// @type.symbol symbol=Emits source="interface Emits<T> {}" type=Emits
/// @definition.interface symbol=Emits source="interface Emits<T> {}" template=(T)
/// @definition.where symbol=Emits source="interface Emits<T> {}" relation=satisfies left=this right=Emits<T>
/// @type.symbol symbol=Emits.T source=T type=T

struct Channel {}
/// @type.symbol symbol=Channel source="struct Channel {}" type=Channel
/// @definition.struct symbol=Channel source="struct Channel {}"

extension of Channel implements Emits<int32>, Emits<string> {}
/// @definition.extension symbol=<module>#2 source="extension of Channel implements Emits<int32>, Emits<string> {}" form=local target=Channel
/// @definition.implements symbol=<module>#2 source=Emits<int32> target=Emits arguments=(int32)
/// @definition.implements symbol=<module>#2 source=Emits<string> target=Emits arguments=(string)
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
interface Eq<T> {
    equals(other: Borrowed<T, L0, "readonly">): boolean;
}

interface Has<T> {
    has(value: Borrowed<T, L0, "readonly">): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T: Eq<T>> of Pack<T> implements Has<T> {
    has<Q: Eq<Q>>(value: Borrowed<Q, L1, "readonly">): boolean {
        false
    }
}

=== checked ===
interface Eq<T> {
/// @generic.template symbol=Eq parameters=(T#1)
/// @type.symbol symbol=Eq type=Eq
/// @definition.interface symbol=Eq template=(T#1)
/// @definition.where symbol=Eq relation=satisfies left=this right=Eq<T#1>
/// @definition.method symbol=Eq.equals source="equals(other: &readonly T): boolean" slot=equals type=<comptime Eq.equals.L0: Lifetime>(this: Eq<T#1>, Borrowed<T#1, Eq.equals.L0, "readonly">) => boolean
/// @type.symbol symbol=Eq.T source=T type=T#1

    equals(other: &readonly T): boolean;
    /// @generic.template symbol=Eq.equals parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Eq.equals source="equals(other: &readonly T): boolean" type=<comptime Eq.equals.L0: Lifetime>(this: Eq<T#1>, Borrowed<T#1, Eq.equals.L0, "readonly">) => boolean
    /// @type.symbol symbol=Eq.equals.other source="other: &readonly T" type=Borrowed<T#1, Eq.equals.L0, "readonly">
    /// @resolution.name source=T target=Eq.T

}

interface Has<T> {
/// @generic.template symbol=Has parameters=(T#2)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(T#2)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#2>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<comptime Has.has.L0: Lifetime>(this: Has<T#2>, Borrowed<T#2, Has.has.L0, "readonly">) => boolean
/// @type.symbol symbol=Has.T source=T type=T#2

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<comptime Has.has.L0: Lifetime>(this: Has<T#2>, Borrowed<T#2, Has.has.L0, "readonly">) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=Borrowed<T#2, Has.has.L0, "readonly">
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(T#3)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(T#3)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#3
/// @type.symbol symbol=Pack.T source=T type=T#3

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#3
    /// @resolution.name source=T target=Pack.T

}

export extension<T: Eq<T>> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#4: Eq<T#4>)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#4>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has arguments=(T#4)
/// @definition.method symbol=has slot=has type=<Q: Eq<Q>, comptime has.L1: Lifetime>(this: Pack<T#4>, Borrowed<Q, has.L1, "readonly">) => boolean
/// @type.symbol symbol=T source="T: Eq<T>" type=T#4
/// @resolution.name source=Eq target=Eq
/// @resolution.name source=T target=T
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Eq<Q>>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=(Q: Eq<Q>, comptime L1: Lifetime)
    /// @type.symbol symbol=has type=<Q: Eq<Q>, comptime has.L1: Lifetime>(this: Pack<T#4>, Borrowed<Q, has.L1, "readonly">) => boolean
    /// @type.symbol symbol=has.Q source="Q: Eq<Q>" type=Q
    /// @resolution.name source=Eq target=Eq
    /// @resolution.name source=Q target=has.Q
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=Borrowed<Q, has.L1, "readonly">
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}

/// @generic.instance id=Eq<T#1> template=Eq arguments=(T#1)
/// @generic.instance id=Has<T#2> template=Has arguments=(T#2)
/// @generic.instance id=Pack<T#4> template=Pack arguments=(T#4)

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

interface Has<T> {
    has(value: Borrowed<T, L0, "readonly">): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q: Marker>(value: Borrowed<Q, L1, "readonly">): boolean {
        false
    }
}

=== checked ===
interface Marker {}
/// @generic.template symbol=Marker parameters=()
/// @type.symbol symbol=Marker source="interface Marker {}" type=Marker
/// @definition.interface symbol=Marker source="interface Marker {}" template=()

interface Has<T> {
/// @generic.template symbol=Has parameters=(T#1)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(T#1)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<comptime Has.has.L0: Lifetime>(this: Has<T#1>, Borrowed<T#1, Has.has.L0, "readonly">) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<comptime Has.has.L0: Lifetime>(this: Has<T#1>, Borrowed<T#1, Has.has.L0, "readonly">) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=Borrowed<T#1, Has.has.L0, "readonly">
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(T#2)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(T#2)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Pack.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#2
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#3>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has arguments=(T#3)
/// @definition.method symbol=has slot=has type=<Q: Marker, comptime has.L1: Lifetime>(this: Pack<T#3>, Borrowed<Q, has.L1, "readonly">) => boolean
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Marker>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=(Q: Marker, comptime L1: Lifetime)
    /// @type.symbol symbol=has type=<Q: Marker, comptime has.L1: Lifetime>(this: Pack<T#3>, Borrowed<Q, has.L1, "readonly">) => boolean
    /// @type.symbol symbol=has.Q source="Q: Marker" type=Q
    /// @resolution.name source=Marker target=Marker
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=Borrowed<Q, has.L1, "readonly">
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}

/// @generic.instance id=Has<T#1> template=Has arguments=(T#1)
/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)

"#,
        r#"/// @diagnostic.error code=EC203 message="type 'Pack<T>' does not implement interface 'Has'"
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
interface Has<T> {
    has(value: Borrowed<T, L0, "readonly">): boolean;
}

struct Pack<T> {
    value: T;
}

export extension<T> of Pack<T> implements Has<T> {
    has<Q>(value: Borrowed<Q, L1, "exclusive">): boolean {
        false
    }
}

=== checked ===
interface Has<T> {
/// @generic.template symbol=Has parameters=(T#1)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(T#1)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<comptime Has.has.L0: Lifetime>(this: Has<T#1>, Borrowed<T#1, Has.has.L0, "readonly">) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#0 parameters=(comptime L0: Lifetime)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<comptime Has.has.L0: Lifetime>(this: Has<T#1>, Borrowed<T#1, Has.has.L0, "readonly">) => boolean
    /// @type.symbol symbol=Has.has.value source="value: &readonly T" type=Borrowed<T#1, Has.has.L0, "readonly">
    /// @resolution.name source=T target=Has.T

}

struct Pack<T> {
/// @generic.template symbol=Pack parameters=(T#2)
/// @type.symbol symbol=Pack type=Pack
/// @definition.struct symbol=Pack template=(T#2)
/// @definition.field symbol=Pack.value source="value: T" key=value type=T#2
/// @type.symbol symbol=Pack.T source=T type=T#2

    value: T;
    /// @type.symbol symbol=Pack.value source="value: T" type=T#2
    /// @resolution.name source=T target=Pack.T

}

export extension<T> of Pack<T> implements Has<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3)
/// @definition.extension symbol=<module>#2 form=exported target=Pack<T#3>
/// @definition.implements symbol=<module>#2 source=Has<T> target=Has arguments=(T#3)
/// @definition.method symbol=has slot=has type=<Q, comptime has.L1: Lifetime>(this: Pack<T#3>, Borrowed<Q, has.L1, "exclusive">) => boolean
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q>(value: &exclusive Q): boolean {
    /// @generic.template symbol=has parent=template#2 parameters=(Q, comptime L1: Lifetime)
    /// @type.symbol symbol=has type=<Q, comptime has.L1: Lifetime>(this: Pack<T#3>, Borrowed<Q, has.L1, "exclusive">) => boolean
    /// @type.symbol symbol=has.Q source=Q type=Q
    /// @type.symbol symbol=has.value source="value: &exclusive Q" type=Borrowed<Q, has.L1, "exclusive">
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}

/// @generic.instance id=Has<T#1> template=Has arguments=(T#1)
/// @generic.instance id=Pack<T#3> template=Pack arguments=(T#3)

"#,
        r#"/// @diagnostic.error code=EC203 message="type 'Pack<T>' does not implement interface 'Has'"
/// @diagnostic.label line=10 column=43 span="Has" line_source="export extension<T> of Pack<T> implements Has<T> {"
"#,
    );
}
