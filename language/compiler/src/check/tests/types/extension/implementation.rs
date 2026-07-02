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
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=inherent target=User
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
/// @definition.extension symbol=<module>#3 form=inherent target=User
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
/// @diagnostic.label line=14 column=1 source="extension of User implements Show {"
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
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {}
/// @definition.extension symbol=<module>#2 source="extension of User implements Show {}" form=inherent target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
"#,
        r#"
/// @diagnostic.error code=EC203 message="type 'User' does not implement interface 'Show'"
/// @diagnostic.label line=8 column=30 source="extension of User implements Show {}"
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
/// @definition.method symbol=Named.name source="name(): string" slot=name type=(this: Named) => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=(this: Named) => string

}

newtype interface Show extends Named {
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show nominal=true
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
/// @definition.extension symbol=<module>#2 form=inherent target=User
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
/// @diagnostic.label line=12 column=30 source="extension of User implements Show {"
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
/// @definition.extension symbol=<module>#2 source="extension of User implements NotInterface {}" form=inherent target=User
/// @resolution.name source=User target=User
/// @resolution.name source=NotInterface target=NotInterface
"#,
        r#"
/// @diagnostic.error code=EC616 message="type 'User' can only implement interfaces, not 'NotInterface'"
/// @diagnostic.label line=5 column=30 source="extension of User implements NotInterface {}"
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
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}
type Alias = Show;
/// @type.symbol symbol=Alias source="type Alias = Show" type=Show
/// @definition.type symbol=Alias source="type Alias = Show" value=Show
/// @resolution.name source=Show target=Show

extension of User implements Alias {
/// @definition.extension symbol=<module>#2 form=inherent target=User
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
/// @diagnostic.label line=8 column=30 source="extension of User implements Alias {"
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
/// @definition.method symbol=Show.show source="show(): string" slot=show type=(this: Show) => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=(this: Show) => string

}
interface Debug {
/// @type.symbol symbol=Debug type=Debug
/// @definition.interface symbol=Debug
/// @definition.method symbol=Debug.debug source="debug(): string" slot=debug type=(this: Debug) => string

    debug(): string;
    /// @type.symbol symbol=Debug.debug source="debug(): string" type=(this: Debug) => string

}

extension of User implements Show | Debug {
/// @definition.extension symbol=<module>#2 form=inherent target=User
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
        todo("double")
    }
}

=== checked ===
interface Doubling {
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling
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
        /// @resolution.call source="todo(\"double\")" parameters=(string) arguments=(provided("double") as string) return=never kind=symbol target=error.panic.todo

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
        todo("halve")
    }
}

=== checked ===
interface Halving {
/// @type.symbol symbol=Halving type=Halving
/// @definition.interface symbol=Halving
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
        /// @resolution.call source="todo(\"halve\")" parameters=(string) arguments=(provided("halve") as string) return=never kind=symbol target=error.panic.todo

    }
}
"#);
}
