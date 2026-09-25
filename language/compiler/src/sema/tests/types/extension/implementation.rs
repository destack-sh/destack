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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
newtype interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show) nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=() => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=() => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show#1 slot=show type=<show#1.'a>(this: &show#1.'a readonly User) => string
/// @definition.conformance symbol=<module>#2 member=show#1 requirement=Show.show
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show#1 parent=template#1 parameters=('a)
    /// @type.symbol symbol=show#1 type=<show#1.'a>(this: &show#1.'a readonly User) => string
    /// @type.symbol symbol=show.this#1 type=&show#1.'a readonly User

        return "user";
    }
}

extension of User implements Show {
/// @definition.extension symbol=<module>#3 form=local target=User
/// @definition.implements symbol=<module>#3 source=Show target=Show
/// @definition.method symbol=show#2 slot=show type=<show#2.'a>(this: &show#2.'a readonly User) => string
/// @definition.conformance symbol=<module>#3 member=show#2 requirement=Show.show
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show#2 parent=template#2 parameters=('a)
    /// @type.symbol symbol=show#2 type=<show#2.'a>(this: &show#2.'a readonly User) => string
    /// @type.symbol symbol=show.this#2 type=&show#2.'a readonly User

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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

struct User {}

extension of User implements Show {}

=== dir ===
newtype interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show) nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=() => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=() => string

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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
newtype interface Named {
/// @generic.template symbol=Named parameters=(this: Named)
/// @type.symbol symbol=Named type=Named
/// @definition.interface symbol=Named template=(this: Named) nominal=true
/// @definition.where symbol=Named relation=satisfies left=this right=Named
/// @definition.method symbol=Named.name source="name(): string" slot=name type=() => string

    name(): string;
    /// @type.symbol symbol=Named.name source="name(): string" type=() => string

}

newtype interface Show extends Named {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show) nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.extends symbol=Show source=Named target=Named
/// @definition.method symbol=Show.show source="show(): string" slot=show type=() => string
/// @resolution.name source=Named target=Named

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=() => string

}

struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

extension of User implements Show {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.implements symbol=<module>#2 source=Show target=Show
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a readonly User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show

    show(): string {
    /// @generic.template symbol=show parent=template#2 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a readonly User) => string
    /// @type.symbol symbol=show.this type=&show.'a readonly User

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
fn test_list_the_base_interface_an_extension_implementation_supplies() {
    let session = TestSession::single(
        r#"
newtype interface PartialEqual<T = this> {
    equal(other: T): boolean;
}

newtype interface Equal<T = this> extends PartialEqual<T> {}

struct Badge {}

extension of Badge implements PartialEqual<Badge>, Equal<Badge> {
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

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface PartialEqual<in T = this> {
    equal(other: T): boolean;
}

newtype interface Equal<in T = this> extends PartialEqual<T> {}

struct Badge {}

extension of Badge implements PartialEqual<Badge>, Equal<Badge> {
    equal(other: Badge): boolean {
        true
    }
}

function compare<T: PartialEqual<T>>(left: T, right: T): boolean {
    left.equal<T>(right)
}

const ok: boolean = compare<Badge>(Badge {}, Badge {});

=== dir ===
newtype interface PartialEqual<T = this> {
/// @generic.template symbol=PartialEqual parameters=(in T#1 = this, this: PartialEqual<T#1>)
/// @type.symbol symbol=PartialEqual type=PartialEqual
/// @generic.instance id=PartialEqual<this> template=PartialEqual arguments=(this)
/// @definition.interface symbol=PartialEqual template=(in T#1 = this, this: PartialEqual<T#1>) nominal=true
/// @definition.where symbol=PartialEqual relation=satisfies left=this right=PartialEqual<T#1>
/// @definition.method symbol=PartialEqual.equal source="equal(other: T): boolean" slot=equal type=(T#1) => boolean
/// @type.symbol symbol=PartialEqual.T source="T = this" type=T#1

    equal(other: T): boolean;
    /// @type.symbol symbol=PartialEqual.equal source="equal(other: T): boolean" type=(T#1) => boolean
    /// @type.symbol symbol=PartialEqual.equal.other source="other: T" type=T#1
    /// @resolution.name source=T target=PartialEqual.T

}

newtype interface Equal<T = this> extends PartialEqual<T> {}
/// @generic.template symbol=Equal parameters=(in T#2 = this, this: Equal<T#2>)
/// @type.symbol symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" type=Equal
/// @generic.instance id=Equal<this> template=Equal arguments=(this)
/// @generic.instance id=PartialEqual<T#2> template=PartialEqual arguments=(T#2)
/// @definition.interface symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" template=(in T#2 = this, this: Equal<T#2>) nominal=true
/// @definition.where symbol=Equal source="newtype interface Equal<T = this> extends PartialEqual<T> {}" relation=satisfies left=this right=Equal<T#2>
/// @definition.extends symbol=Equal source=PartialEqual<T> target=PartialEqual<T#2>
/// @type.symbol symbol=Equal.T source="T = this" type=T#2
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=T target=Equal.T

struct Badge {}
/// @type.symbol symbol=Badge source="struct Badge {}" type=Badge
/// @definition.struct symbol=Badge source="struct Badge {}"

extension of Badge implements PartialEqual<Badge>, Equal<Badge> {
/// @generic.instance id=Equal<Badge> template=Equal arguments=(Badge)
/// @generic.instance id=PartialEqual<Badge> template=PartialEqual arguments=(Badge)
/// @definition.extension symbol=<module>#2 form=local target=Badge
/// @definition.implements symbol=<module>#2 source=Equal<Badge> target=Equal<Badge>
/// @definition.implements symbol=<module>#2 source=PartialEqual<Badge> target=PartialEqual<Badge>
/// @definition.method symbol=equal slot=equal type=<equal.'a>(this: &equal.'a readonly Badge, Badge) => boolean
/// @definition.conformance symbol=<module>#2 member=equal requirement=PartialEqual.equal
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=PartialEqual target=PartialEqual
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=Equal target=Equal
/// @resolution.name source=Badge target=Badge

    equal(other: Badge): boolean {
    /// @generic.template symbol=equal parent=template#2 parameters=('a)
    /// @type.symbol symbol=equal type=<equal.'a>(this: &equal.'a readonly Badge, Badge) => boolean
    /// @type.symbol symbol=equal.this type=&equal.'a readonly Badge
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
/// @generic.instance id=PartialEqual<T#3> template=PartialEqual arguments=(T#3)
/// @resolution.name source=T target=compare.T
/// @type.symbol symbol=compare.left source="left: T" type=T#3
/// @resolution.name source=T target=compare.T
/// @type.symbol symbol=compare.right source="right: T" type=T#3
/// @resolution.name source=T target=compare.T

    left.equal(right)
    /// @resolution.name source=left target=compare.left
    /// @resolution.member source=left.equal receiver=T#3 type=(T#3) => boolean kind=symbol target_receiver=T#3 target=PartialEqual.equal
    /// @resolution.call source=left.equal(right) parameters=(T#3) arguments=(provided(right) as T#3) return=boolean kind=symbol target=PartialEqual.equal receiver=T#3 instance=PartialEqual<T#3>.equal
    /// @resolution.place source=left placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=left root=compare.left
    /// @generic.instantiation id=PartialEqual.equal<T#3> template=PartialEqual.equal arguments=(T#3) owner=compare
    /// @resolution.name source=right target=compare.right
    /// @resolution.place source=right placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=right root=compare.right

}

const ok = compare(Badge {}, Badge {});
/// @type.symbol symbol=ok source=ok type=boolean
/// @resolution.pattern source=ok kind=binding target=ok
/// @resolution.name source=compare target=compare
/// @resolution.call source="compare(Badge {}, Badge {})" parameters=(Badge, Badge) arguments=(provided(Badge {}) as Badge, provided(Badge {}) as Badge) return=boolean kind=symbol target=compare instance=compare<Badge>
/// @generic.instantiation id=compare<Badge> template=compare arguments=(Badge)
/// @generic.instance id=compare<Badge> template=compare arguments=(Badge)
/// @resolution.name source=Badge target=Badge
/// @resolution.name source=Badge target=Badge
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
struct NotInterface {}

extension of User implements NotInterface {}

=== dir ===
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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

=== dir ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show)
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=() => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=() => string

}
type Alias = Show;
/// @type.symbol symbol=Alias source="type Alias = Show" type=Show
/// @definition.type symbol=Alias source="type Alias = Show" value=Show
/// @resolution.name source=Show target=Show

extension of User implements Alias {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a readonly User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Alias target=Alias

    show(): string {
    /// @generic.template symbol=show parent=template#1 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a readonly User) => string
    /// @type.symbol symbol=show.this type=&show.'a readonly User

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
    debug(): ^string;
}

extension of User implements Show | Debug {
    show(): string {
        return "";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
struct User {}
interface Show {
    show(): string;
}
interface Debug {
    debug(): ^string;
}

extension of User implements Show | Debug {
    show(): string {
        return "";
    }
}

=== dir ===
struct User {}
/// @type.symbol symbol=User source="struct User {}" type=User
/// @definition.struct symbol=User source="struct User {}"

interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show)
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(): string" slot=show type=() => string

    show(): string;
    /// @type.symbol symbol=Show.show source="show(): string" type=() => string

}
interface Debug {
/// @generic.template symbol=Debug parameters=(this: Debug)
/// @type.symbol symbol=Debug type=Debug
/// @definition.interface symbol=Debug template=(this: Debug)
/// @definition.where symbol=Debug relation=satisfies left=this right=Debug
/// @definition.method symbol=Debug.debug source="debug(): ^string" slot=debug type=() => ^string

    debug(): ^string;
    /// @type.symbol symbol=Debug.debug source="debug(): ^string" type=() => ^string

}

extension of User implements Show | Debug {
/// @definition.extension symbol=<module>#2 form=local target=User
/// @definition.method symbol=show slot=show type=<show.'a>(this: &show.'a readonly User) => string
/// @resolution.name source=User target=User
/// @resolution.name source=Show target=Show
/// @resolution.name source=Debug target=Debug

    show(): string {
    /// @generic.template symbol=show parent=template#2 parameters=('a)
    /// @type.symbol symbol=show type=<show.'a>(this: &show.'a readonly User) => string
    /// @type.symbol symbol=show.this type=&show.'a readonly User

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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Doubling {
    type Output;

    double(): this.Output;
}

extension of int32 implements Doubling {
    type Output = int32;

    double(): int32 {
        todo("double" as string | undefined)
    }
}

=== dir ===
interface Doubling {
/// @generic.template symbol=Doubling parameters=(this: Doubling)
/// @type.symbol symbol=Doubling type=Doubling
/// @definition.interface symbol=Doubling template=(this: Doubling)
/// @definition.where symbol=Doubling relation=satisfies left=this right=Doubling
/// @definition.associated.type symbol=Doubling.Output source="type Output" key=Output
/// @definition.method symbol=Doubling.double source="double(): this.Output" slot=double type=() => this.Output

    type Output;

    double(): this.Output;
    /// @type.symbol symbol=Doubling.double source="double(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Doubling.Output

}

extension of int32 implements Doubling {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Doubling target=Doubling
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=double slot=double type=<double.'a>(this: &double.'a readonly int32) => int32
/// @definition.conformance symbol=<module>#2 member=Output requirement=Doubling.Output
/// @definition.conformance symbol=<module>#2 member=double requirement=Doubling.double
/// @resolution.name source=Doubling target=Doubling

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    double(): this.Output {
    /// @generic.template symbol=double parent=template#1 parameters=('a)
    /// @type.symbol symbol=double type=<double.'a>(this: &double.'a readonly int32) => int32
    /// @type.symbol symbol=double.this type=&double.'a readonly int32
    /// @resolution.name source=this.Output target=Output

        todo("double")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"double\")" parameters=(string | undefined) arguments=(provided("double") as string | undefined) return=never kind=symbol target=todo

    }
}
"#);
}

#[test]
fn test_generic_extension_projects_associated_type() {
    let session = TestSession::single(
        r#"
interface Container {
    type Item;

    get(): this.Item;
}

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Container {
    type Item = T;

    get(): T {
        todo("get")
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Container {
    type Item;

    get(): this.Item;
}

struct Box<out T> {
    value: T;
}

extension<T> of Box<T> implements Container {
    type Item = T;

    get(): T {
        todo("get" as string | undefined)
    }
}

=== dir ===
interface Container {
/// @generic.template symbol=Container parameters=(this: Container)
/// @type.symbol symbol=Container type=Container
/// @definition.interface symbol=Container template=(this: Container)
/// @definition.where symbol=Container relation=satisfies left=this right=Container
/// @definition.associated.type symbol=Container.Item source="type Item" key=Item
/// @definition.method symbol=Container.get source="get(): this.Item" slot=get type=() => this.Item

    type Item;

    get(): this.Item;
    /// @type.symbol symbol=Container.get source="get(): this.Item" type=() => this.Item
    /// @resolution.name source=this.Item target=Container.Item

}

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

extension<T> of Box<T> implements Container {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.implements symbol=<module>#2 source=Container target=Container
/// @definition.associated.type symbol=Item source="type Item = T" key=Item value=T#2
/// @definition.method symbol=get slot=get type=<get.'a>(this: &get.'a readonly Box<T#2>) => T#2
/// @definition.conformance symbol=<module>#2 member=Item requirement=Container.Item
/// @definition.conformance symbol=<module>#2 member=get requirement=Container.get
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=Container target=Container

    type Item = T;
    /// @type.symbol symbol=Item source="type Item = T" type=T#2
    /// @resolution.name source=T target=T

    get(): T {
    /// @generic.template symbol=get parent=template#2 parameters=('a)
    /// @type.symbol symbol=get type=<get.'a>(this: &get.'a readonly Box<T#2>) => T#2
    /// @type.symbol symbol=get.this type=&get.'a readonly Box<T#2>
    /// @resolution.name source=T target=T

        todo("get")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"get\")" parameters=(string | undefined) arguments=(provided("get") as string | undefined) return=never kind=symbol target=todo

    }
}
"#,
    );
}

#[test]
fn test_generic_extension_preserves_other_interface_application() {
    let session = TestSession::single(
        r#"
interface Container<S> {
    type Item;
    type Output = this.Item;

    get(value: S): (
        this.Item,
        this.Output,
        Container<S>.Item,
        Container<string>.Item,
        Container<S, type Item = string>.Item,
    );
}

struct Box<T> {
    value: T;
}

extension<T> of Box<T> implements Container<T> {
    type Item = int32;

    get(value: T): (int32, int32, int32, Container<string>.Item, string) {
        todo("get")
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Container<in S> {
    type Item;
    type Output = this.Item;

    get(
        value: S,
    ): (
        this.Item,
        this.Output,
        Container<S>.Item,
        Container<string>.Item,
        Container<S, type Item = string>.Item,
    );
}

struct Box<out T> {
    value: T;
}

extension<T> of Box<T> implements Container<T> {
    type Item = int32;

    get(value: T): (int32, int32, int32, Container<string>.Item, string) {
        todo("get" as string | undefined)
    }
}

=== dir ===
interface Container<S> {
/// @generic.template symbol=Container parameters=(in S, this: Container<S>)
/// @type.symbol symbol=Container type=Container
/// @definition.interface symbol=Container template=(in S, this: Container<S>)
/// @definition.where symbol=Container relation=satisfies left=this right=Container<S>
/// @definition.associated.type symbol=Container.Item source="type Item" key=Item
/// @definition.associated.type symbol=Container.Output source="type Output = this.Item" key=Output value=this.Item
/// @definition.method symbol=Container.get slot=get type=(S) => (this.Item, this.Output, Container<S>.Item, Container<string>.Item, string)
/// @type.symbol symbol=Container.S source=S type=S

    type Item;
    type Output = this.Item;
    /// @type.symbol symbol=Container.Output source="type Output = this.Item" type=this.Item
    /// @resolution.name source=this.Item target=Container.Item

    get(value: S): (
    /// @type.symbol symbol=Container.get type=(S) => (this.Item, this.Output, Container<S>.Item, Container<string>.Item, string)
    /// @type.symbol symbol=Container.get.value source="value: S" type=S
    /// @resolution.name source=S target=Container.S

        this.Item,
        /// @resolution.name source=this.Item target=Container.Item

        this.Output,
        /// @resolution.name source=this.Output target=Container.Output

        Container<S>.Item,
        /// @resolution.name source=Container target=Container
        /// @resolution.name source=Container<S>.Item target=Container.Item
        /// @resolution.name source=S target=Container.S

        Container<string>.Item,
        /// @resolution.name source=Container target=Container
        /// @resolution.name source=Container<string>.Item target=Container.Item
        /// @generic.instance id=Container<string> template=Container arguments=(string)

        Container<S, type Item = string>.Item,
        /// @resolution.name source=Container target=Container
        /// @resolution.name source=S target=Container.S
        /// @resolution.name source="type Item = string" target=Container.Item

    );
}

struct Box<T> {
/// @generic.template symbol=Box parameters=(out T#1)
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box template=(out T#1)
/// @definition.field symbol=Box.value source="value: T" key=value type=T#1
/// @type.symbol symbol=Box.T source=T type=T#1

    value: T;
    /// @type.symbol symbol=Box.value source="value: T" type=T#1
    /// @resolution.name source=T target=Box.T

}

extension<T> of Box<T> implements Container<T> {
/// @generic.template symbol=<module>#2 parameters=(T#2)
/// @generic.instance id=Box<T#2> template=Box arguments=(T#2)
/// @generic.instance id=Container<T#2> template=Container arguments=(T#2)
/// @definition.extension symbol=<module>#2 form=local target=Box<T#2>
/// @definition.implements symbol=<module>#2 source=Container<T> target=Container<T#2>
/// @definition.associated.type symbol=Item source="type Item = int32" key=Item value=int32
/// @definition.method symbol=get slot=get type=<get.'a>(this: &get.'a readonly Box<T#2>, T#2) => (int32, int32, int32, Container<string>.Item, string)
/// @definition.conformance symbol=<module>#2 member=Container.Output requirement=Container.Output
/// @definition.conformance symbol=<module>#2 member=Item requirement=Container.Item
/// @definition.conformance symbol=<module>#2 member=get requirement=Container.get
/// @type.symbol symbol=T source=T type=T#2
/// @resolution.name source=Box target=Box
/// @resolution.name source=T target=T
/// @resolution.name source=Container target=Container
/// @resolution.name source=T target=T

    type Item = int32;
    /// @type.symbol symbol=Item source="type Item = int32" type=int32

    get(value: T): (int32, int32, int32, Container<string>.Item, string) {
    /// @generic.template symbol=get parent=template#2 parameters=('a)
    /// @type.symbol symbol=get type=<get.'a>(this: &get.'a readonly Box<T#2>, T#2) => (int32, int32, int32, Container<string>.Item, string)
    /// @type.symbol symbol=get.this type=&get.'a readonly Box<T#2>
    /// @type.symbol symbol=get.value source="value: T" type=T#2
    /// @resolution.name source=T target=T
    /// @resolution.name source=Container target=Container
    /// @resolution.name source=Container<string>.Item target=Container.Item

        todo("get")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"get\")" parameters=(string | undefined) arguments=(provided("get") as string | undefined) return=never kind=symbol target=todo

    }
}
"#,
    );
}

#[test]
fn test_extension_implements_inherited_interface_with_associated_argument() {
    let session = TestSession::single(
        r#"
interface Source<T> {
    static from(value: T): this;
}

interface Representation extends Source<this.Error> {
    type Error;
}

newtype Result<T, E> = T | E;

extension<T, E> of Result<T, E> implements Source<E>, Representation {
    type Error = E;

    static from(value: E): Result<T, E> {
        todo("from")
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Source<in T> {
    static from(value: T): this;
}

interface Representation extends Source<this.Error> {
    type Error;
}

newtype Result<out T, out E> = T | E;

extension<T, E> of Result<T, E> implements Source<E>, Representation {
    type Error = E;

    static from(value: E): Result<T, E> {
        todo("from" as string | undefined)
    }
}

=== dir ===
interface Source<T> {
/// @generic.template symbol=Source parameters=(in T#1, this: Source<T#1>)
/// @type.symbol symbol=Source type=Source
/// @definition.interface symbol=Source template=(in T#1, this: Source<T#1>)
/// @definition.where symbol=Source relation=satisfies left=this right=Source<T#1>
/// @definition.method symbol=Source.from source="static from(value: T): this" slot=from static=true type=(T#1) => this
/// @type.symbol symbol=Source.T source=T type=T#1

    static from(value: T): this;
    /// @type.symbol symbol=Source.from source="static from(value: T): this" type=(T#1) => this
    /// @type.symbol symbol=Source.from.value source="value: T" type=T#1
    /// @resolution.name source=T target=Source.T

}

interface Representation extends Source<this.Error> {
/// @generic.template symbol=Representation parameters=(this: Representation)
/// @type.symbol symbol=Representation type=Representation
/// @generic.instance id=Source<this.Error> template=Source arguments=(this.Error)
/// @definition.interface symbol=Representation template=(this: Representation)
/// @definition.where symbol=Representation relation=satisfies left=this right=Representation
/// @definition.extends symbol=Representation source=Source<this.Error> target=Source<this.Error>
/// @definition.associated.type symbol=Representation.Error source="type Error" key=Error
/// @resolution.name source=Source target=Source
/// @resolution.name source=this.Error target=Representation.Error

    type Error;
}

newtype Result<T, E> = T | E;
/// @generic.template symbol=Result parameters=(out T#2, out E#1)
/// @type.symbol symbol=Result source="newtype Result<T, E> = T | E" type=Result
/// @definition.newtype symbol=Result source="newtype Result<T, E> = T | E" template=(out T#2, out E#1) backing=T#2 | E#1 constructors=[<T#2, E#1>(T#2) => Result<T#2, E#1>, <T#2, E#1>(E#1) => Result<T#2, E#1>, <T#2, E#1>(T#2 | E#1) => Result<T#2, E#1>]
/// @type.symbol symbol=Result.T source=T type=T#2
/// @type.symbol symbol=Result.E source=E type=E#1
/// @resolution.name source=T target=Result.T
/// @resolution.name source=E target=Result.E

extension<T, E> of Result<T, E> implements Source<E>, Representation {
/// @generic.template symbol=<module>#2 parameters=(T#3, E#2)
/// @generic.instance id="Result<T#3, E#2>" template=Result arguments=(T#3, E#2)
/// @generic.instance id=Source<E#2> template=Source arguments=(E#2)
/// @definition.extension symbol=<module>#2 form=local target=Result<T#3, E#2>
/// @definition.implements symbol=<module>#2 source=Representation target=Representation
/// @definition.implements symbol=<module>#2 source=Source<E> target=Source<E#2>
/// @definition.associated.type symbol=Error source="type Error = E" key=Error value=E#2
/// @definition.method symbol=from slot=from static=true type=(E#2) => Result<T#3, E#2>
/// @definition.conformance symbol=<module>#2 member=Error requirement=Representation.Error
/// @definition.conformance symbol=<module>#2 member=from requirement=Source.from
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=E source=E type=E#2
/// @resolution.name source=Result target=Result
/// @resolution.name source=T target=T
/// @resolution.name source=E target=E
/// @resolution.name source=Source target=Source
/// @resolution.name source=E target=E
/// @resolution.name source=Representation target=Representation

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
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"from\")" parameters=(string | undefined) arguments=(provided("from") as string | undefined) return=never kind=symbol target=todo

    }
}
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

    session.assert_dir("main.tspp", DirRows::checked(), r#"
=== annotated ===
interface Halving {
    type Output;

    halve(): this.Output;
}

extension of int32 implements Halving {
    type Output = int32;

    halve(&readonly this): int32 {
        todo("halve" as string | undefined)
    }
}

=== dir ===
interface Halving {
/// @generic.template symbol=Halving parameters=(this: Halving)
/// @type.symbol symbol=Halving type=Halving
/// @definition.interface symbol=Halving template=(this: Halving)
/// @definition.where symbol=Halving relation=satisfies left=this right=Halving
/// @definition.associated.type symbol=Halving.Output source="type Output" key=Output
/// @definition.method symbol=Halving.halve source="halve(): this.Output" slot=halve type=() => this.Output

    type Output;

    halve(): this.Output;
    /// @type.symbol symbol=Halving.halve source="halve(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Halving.Output

}

extension of int32 implements Halving {
/// @definition.extension symbol=<module>#2 form=local target=int32
/// @definition.implements symbol=<module>#2 source=Halving target=Halving
/// @definition.associated.type symbol=Output source="type Output = int32" key=Output value=int32
/// @definition.method symbol=halve slot=halve type=<halve.'a>(this: &halve.'a readonly int32) => int32
/// @definition.conformance symbol=<module>#2 member=Output requirement=Halving.Output
/// @definition.conformance symbol=<module>#2 member=halve requirement=Halving.halve
/// @resolution.name source=Halving target=Halving

    type Output = int32;
    /// @type.symbol symbol=Output source="type Output = int32" type=int32

    halve(&readonly this): this.Output {
    /// @generic.template symbol=halve parent=template#1 parameters=('a)
    /// @type.symbol symbol=halve type=<halve.'a>(this: &halve.'a readonly int32) => int32
    /// @type.symbol symbol=halve.this source="&readonly this" type=&halve.'a readonly int32
    /// @resolution.name source=this.Output target=Output

        todo("halve")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"halve\")" parameters=(string | undefined) arguments=(provided("halve") as string | undefined) return=never kind=symbol target=todo

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

    session.assert_dir(
        "main.tspp",
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

    read(): int32 {
        todo("read" as string | undefined)
    }
}

extension of Cell implements Writing {
    type Output = float64;

    write(): float64 {
        todo("write" as string | undefined)
    }
}

=== dir ===
interface Reading {
/// @generic.template symbol=Reading parameters=(this: Reading)
/// @type.symbol symbol=Reading type=Reading
/// @definition.interface symbol=Reading template=(this: Reading)
/// @definition.where symbol=Reading relation=satisfies left=this right=Reading
/// @definition.associated.type symbol=Reading.Output source="type Output" key=Output
/// @definition.method symbol=Reading.read source="read(): this.Output" slot=read type=() => this.Output

    type Output;

    read(): this.Output;
    /// @type.symbol symbol=Reading.read source="read(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Reading.Output

}

interface Writing {
/// @generic.template symbol=Writing parameters=(this: Writing)
/// @type.symbol symbol=Writing type=Writing
/// @definition.interface symbol=Writing template=(this: Writing)
/// @definition.where symbol=Writing relation=satisfies left=this right=Writing
/// @definition.associated.type symbol=Writing.Output source="type Output" key=Output
/// @definition.method symbol=Writing.write source="write(): this.Output" slot=write type=() => this.Output

    type Output;

    write(): this.Output;
    /// @type.symbol symbol=Writing.write source="write(): this.Output" type=() => this.Output
    /// @resolution.name source=this.Output target=Writing.Output

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
/// @definition.method symbol=read slot=read type=<read.'a>(this: &read.'a readonly Cell) => int32
/// @definition.conformance symbol=<module>#2 member=Output#1 requirement=Reading.Output
/// @definition.conformance symbol=<module>#2 member=read requirement=Reading.read
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Reading target=Reading

    type Output = int32;
    /// @type.symbol symbol=Output#1 source="type Output = int32" type=int32

    read(): this.Output {
    /// @generic.template symbol=read parent=template#2 parameters=('a)
    /// @type.symbol symbol=read type=<read.'a>(this: &read.'a readonly Cell) => int32
    /// @type.symbol symbol=read.this type=&read.'a readonly Cell
    /// @resolution.name source=this.Output target=[Output#1, Output#2]

        todo("read")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"read\")" parameters=(string | undefined) arguments=(provided("read") as string | undefined) return=never kind=symbol target=todo

    }
}

extension of Cell implements Writing {
/// @definition.extension symbol=<module>#3 form=local target=Cell
/// @definition.implements symbol=<module>#3 source=Writing target=Writing
/// @definition.associated.type symbol=Output#2 source="type Output = float64" key=Output value=float64
/// @definition.method symbol=write slot=write type=<write.'a>(this: &write.'a readonly Cell) => float64
/// @definition.conformance symbol=<module>#3 member=Output#2 requirement=Writing.Output
/// @definition.conformance symbol=<module>#3 member=write requirement=Writing.write
/// @resolution.name source=Cell target=Cell
/// @resolution.name source=Writing target=Writing

    type Output = float64;
    /// @type.symbol symbol=Output#2 source="type Output = float64" type=float64

    write(): this.Output {
    /// @generic.template symbol=write parent=template#3 parameters=('a)
    /// @type.symbol symbol=write type=<write.'a>(this: &write.'a readonly Cell) => float64
    /// @type.symbol symbol=write.this type=&write.'a readonly Cell
    /// @resolution.name source=this.Output target=[Output#1, Output#2]

        todo("write")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"write\")" parameters=(string | undefined) arguments=(provided("write") as string | undefined) return=never kind=symbol target=todo

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

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Emits<in out T> {}

struct Channel {}

extension of Channel implements Emits<int32>, Emits<string> {}

=== dir ===
interface Emits<in out T> {}
/// @generic.template symbol=Emits parameters=(in out T, this: Emits<T>)
/// @type.symbol symbol=Emits source="interface Emits<in out T> {}" type=Emits
/// @definition.interface symbol=Emits source="interface Emits<in out T> {}" template=(in out T, this: Emits<T>)
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
        r#"

"#,
    );
}

/// Infer a call parameter before satisfying a blanket implementation bound.
#[test]
fn test_generic_call_resolves_blanket_implementation_arguments() {
    let session = TestSession::single(
        r#"
newtype interface Mine<out T, out U = string> {}

extension<T, U = string> of T implements Mine<T, U> {}

declare function requireMine<T: Mine<T>>(value: T): void;

struct Badge {}

declare const badge: Badge;
requireMine(badge);
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
newtype interface Mine<out T, out U = string> {}

extension<T, U = string> of T implements Mine<T, U> {}

declare function requireMine<T: Mine<T>>(value: T): void;

struct Badge {}

declare const badge: Badge;
requireMine<Badge>(badge);

=== dir ===
newtype interface Mine<out T, out U = string> {}
/// @generic.template symbol=Mine parameters=(out T#1, out U#1 = string, this: Mine<T#1, U#1>)
/// @type.symbol symbol=Mine source="newtype interface Mine<out T, out U = string> {}" type=Mine
/// @definition.interface symbol=Mine source="newtype interface Mine<out T, out U = string> {}" template=(out T#1, out U#1 = string, this: Mine<T#1, U#1>) nominal=true
/// @definition.where symbol=Mine source="newtype interface Mine<out T, out U = string> {}" relation=satisfies left=this right=Mine<T#1, U#1>
/// @type.symbol symbol=Mine.T source="out T" type=T#1
/// @type.symbol symbol=Mine.U source="out U = string" type=U#1

extension<T, U = string> of T implements Mine<T, U> {}
/// @generic.template symbol=<module>#2 parameters=(T#2, U#2 = string)
/// @definition.extension symbol=<module>#2 source="extension<T, U = string> of T implements Mine<T, U> {}" form=local target=T#2
/// @definition.implements symbol=<module>#2 source="Mine<T, U>" target="Mine<T#2, U#2>"
/// @type.symbol symbol=T source=T type=T#2
/// @type.symbol symbol=U source="U = string" type=U#2
/// @resolution.name source=T target=T
/// @resolution.name source=Mine target=Mine
/// @resolution.name source=T target=T
/// @resolution.name source=U target=U

declare function requireMine<T: Mine<T>>(value: T): void;
/// @generic.template symbol=requireMine parameters=(T#3: Mine<T#3, string>)
/// @type.symbol symbol=requireMine source="declare function requireMine<T: Mine<T>>(value: T): void" type=<T#3: Mine<T#3, string>>(T#3) => void
/// @type.symbol symbol=requireMine.T source="T: Mine<T>" type=T#3
/// @resolution.name source=Mine target=Mine
/// @resolution.name source=T target=requireMine.T
/// @resolution.name source=T target=requireMine.T

struct Badge {}
/// @type.symbol symbol=Badge source="struct Badge {}" type=Badge
/// @definition.struct symbol=Badge source="struct Badge {}"

declare const badge: Badge;
/// @type.symbol symbol=badge source=badge type=Badge
/// @resolution.pattern source=badge kind=binding target=badge
/// @resolution.name source=Badge target=Badge

requireMine(badge);
/// @resolution.name source=requireMine target=requireMine
/// @resolution.call source=requireMine(badge) parameters=(Badge) arguments=(provided(badge) as Badge) return=void kind=symbol target=requireMine instance=requireMine<Badge>
/// @generic.instantiation id=requireMine<Badge> template=requireMine arguments=(Badge)
/// @resolution.name source=badge target=badge
/// @resolution.place source=badge placement="local" lifetime="static" access="immutable"
/// @resolution.access source=badge root=badge
"#, r#"
"#);
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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
    has<Q: Eq<Q>>(value: &'b readonly Q): boolean {
        false
    }
}

=== dir ===
interface Eq<T> {
/// @generic.template symbol=Eq parameters=(in T#1, this: Eq<T#1>)
/// @type.symbol symbol=Eq type=Eq
/// @definition.interface symbol=Eq template=(in T#1, this: Eq<T#1>)
/// @definition.where symbol=Eq relation=satisfies left=this right=Eq<T#1>
/// @definition.method symbol=Eq.equals source="equals(other: &readonly T): boolean" slot=equals type=<Eq.equals.'a>(&Eq.equals.'a readonly T#1) => boolean
/// @type.symbol symbol=Eq.T source=T type=T#1

    equals(other: &readonly T): boolean;
    /// @generic.template symbol=Eq.equals parent=template#0 parameters=('a)
    /// @type.symbol symbol=Eq.equals source="equals(other: &readonly T): boolean" type=<Eq.equals.'a>(&Eq.equals.'a readonly T#1) => boolean
    /// @type.symbol symbol=Eq.equals.other source="other: &readonly T" type=&Eq.equals.'a readonly T#1
    /// @resolution.name source=T target=Eq.T

}

interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#2, this: Has<T#2>)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#2, this: Has<T#2>)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#2>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(&Has.has.'a readonly T#2) => boolean
/// @type.symbol symbol=Has.T source=T type=T#2

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(&Has.has.'a readonly T#2) => boolean
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
/// @definition.method symbol=has slot=has type=<has.'a, Q: Eq<Q>, has.'b>(this: &has.'a readonly Pack<T#4>, &has.'b readonly Q) => boolean
/// @definition.conformance symbol=<module>#2 member=has requirement=Has.has
/// @type.symbol symbol=T source="T: Eq<T>" type=T#4
/// @resolution.name source=Eq target=Eq
/// @resolution.name source=T target=T
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Eq<Q>>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=('a, Q: Eq<Q>, 'b)
    /// @type.symbol symbol=has type=<has.'a, Q: Eq<Q>, has.'b>(this: &has.'a readonly Pack<T#4>, &has.'b readonly Q) => boolean
    /// @type.symbol symbol=has.this type=&has.'a readonly Pack<T#4>
    /// @type.symbol symbol=has.Q source="Q: Eq<Q>" type=Q
    /// @resolution.name source=Eq target=Eq
    /// @resolution.name source=Q target=has.Q
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=&has.'b readonly Q
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}
"#,
        r#"

"#,
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

    session.assert_dir_and_diagnostics(
        "main.tspp",
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
    has<Q: Marker>(value: &'b readonly Q): boolean {
        false
    }
}

=== dir ===
interface Marker {}
/// @generic.template symbol=Marker parameters=(this: Marker)
/// @type.symbol symbol=Marker source="interface Marker {}" type=Marker
/// @definition.interface symbol=Marker source="interface Marker {}" template=(this: Marker)
/// @definition.where symbol=Marker source="interface Marker {}" relation=satisfies left=this right=Marker

interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#1, this: Has<T#1>)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#1, this: Has<T#1>)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(&Has.has.'a readonly T#1) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#1 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(&Has.has.'a readonly T#1) => boolean
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
/// @definition.method symbol=has slot=has type=<has.'a, Q: Marker, has.'b>(this: &has.'a readonly Pack<T#3>, &has.'b readonly Q) => boolean
/// @definition.conformance symbol=<module>#2 member=has requirement=Has.has
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q: Marker>(value: &readonly Q): boolean {
    /// @generic.template symbol=has parent=template#3 parameters=('a, Q: Marker, 'b)
    /// @type.symbol symbol=has type=<has.'a, Q: Marker, has.'b>(this: &has.'a readonly Pack<T#3>, &has.'b readonly Q) => boolean
    /// @type.symbol symbol=has.this type=&has.'a readonly Pack<T#3>
    /// @type.symbol symbol=has.Q source="Q: Marker" type=Q
    /// @resolution.name source=Marker target=Marker
    /// @type.symbol symbol=has.value source="value: &readonly Q" type=&has.'b readonly Q
    /// @resolution.name source=Q target=has.Q

        false
        /// @type.node source=false type=false

    }
}
"#,
        r#"

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
    has<Q>(value: &Q): boolean {
        false
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
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
    has<Q>(value: &'b Q): boolean {
        false
    }
}

=== dir ===
interface Has<T> {
/// @generic.template symbol=Has parameters=(in T#1, this: Has<T#1>)
/// @type.symbol symbol=Has type=Has
/// @definition.interface symbol=Has template=(in T#1, this: Has<T#1>)
/// @definition.where symbol=Has relation=satisfies left=this right=Has<T#1>
/// @definition.method symbol=Has.has source="has(value: &readonly T): boolean" slot=has type=<Has.has.'a>(&Has.has.'a readonly T#1) => boolean
/// @type.symbol symbol=Has.T source=T type=T#1

    has(value: &readonly T): boolean;
    /// @generic.template symbol=Has.has parent=template#0 parameters=('a)
    /// @type.symbol symbol=Has.has source="has(value: &readonly T): boolean" type=<Has.has.'a>(&Has.has.'a readonly T#1) => boolean
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
/// @definition.method symbol=has slot=has type=<has.'a, Q, has.'b>(this: &has.'a readonly Pack<T#3>, &has.'b Q) => boolean
/// @type.symbol symbol=T source=T type=T#3
/// @resolution.name source=Pack target=Pack
/// @resolution.name source=T target=T
/// @resolution.name source=Has target=Has
/// @resolution.name source=T target=T

    has<Q>(value: &Q): boolean {
    /// @generic.template symbol=has parent=template#2 parameters=('a, Q, 'b)
    /// @type.symbol symbol=has type=<has.'a, Q, has.'b>(this: &has.'a readonly Pack<T#3>, &has.'b Q) => boolean
    /// @type.symbol symbol=has.this type=&has.'a readonly Pack<T#3>
    /// @type.symbol symbol=has.Q source=Q type=Q
    /// @type.symbol symbol=has.value source="value: &Q" type=&has.'b Q
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

/// Conform borrowed entry iterables through one shared lifetime.
#[test]
fn test_conform_borrowed_entry_iterables_through_one_shared_lifetime() {
    let session = TestSession::single(
        r#"
import { Iterable, Iterator } from "tspp:iter";
import { Access, WithAccess } from "tspp:memory";
import { todo } from "tspp:error";

export struct Entry<K, V> {
    key: K;
    value: V;
}

export class Bag<K, V> {
    keys: K[] = [];
    values: V[] = [];
}

export extension<K, V, 'a> of Bag<K, V>
    implements
        Iterable<(K, V)>,
        Iterable<Entry<&'a readonly K, &'a V>> {
    iterator(): Iterator<(K, V)> {
        todo("Bag.iterator")
    }

    iterator<const A: Access = "readonly">(
        this: WithAccess<&Bag<K, V>, A>,
    ): Iterator<Entry<&'a readonly K, WithAccess<&'a V, A>>> {
        todo("Bag.iterator")
    }
}
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { todo } from "tspp:error";
import { Iterable, Iterator } from "tspp:iter";
import { Access, WithAccess } from "tspp:memory";

export struct Entry<out K, out V> {
    key: K;
    value: V;
}

export class Bag<in out K, in out V> {
    keys: K[] = [];
    values: V[] = [];
}

export extension<K, V, 'a> of Bag<K, V>
    implements Iterable<(K, V)>, Iterable<Entry<&'a readonly K, &'a V>>
{
    iterator(): Iterator<(K, V)> {
        todo("Bag.iterator" as string | undefined)
    }

    iterator<const A: Access = "readonly">(
        this: WithAccess<&Bag<K, V>, A>,
    ): Iterator<Entry<&'a readonly K, WithAccess<&'a V, A>>> {
        todo("Bag.iterator" as string | undefined)
    }
}

=== dir ===
import { Iterable, Iterator } from "tspp:iter";
import { Access, WithAccess } from "tspp:memory";
import { todo } from "tspp:error";

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
/// @type.symbol symbol=Bag type=typeof Bag
/// @definition.class symbol=Bag template=(in out K#2, in out V#2)
/// @definition.field symbol=Bag.keys source="keys: K[] = []" key=keys type=K#2[]
/// @definition.field symbol=Bag.values source="values: V[] = []" key=values type=V#2[]
/// @type.symbol symbol=Bag.K source=K type=K#2
/// @type.symbol symbol=Bag.V source=V type=V#2

    keys: K[] = [];
    /// @type.symbol symbol=Bag.keys source="keys: K[] = []" type=K#2[]
    /// @generic.instance id=Array<K#2> template=Array arguments=(K#2)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<K#2>> template=sliceAssumeInit arguments=(MaybeUninit<K#2>)
    /// @generic.instance id=sliceUninit<MaybeUninit<K#2>> template=sliceUninit arguments=(MaybeUninit<K#2>)
    /// @resolution.name source=K target=Bag.K
    /// @resolution.call source=[] parameters=(^Slice<K#2>) arguments=(rest() as K#2) return=K#2[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<K#2>
    /// @generic.instantiation id=arrayFromOwnedSlice<K#2> template=arrayFromOwnedSlice arguments=(K#2) owner=Bag
    /// @generic.instance id=arrayFromOwnedSlice<K#2> template=arrayFromOwnedSlice arguments=(K#2)

    values: V[] = [];
    /// @type.symbol symbol=Bag.values source="values: V[] = []" type=V#2[]
    /// @generic.instance id=Array<V#2> template=Array arguments=(V#2)
    /// @generic.instance id=sliceAssumeInit<MaybeUninit<V#2>> template=sliceAssumeInit arguments=(MaybeUninit<V#2>)
    /// @generic.instance id=sliceUninit<MaybeUninit<V#2>> template=sliceUninit arguments=(MaybeUninit<V#2>)
    /// @resolution.name source=V target=Bag.V
    /// @resolution.call source=[] parameters=(^Slice<V#2>) arguments=(rest() as V#2) return=V#2[] kind=symbol target=arrayFromOwnedSlice instance=arrayFromOwnedSlice<V#2>
    /// @generic.instantiation id=arrayFromOwnedSlice<V#2> template=arrayFromOwnedSlice arguments=(V#2) owner=Bag
    /// @generic.instance id=arrayFromOwnedSlice<V#2> template=arrayFromOwnedSlice arguments=(V#2)

}

export extension<K, V, 'a> of Bag<K, V>
/// @generic.template symbol=<module>#2 parameters=(K#3, V#3, 'a)
/// @generic.instance id="Bag<K#3, V#3>" template=Bag arguments=(K#3, V#3)
/// @generic.instance id="Entry<&'a readonly K#3, &'a V#3>" template=Entry arguments=(&'a readonly K#3, &'a V#3)
/// @generic.instance id="Iterable<(K#3, V#3)>" template=Iterable arguments=((K#3, V#3))
/// @generic.instance id="Iterable<Entry<&'a readonly K#3, &'a V#3>>" template=Iterable arguments=(Entry<&'a readonly K#3, &'a V#3>)
/// @definition.extension symbol=<module>#2 form=exported target=Bag<K#3, V#3>
/// @definition.implements symbol=<module>#2 source="Iterable<(K, V)>" target="Iterable<(K#3, V#3)>"
/// @definition.implements symbol=<module>#2 source="Iterable<Entry<&'a readonly K, &'a V>>" target="Iterable<Entry<&'a readonly K#3, &'a V#3>>"
/// @definition.method symbol=iterator#1 slot=iterator type=(this: Bag<K#3, V#3>) => Iterator<(K#3, V#3)>
/// @definition.method symbol=iterator#2 slot=iterator type=<const A: Access = "readonly", iterator#2.'a>(this: WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>) => Iterator<Entry<&'a readonly K#3, WithAccess<&'a V#3, A>>>
/// @definition.conformance symbol=<module>#2 member=Iterable.Iterator requirement=Iterable.Iterator
/// @definition.conformance symbol=<module>#2 member=iterator#1 requirement=Iterable.iterator
/// @definition.conformance symbol=<module>#2 member=iterator#2 requirement=Iterable.iterator
/// @type.symbol symbol=K source=K type=K#3
/// @type.symbol symbol=V source=V type=V#3
/// @type.symbol symbol='a source='a type='a
/// @resolution.name source=Bag target=Bag
/// @resolution.name source=K target=K
/// @resolution.name source=V target=V

    implements
        Iterable<(K, V)>,
        /// @resolution.name source=Iterable target=Iterable
        /// @resolution.name source=K target=K
        /// @resolution.name source=V target=V

        Iterable<Entry<&'a readonly K, &'a V>> {
        /// @resolution.name source=Iterable target=Iterable
        /// @resolution.name source=Entry target=Entry
        /// @resolution.name source='a target='a
        /// @resolution.name source=K target=K
        /// @resolution.name source='a target='a
        /// @resolution.name source=V target=V

    iterator(): Iterator<(K, V)> {
    /// @type.symbol symbol=iterator#1 type=(this: Bag<K#3, V#3>) => Iterator<(K#3, V#3)>
    /// @type.symbol symbol=iterator.this#1 type=Bag<K#3, V#3>
    /// @generic.instance id="Iterator<(K#3, V#3)>" template=Iterator arguments=((K#3, V#3))
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=K target=K
    /// @resolution.name source=V target=V

        todo("Bag.iterator")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Bag.iterator\")" parameters=(string | undefined) arguments=(provided("Bag.iterator") as string | undefined) return=never kind=symbol target=todo

    }

    iterator<const A: Access = "readonly">(
    /// @generic.template symbol=iterator#2 parent=template#2 parameters=(const A: Access = "readonly", 'a)
    /// @type.symbol symbol=iterator#2 type=<const A: Access = "readonly", iterator#2.'a>(this: WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>) => Iterator<Entry<&'a readonly K#3, WithAccess<&'a V#3, A>>>
    /// @generic.instance id="Entry<&'a readonly K#3, WithAccess<&'a V#3, A>>" template=Entry arguments=(&'a readonly K#3, WithAccess<&'a V#3, A>)
    /// @generic.instance id="Iterator<Entry<&'a readonly K#3, WithAccess<&'a V#3, A>>>" template=Iterator arguments=(Entry<&'a readonly K#3, WithAccess<&'a V#3, A>>)
    /// @type.symbol symbol=iterator.A source="const A: Access = \"readonly\"" type=A
    /// @resolution.name source=Access target=Access

        this: WithAccess<&Bag<K, V>, A>,
        /// @type.symbol symbol=iterator.this#2 source="this: WithAccess<&Bag<K, V>, A>" type=WithAccess<&iterator#2.'a Bag<K#3, V#3>, A>
        /// @resolution.name source=WithAccess target=WithAccess
        /// @resolution.name source=Bag target=Bag
        /// @resolution.name source=K target=K
        /// @resolution.name source=V target=V
        /// @resolution.name source=A target=iterator.A

    ): Iterator<Entry<&'a readonly K, WithAccess<&'a V, A>>> {
    /// @resolution.name source=Iterator target=Iterator
    /// @resolution.name source=Entry target=Entry
    /// @resolution.name source='a target='a
    /// @resolution.name source=K target=K
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source='a target='a
    /// @resolution.name source=V target=V
    /// @resolution.name source=A target=iterator.A

        todo("Bag.iterator")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Bag.iterator\")" parameters=(string | undefined) arguments=(provided("Bag.iterator") as string | undefined) return=never kind=symbol target=todo

    }
}
"#,
    );
}

#[test]
fn test_satisfy_extension_implements_with_inherent_members() {
    let session = TestSession::single(
        r#"
interface Greeter {
    greet(): string;
}

class Robot {
    greet(): string {
        return "beep";
    }
}

extension of Robot implements Greeter {}

declare const robot: Robot;
const greeter: Greeter = robot;
"#,
    );

    session.assert_dir(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Greeter {
    greet(): string;
}

class Robot {
    greet(): string {
        return "beep";
    }
}

extension of Robot implements Greeter {}

declare const robot: Robot;
const greeter: Greeter = robot as Greeter;

=== dir ===
interface Greeter {
/// @generic.template symbol=Greeter parameters=(this: Greeter)
/// @type.symbol symbol=Greeter type=Greeter
/// @definition.interface symbol=Greeter template=(this: Greeter)
/// @definition.where symbol=Greeter relation=satisfies left=this right=Greeter
/// @definition.method symbol=Greeter.greet source="greet(): string" slot=greet type=() => string

    greet(): string;
    /// @type.symbol symbol=Greeter.greet source="greet(): string" type=() => string

}

class Robot {
/// @type.symbol symbol=Robot type=typeof Robot
/// @definition.class symbol=Robot
/// @definition.method symbol=Robot.greet slot=greet type=(this: Robot) => string

    greet(): string {
    /// @type.symbol symbol=Robot.greet type=(this: Robot) => string
    /// @type.symbol symbol=Robot.greet.this type=Robot

        return "beep";
    }
}

extension of Robot implements Greeter {}
/// @definition.extension symbol=<module>#2 source="extension of Robot implements Greeter {}" form=local target=Robot
/// @definition.implements symbol=<module>#2 source=Greeter target=Greeter
/// @definition.conformance symbol=<module>#2 member=Robot.greet requirement=Greeter.greet
/// @resolution.name source=Robot target=Robot
/// @resolution.name source=Greeter target=Greeter

declare const robot: Robot;
/// @type.symbol symbol=robot source=robot type=Robot
/// @resolution.pattern source=robot kind=binding target=robot
/// @resolution.name source=Robot target=Robot

const greeter: Greeter = robot;
/// @type.symbol symbol=greeter source=greeter type=Greeter
/// @resolution.pattern source=greeter kind=binding target=greeter
/// @resolution.name source=Greeter target=Greeter
/// @resolution.name source=robot target=robot
/// @resolution.place source=robot placement="local" lifetime="static" access="immutable"
/// @resolution.access source=robot root=robot
"#,
    );
}

#[test]
fn test_accept_scalar_implementations_across_the_widening_chain() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

extension Int8Show of int8 implements Show {
    show(): string {
        return "int8";
    }
}

extension Int16Show of int16 implements Show {
    show(): string {
        return "int16";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

extension Int8Show of int8 implements Show {
    show(): string {
        return "int8";
    }
}

extension Int16Show of int16 implements Show {
    show(): string {
        return "int16";
    }
}

=== dir ===
newtype interface Show {
    show(): string;
}

extension Int8Show of int8 implements Show {
    show(): string {
        return "int8";
    }
}

extension Int16Show of int16 implements Show {
    show(): string {
        return "int16";
    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_reject_duplicate_implementations_of_one_scalar() {
    let session = TestSession::single(
        r#"
newtype interface Show {
    show(): string;
}

extension FirstShow of int8 implements Show {
    show(): string {
        return "first";
    }
}

extension SecondShow of int8 implements Show {
    show(): string {
        return "second";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
newtype interface Show {
    show(): string;
}

extension FirstShow of int8 implements Show {
    show(): string {
        return "first";
    }
}

extension SecondShow of int8 implements Show {
    show(): string {
        return "second";
    }
}

=== dir ===
newtype interface Show {
    show(): string;
}

extension FirstShow of int8 implements Show {
    show(): string {
        return "first";
    }
}

extension SecondShow of int8 implements Show {
    show(): string {
        return "second";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Show' for type 'int8'"
/// @diagnostic.label line=12 column=11 span="SecondShow" line_source="extension SecondShow of int8 implements Show {"
/// @diagnostic.related line=6 column=11 span="FirstShow" line_source="extension FirstShow of int8 implements Show {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_reject_an_implementation_overlapping_a_bounded_blanket() {
    let session = TestSession::single(
        r#"
import { Integer } from "tspp:math";

newtype interface Show {
    show(&readonly this): string;
}

extension AnyShow<T: Integer> of T implements Show {
    show(&readonly this): string {
        return "any";
    }
}

extension IntShow of int16 implements Show {
    show(&readonly this): string {
        return "int16";
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Integer } from "tspp:math";

newtype interface Show {
    show(&readonly this): string;
}

extension AnyShow<T: Integer> of T implements Show {
    show(&readonly this): string {
        return "any";
    }
}

extension IntShow of int16 implements Show {
    show(&readonly this): string {
        return "int16";
    }
}

=== dir ===
import { Integer } from "tspp:math";

newtype interface Show {
/// @generic.template symbol=Show parameters=(this: Show)
/// @type.symbol symbol=Show type=Show
/// @definition.interface symbol=Show template=(this: Show) nominal=true
/// @definition.where symbol=Show relation=satisfies left=this right=Show
/// @definition.method symbol=Show.show source="show(&readonly this): string" slot=show type=<Show.show.'a>(this: &Show.show.'a readonly this) => string

    show(&readonly this): string;
    /// @generic.template symbol=Show.show parent=template#0 parameters=('a)
    /// @type.symbol symbol=Show.show source="show(&readonly this): string" type=<Show.show.'a>(this: &Show.show.'a readonly this) => string
    /// @type.symbol symbol=Show.show.this source="&readonly this" type=&Show.show.'a readonly this

}

extension AnyShow<T: Integer> of T implements Show {
/// @generic.template symbol=AnyShow parameters=(T: Integer)
/// @definition.extension symbol=AnyShow form=local target=T
/// @definition.implements symbol=AnyShow source=Show target=Show
/// @definition.method symbol=AnyShow.show slot=show type=<AnyShow.show.'a>(this: &AnyShow.show.'a readonly T) => string
/// @definition.conformance symbol=AnyShow member=AnyShow.show requirement=Show.show
/// @type.symbol symbol=AnyShow.T source="T: Integer" type=T
/// @resolution.name source=Integer target=Integer
/// @resolution.name source=T target=AnyShow.T
/// @resolution.name source=Show target=Show

    show(&readonly this): string {
    /// @generic.template symbol=AnyShow.show parent=template#1 parameters=('a)
    /// @type.symbol symbol=AnyShow.show type=<AnyShow.show.'a>(this: &AnyShow.show.'a readonly T) => string
    /// @type.symbol symbol=AnyShow.show.this source="&readonly this" type=&AnyShow.show.'a readonly T

        return "any";
    }
}

extension IntShow of int16 implements Show {
/// @definition.extension symbol=IntShow form=local target=int16
/// @definition.implements symbol=IntShow source=Show target=Show
/// @definition.method symbol=IntShow.show slot=show type=<IntShow.show.'a>(this: &IntShow.show.'a readonly int16) => string
/// @definition.conformance symbol=IntShow member=IntShow.show requirement=Show.show
/// @resolution.name source=Show target=Show

    show(&readonly this): string {
    /// @generic.template symbol=IntShow.show parent=template#2 parameters=('a)
    /// @type.symbol symbol=IntShow.show type=<IntShow.show.'a>(this: &IntShow.show.'a readonly int16) => string
    /// @type.symbol symbol=IntShow.show.this source="&readonly this" type=&IntShow.show.'a readonly int16

        return "int16";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Show' for type 'int16'"
/// @diagnostic.label line=14 column=11 span="IntShow" line_source="extension IntShow of int16 implements Show {"
/// @diagnostic.related line=8 column=11 span="AnyShow" line_source="extension AnyShow<T: Integer> of T implements Show {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_accept_distinct_interface_instantiations_on_one_scalar() {
    let session = TestSession::single(
        r#"
newtype interface Convert<U> {
    to(): U;
}

extension ToInt16 of int8 implements Convert<int16> {
    to(): int16 {
        return 1;
    }
}

extension ToInt32 of int8 implements Convert<int32> {
    to(): int32 {
        return 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::none(),
        r#"
=== annotated ===
newtype interface Convert<out U> {
    to(): U;
}

extension ToInt16 of int8 implements Convert<int16> {
    to(): int16 {
        return 1;
    }
}

extension ToInt32 of int8 implements Convert<int32> {
    to(): int32 {
        return 1;
    }
}

=== dir ===
newtype interface Convert<U> {
    to(): U;
}

extension ToInt16 of int8 implements Convert<int16> {
    to(): int16 {
        return 1;
    }
}

extension ToInt32 of int8 implements Convert<int32> {
    to(): int32 {
        return 1;
    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_skip_private_members_when_selecting_conformance_members() {
    let session = TestSession::single(
        r#"
newtype interface It<T, R = void> {
    next(this): R {
        todo("next")
    }

    first(this): T | undefined {
        todo("first")
    }
}

struct Chain<I, J, out T> {
    private first: I;
    private second: J;
    private marker: () => T;
}

extension<T, R, Q, I: It<T, R>, J: It<T, Q>> of Chain<I, J, T> implements It<T> {
    next(): void {
        todo("next")
    }
}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface It<out T, out R = void> {
    next(this): R {
        todo("next" as string | undefined)
    }

    first(this): T | undefined {
        todo("first" as string | undefined)
    }
}

struct Chain<out I, out J, out T> {
    private first: I;
    private second: J;
    private marker: () => T;
}

extension<T, R, Q, I: It<T, R>, J: It<T, Q>> of Chain<I, J, T> implements It<T> {
    next(): void {
        todo("next" as string | undefined);
    }
}

=== dir ===
newtype interface It<T, R = void> {
/// @generic.template symbol=It parameters=(out T#1, out R#1 = void, this: It<T#1, R#1>)
/// @type.symbol symbol=It type=It
/// @definition.interface symbol=It template=(out T#1, out R#1 = void, this: It<T#1, R#1>) nominal=true
/// @definition.where symbol=It relation=satisfies left=this right=It<T#1, R#1>
/// @definition.method symbol=It.first slot=first type=(this: this) => T#1 | undefined
/// @definition.method symbol=It.next slot=next type=(this: this) => R#1
/// @type.symbol symbol=It.T source=T type=T#1
/// @type.symbol symbol=It.R source="R = void" type=R#1

    next(this): R {
    /// @type.symbol symbol=It.next type=(this: this) => R#1
    /// @type.symbol symbol=It.next.this source=this type=this
    /// @resolution.name source=R target=It.R

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }

    first(this): T | undefined {
    /// @type.symbol symbol=It.first type=(this: this) => T#1 | undefined
    /// @type.symbol symbol=It.first.this source=this type=this
    /// @resolution.name source=T target=It.T

        todo("first")
        /// @type.node source="todo(\"first\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"first\")" parameters=(string | undefined) arguments=(provided("first") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"first\"" type="first"

    }
}

struct Chain<I, J, out T> {
/// @generic.template symbol=Chain parameters=(out I#1, out J#1, out T#2)
/// @type.symbol symbol=Chain type=Chain
/// @definition.struct symbol=Chain template=(out I#1, out J#1, out T#2)
/// @definition.field symbol=Chain.first source="private first: I" key=first visibility=private type=I#1
/// @definition.field symbol=Chain.marker source="private marker: () => T" key=marker visibility=private type=() => T#2
/// @definition.field symbol=Chain.second source="private second: J" key=second visibility=private type=J#1
/// @type.symbol symbol=Chain.I source=I type=I#1
/// @type.symbol symbol=Chain.J source=J type=J#1
/// @type.symbol symbol=Chain.T source="out T" type=T#2

    private first: I;
    /// @type.symbol symbol=Chain.first source="private first: I" type=I#1
    /// @resolution.name source=I target=Chain.I

    private second: J;
    /// @type.symbol symbol=Chain.second source="private second: J" type=J#1
    /// @resolution.name source=J target=Chain.J

    private marker: () => T;
    /// @type.symbol symbol=Chain.marker source="private marker: () => T" type=() => T#2
    /// @resolution.name source=T target=Chain.T

}

extension<T, R, Q, I: It<T, R>, J: It<T, Q>> of Chain<I, J, T> implements It<T> {
/// @generic.template symbol=<module>#2 parameters=(T#3, R#2, Q, I#2: It<T#3, R#2>, J#2: It<T#3, Q>)
/// @generic.instance id="Chain<I#2, J#2, T#3>" template=Chain arguments=(I#2, J#2, T#3)
/// @generic.instance id="It<T#3, void>" template=It arguments=(T#3, void)
/// @definition.extension symbol=<module>#2 form=local target=Chain<I#2, J#2, T#3>
/// @definition.implements symbol=<module>#2 source=It<T> target="It<T#3, void>"
/// @definition.method symbol=next slot=next type=<next.'a>(this: &next.'a readonly Chain<I#2, J#2, T#3>) => void
/// @definition.conformance symbol=<module>#2 member=It.first requirement=It.first
/// @definition.conformance symbol=<module>#2 member=next requirement=It.next
/// @type.symbol symbol=T source=T type=T#3
/// @type.symbol symbol=R source=R type=R#2
/// @type.symbol symbol=Q source=Q type=Q
/// @type.symbol symbol=I source="I: It<T, R>" type=I#2
/// @resolution.name source=It target=It
/// @generic.instance id="It<T#3, R#2>" template=It arguments=(T#3, R#2)
/// @resolution.name source=T target=T
/// @resolution.name source=R target=R
/// @type.symbol symbol=J source="J: It<T, Q>" type=J#2
/// @resolution.name source=It target=It
/// @generic.instance id="It<T#3, Q>" template=It arguments=(T#3, Q)
/// @resolution.name source=T target=T
/// @resolution.name source=Q target=Q
/// @resolution.name source=Chain target=Chain
/// @resolution.name source=I target=I
/// @resolution.name source=J target=J
/// @resolution.name source=T target=T
/// @resolution.name source=It target=It
/// @resolution.name source=T target=T

    next(): void {
    /// @generic.template symbol=next parent=template#2 parameters=('a)
    /// @type.symbol symbol=next type=<next.'a>(this: &next.'a readonly Chain<I#2, J#2, T#3>) => void
    /// @type.symbol symbol=next.this type=&next.'a readonly Chain<I#2, J#2, T#3>

        todo("next")
        /// @type.node source="todo(\"next\")" type=never
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"next\")" parameters=(string | undefined) arguments=(provided("next") as string | undefined) return=never kind=symbol target=todo
        /// @type.node source="\"next\"" type="next"

    }
}
"#);
}

#[test]
fn test_select_conformance_members_from_the_target_declaration() {
    let session = TestSession::single(
        r#"
newtype interface Sized {
    size(this): isize;
}

class Box {
    width: isize = 0;

    size(this): isize {
        this.width
    }
}

extension of Box implements Sized {}
"#,
    );

    session.assert_dir("main.tspp", DirRows::checked().with_node_types(), r#"
=== annotated ===
newtype interface Sized {
    size(this): isize;
}

class Box {
    width: isize = 0;

    size(this): isize {
        this.width
    }
}

extension of Box implements Sized {}

=== dir ===
newtype interface Sized {
/// @generic.template symbol=Sized parameters=(this: Sized)
/// @type.symbol symbol=Sized type=Sized
/// @definition.interface symbol=Sized template=(this: Sized) nominal=true
/// @definition.where symbol=Sized relation=satisfies left=this right=Sized
/// @definition.method symbol=Sized.size source="size(this): isize" slot=size type=(this: this) => isize

    size(this): isize;
    /// @type.symbol symbol=Sized.size source="size(this): isize" type=(this: this) => isize
    /// @type.symbol symbol=Sized.size.this source=this type=this

}

class Box {
/// @type.symbol symbol=Box type=typeof Box
/// @definition.class symbol=Box
/// @definition.field symbol=Box.width source="width: isize = 0" key=width type=isize
/// @definition.method symbol=Box.size slot=size type=(this: Box) => isize

    width: isize = 0;
    /// @type.symbol symbol=Box.width source="width: isize = 0" type=isize
    /// @type.node source=0 type=0

    size(this): isize {
    /// @type.symbol symbol=Box.size type=(this: Box) => isize
    /// @type.symbol symbol=Box.size.this source=this type=Box

        this.width
        /// @type.node source=this.width type=isize
        /// @resolution.member source=this.width receiver=Box type=isize kind=field target_receiver=Box key=width target=Box.width target_type=isize
        /// @resolution.receiver source=this kind=this declaration=Box type=Box
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.width placement="local" lifetime="managed" access="mutable"
        /// @resolution.access source=this.width root=this keys=[width]

    }
}

extension of Box implements Sized {}
/// @definition.extension symbol=<module>#2 source="extension of Box implements Sized {}" form=local target=Box
/// @definition.implements symbol=<module>#2 source=Sized target=Sized
/// @definition.conformance symbol=<module>#2 member=Box.size requirement=Sized.size
/// @resolution.name source=Box target=Box
/// @resolution.name source=Sized target=Sized
"#);
}

#[test]
fn test_reject_conformances_without_a_providing_member() {
    let session = TestSession::single(
        r#"
newtype interface Sized {
    size(this): isize;
}

struct Box {
    private size: isize;
}

extension of Box implements Sized {}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked().with_node_types(),
        r#"
=== annotated ===
newtype interface Sized {
    size(this): isize;
}

struct Box {
    private size: isize;
}

extension of Box implements Sized {}

=== dir ===
newtype interface Sized {
/// @generic.template symbol=Sized parameters=(this: Sized)
/// @type.symbol symbol=Sized type=Sized
/// @definition.interface symbol=Sized template=(this: Sized) nominal=true
/// @definition.where symbol=Sized relation=satisfies left=this right=Sized
/// @definition.method symbol=Sized.size source="size(this): isize" slot=size type=(this: this) => isize

    size(this): isize;
    /// @type.symbol symbol=Sized.size source="size(this): isize" type=(this: this) => isize
    /// @type.symbol symbol=Sized.size.this source=this type=this

}

struct Box {
/// @type.symbol symbol=Box type=Box
/// @definition.struct symbol=Box
/// @definition.field symbol=Box.size source="private size: isize" key=size visibility=private type=isize

    private size: isize;
    /// @type.symbol symbol=Box.size source="private size: isize" type=isize

}

extension of Box implements Sized {}
/// @definition.extension symbol=<module>#2 source="extension of Box implements Sized {}" form=local target=Box
/// @definition.implements symbol=<module>#2 source=Sized target=Sized
/// @resolution.name source=Box target=Box
/// @resolution.name source=Sized target=Sized
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Box' does not implement interface 'Sized'"
/// @diagnostic.label line=10 column=29 span="Sized" line_source="extension of Box implements Sized {}"
"#,
    );
}

#[test]
fn test_reject_implementations_meeting_across_modules() {
    let session = TestSession::builder()
        .module(
            "bell.tspp",
            r#"
export newtype interface Quiet {
    whisper(this): string;
}

export struct Bell {}
"#,
        )
        .module(
            "first.tspp",
            r#"
import { Bell, Quiet } from "./bell.tspp";

export extension FirstQuiet of Bell implements Quiet {
    whisper(this): string {
        return "first";
    }
}
"#,
        )
        .module(
            "second.tspp",
            r#"
import { Bell, Quiet } from "./bell.tspp";

export extension SecondQuiet of Bell implements Quiet {
    whisper(this): string {
        return "second";
    }
}
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "second.tspp",
        DirRows::none(),
        r#"
=== annotated ===
import { Bell, Quiet } from "./bell.tspp";

export extension SecondQuiet of Bell implements Quiet {
    whisper(this): string {
        return "second";
    }
}

=== dir ===
import { Bell, Quiet } from "./bell.tspp";

export extension SecondQuiet of Bell implements Quiet {
    whisper(this): string {
        return "second";
    }
}
"#,
        r#"
/// @diagnostic.error id=conflicting-implementation message="conflicting implementations of interface 'Quiet' for type 'Bell'"
/// @diagnostic.label line=4 column=18 span="SecondQuiet" line_source="export extension SecondQuiet of Bell implements Quiet {"
/// @diagnostic.related file="first.tspp" line=4 column=18 span="FirstQuiet" line_source="export extension FirstQuiet of Bell implements Quiet {" message="conflicting implementation"
"#,
    );
}

#[test]
fn test_satisfy_a_bound_through_an_unimported_implementation() {
    let session = TestSession::builder()
        .module(
            "bell.tspp",
            r#"
export newtype interface Quiet {
    whisper(this): string;
}

export struct Bell {}
"#,
        )
        .module(
            "first.tspp",
            r#"
import { Bell, Quiet } from "./bell.tspp";

export extension FirstQuiet of Bell implements Quiet {
    whisper(this): string {
        return "first";
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { Bell, Quiet } from "./bell.tspp";

declare function hush<T: Quiet>(value: T): string;

const sound = hush(Bell {});
"#,
        )
        .build();

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Bell, Quiet } from "./bell.tspp";

declare function hush<T: Quiet>(value: T): string;

const sound: string = hush<Bell>(Bell {});

=== dir ===
import { Bell, Quiet } from "./bell.tspp";

declare function hush<T: Quiet>(value: T): string;
/// @generic.template symbol=hush parameters=(T: bell.Quiet)
/// @type.symbol symbol=hush source="declare function hush<T: Quiet>(value: T): string" type=<T: bell.Quiet>(T) => string
/// @type.symbol symbol=hush.T source="T: Quiet" type=T
/// @resolution.name source=Quiet target=bell.Quiet
/// @resolution.name source=T target=hush.T

const sound = hush(Bell {});
/// @type.symbol symbol=sound source=sound type=string
/// @resolution.pattern source=sound kind=binding target=sound
/// @resolution.name source=hush target=hush
/// @resolution.call source="hush(Bell {})" parameters=(bell.Bell) arguments=(provided(Bell {}) as bell.Bell) return=string kind=symbol target=hush instance=hush<bell.Bell>
/// @generic.instantiation id=hush<bell.Bell> template=hush arguments=(bell.Bell)
/// @resolution.name source=Bell target=bell.Bell
"#,
        r#"

"#,
    );
}

/// Implement an interface over a generic tuple target and satisfy a bound through it.
#[test]
fn test_implement_an_interface_over_a_generic_tuple_target() {
    let session = TestSession::single(
        r#"
interface Sized {
    size(): isize;
}

extension<First: Copy, Second: Copy> of (First, Second) implements Sized {
    size(): isize {
        return 2;
    }
}

function measure<T: Sized>(value: T): isize {
    return value.size();
}

declare const pair: (int32, boolean);

const direct = pair.size();
const bounded = measure(pair);
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
interface Sized {
    size(): isize;
}

extension<First: Copy, Second: Copy> of (First, Second) implements Sized {
    size(): isize {
        return 2;
    }
}

function measure<T: Sized>(value: T): isize {
    return value.size();
}

declare const pair: (int32, boolean);

const direct: isize = pair.size<int32, boolean, "static">();
const bounded: isize = measure<(int32, boolean)>(pair);

=== dir ===
interface Sized {
/// @generic.template symbol=Sized parameters=(this: Sized)
/// @type.symbol symbol=Sized type=Sized
/// @definition.interface symbol=Sized template=(this: Sized)
/// @definition.where symbol=Sized relation=satisfies left=this right=Sized
/// @definition.method symbol=Sized.size source="size(): isize" slot=size type=() => isize

    size(): isize;
    /// @type.symbol symbol=Sized.size source="size(): isize" type=() => isize

}

extension<First: Copy, Second: Copy> of (First, Second) implements Sized {
/// @generic.template symbol=<module>#2 parameters=(First: Copy, Second: Copy)
/// @definition.extension symbol=<module>#2 form=local target=(First, Second)
/// @definition.implements symbol=<module>#2 source=Sized target=Sized
/// @definition.method symbol=size slot=size type=<size.'a>(this: &size.'a readonly (First, Second)) => isize
/// @definition.conformance symbol=<module>#2 member=size requirement=Sized.size
/// @type.symbol symbol=First source="First: Copy" type=First
/// @resolution.name source=Copy target=Copy
/// @type.symbol symbol=Second source="Second: Copy" type=Second
/// @resolution.name source=Copy target=Copy
/// @resolution.name source=First target=First
/// @resolution.name source=Second target=Second
/// @resolution.name source=Sized target=Sized

    size(): isize {
    /// @generic.template symbol=size parent=template#1 parameters=('a)
    /// @type.symbol symbol=size type=<size.'a>(this: &size.'a readonly (First, Second)) => isize
    /// @type.symbol symbol=size.this type=&size.'a readonly (First, Second)

        return 2;
    }
}

function measure<T: Sized>(value: T): isize {
/// @generic.template symbol=measure parameters=(T: Sized)
/// @type.symbol symbol=measure type=<T: Sized>(T) => isize
/// @type.symbol symbol=measure.T source="T: Sized" type=T
/// @resolution.name source=Sized target=Sized
/// @type.symbol symbol=measure.value source="value: T" type=T
/// @resolution.name source=T target=measure.T

    return value.size();
    /// @resolution.name source=value target=measure.value
    /// @resolution.member source=value.size receiver=T type=() => isize kind=symbol target_receiver=T target=Sized.size
    /// @resolution.call source=value.size() parameters=() return=isize kind=symbol target=Sized.size receiver=T
    /// @resolution.place source=value placement="local" lifetime="frame" access="exclusive"
    /// @resolution.access source=value root=measure.value

}

declare const pair: (int32, boolean);
/// @type.symbol symbol=pair source=pair type=(int32, boolean)
/// @resolution.pattern source=pair kind=binding target=pair

const direct = pair.size();
/// @type.symbol symbol=direct source=direct type=isize
/// @resolution.pattern source=direct kind=binding target=direct
/// @resolution.name source=pair target=pair
/// @resolution.member source=pair.size receiver=(int32, boolean) type=<size.'a>(this: &size.'a readonly (int32, boolean)) => isize kind=symbol target_receiver=(int32, boolean) target=size
/// @resolution.call source=pair.size() parameters=() return=isize regions=("static" & "local") kind=symbol target=size receiver=(int32, boolean) adjustments=(borrow(&'static readonly (int32, boolean))) instance="(First, Second).<extension#1>.size<\"static\" & \"local\">"
/// @resolution.place source=pair placement="local" lifetime="static" access="immutable"
/// @resolution.access source=pair root=pair
/// @generic.instantiation id="size<int32, boolean, \"static\" & \"local\">" template=size arguments=(int32, boolean, "static" & "local")
/// @generic.instantiation id="size<int32, boolean>" template=size arguments=(int32, boolean)

const bounded = measure(pair);
/// @type.symbol symbol=bounded source=bounded type=isize
/// @resolution.pattern source=bounded kind=binding target=bounded
/// @resolution.name source=measure target=measure
/// @resolution.call source=measure(pair) parameters=((int32, boolean)) arguments=(provided(pair) as (int32, boolean)) return=isize kind=symbol target=measure instance="measure<(int32, boolean)>"
/// @generic.instantiation id="measure<(int32, boolean)>" template=measure arguments=((int32, boolean))
/// @resolution.name source=pair target=pair
/// @resolution.place source=pair placement="local" lifetime="static" access="immutable"
/// @resolution.access source=pair root=pair
"#,
        r#"
"#,
    );
}

/// An extension warns when target and interface are both foreign.
#[test]
fn test_foreign_extension_implementation_warns_as_non_local() {
    let session = TestSession::single(
        r#"
import { todo } from "tspp:error";
import { Zero } from "tspp:math";
import { Duration } from "tspp:time";

extension of Duration implements Zero {
    static zero(): Duration {
        todo("Duration.zero")
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.tspp", DirRows::checked(), r#"
=== annotated ===
import { todo } from "tspp:error";
import { Zero } from "tspp:math";
import { Duration } from "tspp:time";

extension of Duration implements Zero {
    static zero(): Duration {
        todo("Duration.zero" as string | undefined)
    }
}

=== dir ===
import { todo } from "tspp:error";
import { Zero } from "tspp:math";
import { Duration } from "tspp:time";

extension of Duration implements Zero {
/// @definition.extension symbol=<module>#2 form=local target=Duration
/// @definition.implements symbol=<module>#2 source=Zero target=Zero
/// @definition.method symbol=zero slot=zero static=true type=() => Duration
/// @definition.conformance symbol=<module>#2 member=zero requirement=Zero.zero
/// @resolution.name source=Duration target=Duration
/// @resolution.name source=Zero target=Zero

    static zero(): Duration {
    /// @type.symbol symbol=zero type=() => Duration
    /// @resolution.name source=Duration target=Duration

        todo("Duration.zero")
        /// @resolution.name source=todo target=todo
        /// @resolution.call source="todo(\"Duration.zero\")" parameters=(string | undefined) arguments=(provided("Duration.zero") as string | undefined) return=never kind=symbol target=todo

    }
}
"#, r#"
/// @diagnostic.warning id=non-local-implementation message="implementation of foreign interface 'Zero' for foreign type 'Duration' is not local to this package"
/// @diagnostic.label line=6 column=14 span="Duration" line_source="extension of Duration implements Zero {"
"#);
}

/// Accept an implementation method at the receiver its requirement declares.
#[test]
fn test_accept_an_implementation_at_the_requirement_receiver() {
    let session = TestSession::single(
        r#"
newtype interface Tally {
    bump(&this): void;
}

struct Counter {
    value: int32;
}

extension of Counter implements Tally {
    bump(&this): void {
        this.value += 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Tally {
    bump(&this): void;
}

struct Counter {
    value: int32;
}

extension of Counter implements Tally {
    bump(&this): void {
        this.value += 1;
    }
}

=== dir ===
newtype interface Tally {
/// @generic.template symbol=Tally parameters=(this: Tally)
/// @type.symbol symbol=Tally type=Tally
/// @definition.interface symbol=Tally template=(this: Tally) nominal=true
/// @definition.where symbol=Tally relation=satisfies left=this right=Tally
/// @definition.method symbol=Tally.bump source="bump(&this): void" slot=bump type=<Tally.bump.'a>(this: &Tally.bump.'a this) => void

    bump(&this): void;
    /// @generic.template symbol=Tally.bump parent=template#0 parameters=('a)
    /// @type.symbol symbol=Tally.bump source="bump(&this): void" type=<Tally.bump.'a>(this: &Tally.bump.'a this) => void
    /// @type.symbol symbol=Tally.bump.this source=&this type=&Tally.bump.'a this

}

struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

}

extension of Counter implements Tally {
/// @definition.extension symbol=<module>#2 form=local target=Counter
/// @definition.implements symbol=<module>#2 source=Tally target=Tally
/// @definition.method symbol=bump slot=bump type=<bump.'a>(this: &bump.'a Counter) => void
/// @definition.conformance symbol=<module>#2 member=bump requirement=Tally.bump
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=Tally target=Tally

    bump(&this): void {
    /// @generic.template symbol=bump parent=template#1 parameters=('a)
    /// @type.symbol symbol=bump type=<bump.'a>(this: &bump.'a Counter) => void
    /// @type.symbol symbol=bump.this source=&this type=&bump.'a Counter

        this.value += 1;
        /// @resolution.operator source="this.value += 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&bump.'a Counter
        /// @resolution.place source=this placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.assignment source=this.value read="receiver=&bump.'a Counter, target=field(receiver=&bump.'a Counter, target=Counter.value, type=int32), type=int32" write="receiver=&bump.'a Counter, target=field(receiver=&bump.'a Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
        r#"

"#,
    );
}

#[test]
fn test_accept_an_implementation_demanding_a_weaker_receiver() {
    let session = TestSession::single(
        r#"
newtype interface Tally {
    bump(&this): void;
}

struct Counter {
    value: int32;
}

extension of Counter implements Tally {
    bump(&this): void {
        this.value += 1;
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Tally {
    bump(&this): void;
}

struct Counter {
    value: int32;
}

extension of Counter implements Tally {
    bump(&this): void {
        this.value += 1;
    }
}

=== dir ===
newtype interface Tally {
/// @generic.template symbol=Tally parameters=(this: Tally)
/// @type.symbol symbol=Tally type=Tally
/// @definition.interface symbol=Tally template=(this: Tally) nominal=true
/// @definition.where symbol=Tally relation=satisfies left=this right=Tally
/// @definition.method symbol=Tally.bump source="bump(&this): void" slot=bump type=<Tally.bump.'a>(this: &Tally.bump.'a this) => void

    bump(&this): void;
    /// @generic.template symbol=Tally.bump parent=template#0 parameters=('a)
    /// @type.symbol symbol=Tally.bump source="bump(&this): void" type=<Tally.bump.'a>(this: &Tally.bump.'a this) => void
    /// @type.symbol symbol=Tally.bump.this source=&this type=&Tally.bump.'a this

}

struct Counter {
/// @type.symbol symbol=Counter type=Counter
/// @definition.struct symbol=Counter
/// @definition.field symbol=Counter.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Counter.value source="value: int32" type=int32

}

extension of Counter implements Tally {
/// @definition.extension symbol=<module>#2 form=local target=Counter
/// @definition.implements symbol=<module>#2 source=Tally target=Tally
/// @definition.method symbol=bump slot=bump type=<bump.'a>(this: &bump.'a Counter) => void
/// @definition.conformance symbol=<module>#2 member=bump requirement=Tally.bump
/// @resolution.name source=Counter target=Counter
/// @resolution.name source=Tally target=Tally

    bump(&this): void {
    /// @generic.template symbol=bump parent=template#1 parameters=('a)
    /// @type.symbol symbol=bump type=<bump.'a>(this: &bump.'a Counter) => void
    /// @type.symbol symbol=bump.this source=&this type=&bump.'a Counter

        this.value += 1;
        /// @resolution.operator source="this.value += 1" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), 1 as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&bump.'a Counter
        /// @resolution.place source=this placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.pattern.assign source=this.value kind=place
        /// @resolution.place source=this.value placement=bump.'a lifetime=bump.'a access="mutable"
        /// @resolution.assignment source=this.value read="receiver=&bump.'a Counter, target=field(receiver=&bump.'a Counter, target=Counter.value, type=int32), type=int32" write="receiver=&bump.'a Counter, target=field(receiver=&bump.'a Counter, target=Counter.value, type=int32), type=int32" type=int32
        /// @resolution.access source=this.value root=this keys=[value]

    }
}
"#,
        r#"
"#,
    );
}

#[test]
fn test_reject_a_literal_demand_under_an_access_generic_promise() {
    let session = TestSession::single(
        r#"
import { Access, WithAccess } from "tspp:memory";

struct Buffer {
    x: int32;
}

interface Viewable {
    view<const A: Access>(this: WithAccess<&Buffer, A>): int32;
}

extension of Buffer implements Viewable {
    view(&this): int32 {
        this.x
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
import { Access, WithAccess } from "tspp:memory";

struct Buffer {
    x: int32;
}

interface Viewable {
    view<const A: Access>(this: WithAccess<&Buffer, A>): int32;
}

extension of Buffer implements Viewable {
    view(&this): int32 {
        this.x
    }
}

=== dir ===
import { Access, WithAccess } from "tspp:memory";

struct Buffer {
/// @type.symbol symbol=Buffer type=Buffer
/// @definition.struct symbol=Buffer
/// @definition.field symbol=Buffer.x source="x: int32" key=x type=int32

    x: int32;
    /// @type.symbol symbol=Buffer.x source="x: int32" type=int32

}

interface Viewable {
/// @generic.template symbol=Viewable parameters=(this: Viewable)
/// @type.symbol symbol=Viewable type=Viewable
/// @definition.interface symbol=Viewable template=(this: Viewable)
/// @definition.where symbol=Viewable relation=satisfies left=this right=Viewable
/// @definition.method symbol=Viewable.view source="view<const A: Access>(this: WithAccess<&Buffer, A>): int32" slot=view type=<const A: Access, Viewable.view.'a>(this: WithAccess<&Viewable.view.'a Buffer, A>) => int32

    view<const A: Access>(this: WithAccess<&Buffer, A>): int32;
    /// @generic.template symbol=Viewable.view parent=template#0 parameters=(const A: Access, 'a)
    /// @type.symbol symbol=Viewable.view source="view<const A: Access>(this: WithAccess<&Buffer, A>): int32" type=<const A: Access, Viewable.view.'a>(this: WithAccess<&Viewable.view.'a Buffer, A>) => int32
    /// @type.symbol symbol=Viewable.view.A source="const A: Access" type=A
    /// @resolution.name source=Access target=Access
    /// @type.symbol symbol=Viewable.view.this source="this: WithAccess<&Buffer, A>" type=WithAccess<&Viewable.view.'a Buffer, A>
    /// @resolution.name source=WithAccess target=WithAccess
    /// @resolution.name source=Buffer target=Buffer
    /// @resolution.name source=A target=Viewable.view.A

}

extension of Buffer implements Viewable {
/// @definition.extension symbol=<module>#2 form=local target=Buffer
/// @definition.implements symbol=<module>#2 source=Viewable target=Viewable
/// @definition.method symbol=view slot=view type=<view.'a>(this: &view.'a Buffer) => int32
/// @resolution.name source=Buffer target=Buffer
/// @resolution.name source=Viewable target=Viewable

    view(&this): int32 {
    /// @generic.template symbol=view parent=template#1 parameters=('a)
    /// @type.symbol symbol=view type=<view.'a>(this: &view.'a Buffer) => int32
    /// @type.symbol symbol=view.this source=&this type=&view.'a Buffer

        this.x
        /// @resolution.member source=this.x receiver=&view.'a Buffer type=int32 kind=field target_receiver=&view.'a Buffer key=x target=Buffer.x target_type=int32
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&view.'a Buffer
        /// @resolution.place source=this placement=view.'a lifetime=view.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.x placement=view.'a lifetime=view.'a access="mutable"
        /// @resolution.access source=this.x root=this keys=[x]

    }
}
"#,
        r#"
/// @diagnostic.error id=interface-not-implemented message="type 'Buffer' does not implement interface 'Viewable'"
/// @diagnostic.label line=12 column=32 span="Viewable" line_source="extension of Buffer implements Viewable {"
"#,
    );
}

#[test]
fn test_accept_elided_receivers_across_a_conformance() {
    let session = TestSession::single(
        r#"
newtype interface Speak {
    say(): void;
}

struct Dog {}

extension of Dog implements Speak {
    say(): void {}
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Speak {
    say(): void;
}

struct Dog {}

extension of Dog implements Speak {
    say(): void {}
}

=== dir ===
newtype interface Speak {
/// @generic.template symbol=Speak parameters=(this: Speak)
/// @type.symbol symbol=Speak type=Speak
/// @definition.interface symbol=Speak template=(this: Speak) nominal=true
/// @definition.where symbol=Speak relation=satisfies left=this right=Speak
/// @definition.method symbol=Speak.say source="say(): void" slot=say type=() => void

    say(): void;
    /// @type.symbol symbol=Speak.say source="say(): void" type=() => void

}

struct Dog {}
/// @type.symbol symbol=Dog source="struct Dog {}" type=Dog
/// @definition.struct symbol=Dog source="struct Dog {}"

extension of Dog implements Speak {
/// @definition.extension symbol=<module>#2 form=local target=Dog
/// @definition.implements symbol=<module>#2 source=Speak target=Speak
/// @definition.method symbol=say source="say(): void {}" slot=say type=<say.'a>(this: &say.'a readonly Dog) => void
/// @definition.conformance symbol=<module>#2 member=say requirement=Speak.say
/// @resolution.name source=Dog target=Dog
/// @resolution.name source=Speak target=Speak

    say(): void {}
    /// @generic.template symbol=say parent=template#1 parameters=('a)
    /// @type.symbol symbol=say source="say(): void {}" type=<say.'a>(this: &say.'a readonly Dog) => void
    /// @type.symbol symbol=say.this type=&say.'a readonly Dog

}
"#,
        r#"

"#,
    );
}

#[test]
fn test_accept_an_associated_output_conformance() {
    let session = TestSession::single(
        r#"
newtype interface Adder<T = this> {
    type Output;

    add(other: T): this.Output;
}

struct Num {
    value: int32;
}

extension of Num implements Adder {
    type Output = ^Num;

    add(other: Num): ^Num {
        Num { value: this.value + other.value }
    }
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.tspp",
        DirRows::checked(),
        r#"
=== annotated ===
newtype interface Adder<in T = this> {
    type Output;

    add(other: T): this.Output;
}

struct Num {
    value: int32;
}

extension of Num implements Adder {
    type Output = ^Num;

    add(other: Num): Num {
        Num { value: this.value + other.value }
    }
}

=== dir ===
newtype interface Adder<T = this> {
/// @generic.template symbol=Adder parameters=(in T = this, this: Adder<T>)
/// @type.symbol symbol=Adder type=Adder
/// @definition.interface symbol=Adder template=(in T = this, this: Adder<T>) nominal=true
/// @definition.where symbol=Adder relation=satisfies left=this right=Adder<T>
/// @definition.associated.type symbol=Adder.Output source="type Output" key=Output
/// @definition.method symbol=Adder.add source="add(other: T): this.Output" slot=add type=(T) => this.Output
/// @type.symbol symbol=Adder.T source="T = this" type=T

    type Output;

    add(other: T): this.Output;
    /// @type.symbol symbol=Adder.add source="add(other: T): this.Output" type=(T) => this.Output
    /// @type.symbol symbol=Adder.add.other source="other: T" type=T
    /// @resolution.name source=T target=Adder.T
    /// @resolution.name source=this.Output target=Adder.Output

}

struct Num {
/// @type.symbol symbol=Num type=Num
/// @definition.struct symbol=Num
/// @definition.field symbol=Num.value source="value: int32" key=value type=int32

    value: int32;
    /// @type.symbol symbol=Num.value source="value: int32" type=int32

}

extension of Num implements Adder {
/// @definition.extension symbol=<module>#2 form=local target=Num
/// @definition.implements symbol=<module>#2 source=Adder target=Adder<this>
/// @definition.associated.type symbol=Output source="type Output = ^Num" key=Output value=Num
/// @definition.method symbol=add slot=add type=<add.'a>(this: &add.'a readonly Num, Num) => Num
/// @definition.conformance symbol=<module>#2 member=Output requirement=Adder.Output
/// @definition.conformance symbol=<module>#2 member=add requirement=Adder.add
/// @resolution.name source=Num target=Num
/// @resolution.name source=Adder target=Adder

    type Output = ^Num;
    /// @type.symbol symbol=Output source="type Output = ^Num" type=Num
    /// @resolution.name source=Num target=Num

    add(other: Num): ^Num {
    /// @generic.template symbol=add parent=template#1 parameters=('a)
    /// @type.symbol symbol=add type=<add.'a>(this: &add.'a readonly Num, Num) => Num
    /// @type.symbol symbol=add.this type=&add.'a readonly Num
    /// @type.symbol symbol=add.other source="other: Num" type=Num
    /// @resolution.name source=Num target=Num
    /// @resolution.name source=Num target=Num

        Num { value: this.value + other.value }
        /// @resolution.name source=Num target=Num
        /// @resolution.member source=this.value receiver=&add.'a readonly Num type=int32 kind=field target_receiver=&add.'a readonly Num key=value target=Num.value target_type=int32
        /// @resolution.operator source="this.value + other.value" type=int32 operator="+" kind=builtin operands=[this.value as int32 families=(integer), other.value as int32 families=(integer)]
        /// @resolution.receiver source=this kind=this declaration=<module>#2 type=&add.'a readonly Num
        /// @resolution.place source=this placement=add.'a lifetime=add.'a access="readonly"
        /// @resolution.access source=this root=this
        /// @resolution.place source=this.value placement=add.'a lifetime=add.'a access="readonly"
        /// @resolution.access source=this.value root=this keys=[value]
        /// @resolution.name source=other target=add.other
        /// @resolution.member source=other.value receiver=Num type=int32 kind=field target_receiver=Num key=value target=Num.value target_type=int32
        /// @resolution.place source=other placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other root=add.other
        /// @resolution.place source=other.value placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=other.value root=add.other keys=[value]

    }
}
"#,
        r#"

"#,
    );
}
