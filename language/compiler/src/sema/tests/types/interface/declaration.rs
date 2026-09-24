use crate::tests::{DirRows, TestSession};

/// Report an unannotated interface field.
#[test]
fn test_report_unannotated_interface_field() {
    let session = TestSession::single(
        r#"
interface User {
    name
}
"#,
    );

    session.assert_dir_and_diagnostics(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface User {
    name;
}

=== dir ===
interface User {
/// @generic.template symbol=User parameters=(this: User)
/// @type.symbol symbol=User type=User
/// @definition.interface symbol=User template=(this: User)
/// @definition.where symbol=User relation=satisfies left=this right=User
/// @definition.field symbol=User.name source=name key=name type=<error>

    name
    /// @type.symbol symbol=User.name source=name type=<error>

}
"#,
        r#"
/// @diagnostic.error id=missing-type-annotation message="missing type annotation"
/// @diagnostic.label line=3 column=5 span="name" line_source="name"
"#,
    );
}

/// An extension records the interface members it selects.
#[test]
fn test_interface_implementation_records_selected_members() {
    let session = TestSession::single(
        r#"
declare interface ForeignProtocol {
    snake_name(): void;
}
struct Value {}
extension of Value implements ForeignProtocol {
    snake_name(): void {}
}
"#,
    );

    session.assert_dir("main.ds", DirRows::checked(), r#"
=== annotated ===
declare interface ForeignProtocol {
    snake_name(): void;
}
struct Value {}
extension of Value implements ForeignProtocol {
    snake_name(): void {}
}

=== dir ===
declare interface ForeignProtocol {
/// @generic.template symbol=ForeignProtocol parameters=(this: ForeignProtocol)
/// @type.symbol symbol=ForeignProtocol type=ForeignProtocol
/// @definition.interface symbol=ForeignProtocol template=(this: ForeignProtocol)
/// @definition.where symbol=ForeignProtocol relation=satisfies left=this right=ForeignProtocol
/// @definition.method symbol=ForeignProtocol.snake_name source="snake_name(): void" slot=snake_name type=() => void

    snake_name(): void;
    /// @type.symbol symbol=ForeignProtocol.snake_name source="snake_name(): void" type=() => void

}
struct Value {}
/// @type.symbol symbol=Value source="struct Value {}" type=Value
/// @definition.struct symbol=Value source="struct Value {}"

extension of Value implements ForeignProtocol {
/// @definition.extension symbol=<module>#2 form=local target=Value
/// @definition.implements symbol=<module>#2 source=ForeignProtocol target=ForeignProtocol
/// @definition.method symbol=snake_name source="snake_name(): void {}" slot=snake_name type=<snake_name.'a>(this: &snake_name.'a readonly Value) => void
/// @definition.conformance symbol=<module>#2 member=snake_name requirement=ForeignProtocol.snake_name
/// @resolution.name source=Value target=Value
/// @resolution.name source=ForeignProtocol target=ForeignProtocol

    snake_name(): void {}
    /// @generic.template symbol=snake_name parent=template#1 parameters=('a)
    /// @type.symbol symbol=snake_name source="snake_name(): void {}" type=<snake_name.'a>(this: &snake_name.'a readonly Value) => void
    /// @type.symbol symbol=snake_name.this type=&snake_name.'a readonly Value

}
"#);
}

/// An interface declares readonly fields, optional fields, and methods.
#[test]
fn test_interface_declares_fields_and_methods() {
    let session = TestSession::single(
        r#"
interface Person {
    readonly id: string;
    name?: string;

    rename(value: string): void;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly id: string;
    name?: string;

    rename(value: string): void;
}

=== dir ===
interface Person {
/// @generic.template symbol=Person parameters=(this: Person)
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person template=(this: Person)
/// @definition.where symbol=Person relation=satisfies left=this right=Person
/// @definition.field symbol=Person.id source="readonly id: string" key=id type=string
/// @definition.field symbol=Person.name source="name?: string" key=name type=string
/// @definition.method symbol=Person.rename source="rename(value: string): void" slot=rename type=(string) => void

    readonly id: string;
    /// @type.symbol symbol=Person.id source="readonly id: string" type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string

    rename(value: string): void;
    /// @type.symbol symbol=Person.rename source="rename(value: string): void" type=(string) => void
    /// @type.symbol symbol=Person.rename.value source="value: string" type=string

}
"#,
    );
}

/// A bound naming this holds inside the interface declaring it.
#[test]
fn test_this_bound_holds_inside_its_own_interface() {
    let session = TestSession::single(
        r#"
interface Serializer {
    serializeValue<T: Serialize<this>>(value: T): void;
}

interface Serialize<S: Serializer> {
    serialize(target: S): void;
}
"#,
    );

    session.assert_dir(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Serializer {
    serializeValue<T: Serialize<this>>(value: T): void;
}

interface Serialize<in S: Serializer> {
    serialize(target: S): void;
}

=== dir ===
interface Serializer {
/// @generic.template symbol=Serializer parameters=(this: Serializer)
/// @type.symbol symbol=Serializer type=Serializer
/// @definition.interface symbol=Serializer template=(this: Serializer)
/// @definition.where symbol=Serializer relation=satisfies left=this right=Serializer
/// @definition.method symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" slot=serializeValue type=<T: Serialize<this>>(T) => void

    serializeValue<T: Serialize<this>>(value: T): void;
    /// @generic.template symbol=Serializer.serializeValue parent=template#0 parameters=(T: Serialize<this>)
    /// @type.symbol symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" type=<T: Serialize<this>>(T) => void
    /// @type.symbol symbol=Serializer.serializeValue.T source="T: Serialize<this>" type=T
    /// @resolution.name source=Serialize target=Serialize
    /// @generic.instance id=Serialize<this> template=Serialize arguments=(this)
    /// @type.symbol symbol=Serializer.serializeValue.value source="value: T" type=T
    /// @resolution.name source=T target=Serializer.serializeValue.T

}

interface Serialize<S: Serializer> {
/// @generic.template symbol=Serialize parameters=(in S: Serializer, this: Serialize<S>)
/// @type.symbol symbol=Serialize type=Serialize
/// @definition.interface symbol=Serialize template=(in S: Serializer, this: Serialize<S>)
/// @definition.where symbol=Serialize relation=satisfies left=this right=Serialize<S>
/// @definition.method symbol=Serialize.serialize source="serialize(target: S): void" slot=serialize type=(S) => void
/// @type.symbol symbol=Serialize.S source="S: Serializer" type=S
/// @resolution.name source=Serializer target=Serializer

    serialize(target: S): void;
    /// @type.symbol symbol=Serialize.serialize source="serialize(target: S): void" type=(S) => void
    /// @type.symbol symbol=Serialize.serialize.target source="target: S" type=S
    /// @resolution.name source=S target=Serialize.S

}
"#,
    );
}

/// Anchor the induced template of a signature member on the signature's own scope.
#[test]
fn test_anchor_a_signature_member_template_on_its_own_scope() {
    let session = TestSession::single(
        r#"
export newtype interface Table<T, Context> {
    (name: string, body?: (value: &readonly T, context: &Context) => void): void;
    (name: string, options: int32, body?: (value: &readonly T, context: &Context) => void): void;
    readonly skip: Table<T, Context>;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
export newtype interface Table<out T, in out Context> {
    (name: string, body?: (value: &readonly T, context: &Context) => void): void;
    (name: string, options: int32, body?: (value: &readonly T, context: &Context) => void): void;
    readonly skip: Table<T, Context>;
}

=== dir ===
export newtype interface Table<T, Context> {
/// @generic.template symbol=Table parameters=(out T, in out Context, this: Table<T, Context>)
/// @type.symbol symbol=Table type=Table
/// @definition.interface symbol=Table template=(out T, in out Context, this: Table<T, Context>) nominal=true
/// @definition.where symbol=Table relation=satisfies left=this right=Table<T, Context>
/// @definition.field symbol=Table.skip source="readonly skip: Table<T, Context>" key=skip type=Table<T, Context>
/// @definition.signature kind=call source="(name: string, body?: (value: &readonly T, context: &Context) => void): void" type=(string, <type_expression.'a, type_expression.'b>(&type_expression.'a readonly T, &type_expression.'b Context) => void | undefined?) => void
/// @definition.signature kind=call type=(string, int32, <type_expression.'a, type_expression.'b>(&type_expression.'a readonly T, &type_expression.'b Context) => void | undefined?) => void
/// @type.symbol symbol=Table.T source=T type=T
/// @type.symbol symbol=Table.Context source=Context type=Context

    (name: string, body?: (value: &readonly T, context: &Context) => void): void;
    /// @type.symbol symbol=Table.name#1 source="name: string" type=string
    /// @type.symbol symbol=Table.body#1 source="body?: (value: &readonly T, context: &Context) => void" type=<type_expression.'a, type_expression.'b>(&type_expression.'a readonly T, &type_expression.'b Context) => void | undefined
    /// @generic.template source=type_expression parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=Table.value#1 source="value: &readonly T" type=&type_expression.'a readonly T
    /// @resolution.name source=T target=Table.T
    /// @type.symbol symbol=Table.context#1 source="context: &Context" type=&type_expression.'b Context
    /// @resolution.name source=Context target=Table.Context

    (name: string, options: int32, body?: (value: &readonly T, context: &Context) => void): void;
    /// @type.symbol symbol=Table.name#2 source="name: string" type=string
    /// @type.symbol symbol=Table.options source="options: int32" type=int32
    /// @type.symbol symbol=Table.body#2 source="body?: (value: &readonly T, context: &Context) => void" type=<type_expression.'a, type_expression.'b>(&type_expression.'a readonly T, &type_expression.'b Context) => void | undefined
    /// @generic.template source=type_expression parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=Table.value#2 source="value: &readonly T" type=&type_expression.'a readonly T
    /// @resolution.name source=T target=Table.T
    /// @type.symbol symbol=Table.context#2 source="context: &Context" type=&type_expression.'b Context
    /// @resolution.name source=Context target=Table.Context

    readonly skip: Table<T, Context>;
    /// @type.symbol symbol=Table.skip source="readonly skip: Table<T, Context>" type=Table<T, Context>
    /// @resolution.name source=Table target=Table
    /// @resolution.name source=T target=Table.T
    /// @resolution.name source=Context target=Table.Context

}
"#, r#"
"#);
}

#[test]
fn test_check_a_default_interface_body_under_its_receiver() {
    let session = TestSession::single(
        r#"
newtype interface Duplicate {
    clone(&readonly this): ^this;

    cloneFrom(&this, source: &readonly this): void {
        *this = source.clone();
    }
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
newtype interface Duplicate {
    clone(&readonly this): ^this;

    cloneFrom(&this, source: &'b readonly this): void {
        *this = source.clone<'b>();
    }
}

=== dir ===
newtype interface Duplicate {
/// @generic.template symbol=Duplicate parameters=(this: Duplicate)
/// @type.symbol symbol=Duplicate type=Duplicate
/// @definition.interface symbol=Duplicate template=(this: Duplicate) nominal=true
/// @definition.where symbol=Duplicate relation=satisfies left=this right=Duplicate
/// @definition.method symbol=Duplicate.clone source="clone(&readonly this): ^this" slot=clone type=<Duplicate.clone.'a>(this: &Duplicate.clone.'a readonly this) => ^this
/// @definition.method symbol=Duplicate.cloneFrom slot=cloneFrom type=<Duplicate.cloneFrom.'a, Duplicate.cloneFrom.'b>(this: &Duplicate.cloneFrom.'a this, &Duplicate.cloneFrom.'b readonly this) => void

    clone(&readonly this): ^this;
    /// @generic.template symbol=Duplicate.clone parent=template#0 parameters=('a)
    /// @type.symbol symbol=Duplicate.clone source="clone(&readonly this): ^this" type=<Duplicate.clone.'a>(this: &Duplicate.clone.'a readonly this) => ^this
    /// @type.symbol symbol=Duplicate.clone.this source="&readonly this" type=&Duplicate.clone.'a readonly this

    cloneFrom(&this, source: &readonly this): void {
    /// @generic.template symbol=Duplicate.cloneFrom parent=template#0 parameters=('a, 'b)
    /// @type.symbol symbol=Duplicate.cloneFrom type=<Duplicate.cloneFrom.'a, Duplicate.cloneFrom.'b>(this: &Duplicate.cloneFrom.'a this, &Duplicate.cloneFrom.'b readonly this) => void
    /// @type.symbol symbol=Duplicate.cloneFrom.this source=&this type=&Duplicate.cloneFrom.'a this
    /// @type.symbol symbol=Duplicate.cloneFrom.source source="source: &readonly this" type=&Duplicate.cloneFrom.'b readonly this

        *this = source.clone();
        /// @resolution.pattern.assign source=*this kind=place
        /// @resolution.place source=*this placement=Duplicate.cloneFrom.'a lifetime=Duplicate.cloneFrom.'a access="mutable"
        /// @resolution.assignment source=*this write="&Duplicate.cloneFrom.'a this => builtin -> ^this" type=^this
        /// @resolution.name source=this target=Duplicate.cloneFrom.this
        /// @resolution.place source=this placement=Duplicate.cloneFrom.'a lifetime=Duplicate.cloneFrom.'a access="mutable"
        /// @resolution.access source=this root=this
        /// @resolution.name source=source target=Duplicate.cloneFrom.source
        /// @resolution.member source=source.clone receiver=&Duplicate.cloneFrom.'b readonly this type=<Duplicate.clone.'a>(this: &Duplicate.clone.'a readonly this) => ^this kind=symbol target_receiver=&Duplicate.cloneFrom.'b readonly this target=Duplicate.clone
        /// @resolution.call source=source.clone() parameters=() return=^this regions=(Duplicate.cloneFrom.'b) kind=symbol target=Duplicate.clone receiver=&Duplicate.cloneFrom.'b readonly this instance=Duplicate.clone<Duplicate.cloneFrom.'b>
        /// @resolution.place source=source placement=Duplicate.cloneFrom.'b lifetime=Duplicate.cloneFrom.'b access="readonly"
        /// @resolution.access source=source root=Duplicate.cloneFrom.source
        /// @generic.instantiation id="Duplicate.clone<this, Duplicate.cloneFrom.'b>" template=Duplicate.clone arguments=(Duplicate.cloneFrom.'b) owner=Duplicate

    }
}
"#, r#"
"#);
}

#[test]
fn test_call_a_generic_sibling_from_a_default_interface_body() {
    let session = TestSession::single(
        r#"
newtype interface Values<T> {
    first(this): T {
        return this.pick<T>();
    }

    pick<C>(this): C;
}
"#,
    );

    session.assert_dir_and_diagnostics("main.ds", DirRows::checked(), r#"
=== annotated ===
newtype interface Values<out T> {
    first(this): T {
        return this.pick<T>();
    }

    pick<C>(this): C;
}

=== dir ===
newtype interface Values<T> {
/// @generic.template symbol=Values parameters=(out T, this: Values<T>)
/// @type.symbol symbol=Values type=Values
/// @definition.interface symbol=Values template=(out T, this: Values<T>) nominal=true
/// @definition.where symbol=Values relation=satisfies left=this right=Values<T>
/// @definition.method symbol=Values.first slot=first type=(this: this) => T
/// @definition.method symbol=Values.pick source="pick<C>(this): C" slot=pick type=<C>(this: this) => C
/// @type.symbol symbol=Values.T source=T type=T

    first(this): T {
    /// @type.symbol symbol=Values.first type=(this: this) => T
    /// @type.symbol symbol=Values.first.this source=this type=this
    /// @resolution.name source=T target=Values.T

        return this.pick<T>();
        /// @resolution.name source=this target=Values.first.this
        /// @resolution.member source=this.pick receiver=this type=<C>(this: this) => C kind=symbol target_receiver=this target=Values.pick
        /// @resolution.call source=this.pick<T>() parameters=() return=T kind=symbol target=Values.pick receiver=this instance=Values<T>.pick<T>
        /// @resolution.place source=this placement="local" lifetime="frame" access="exclusive"
        /// @resolution.access source=this root=this
        /// @generic.instantiation id="Values.pick<this, T, T>" template=Values.pick arguments=(T, T) owner=Values
        /// @generic.instantiation id=Values.pick<T> template=Values.pick arguments=(T) owner=Values
        /// @resolution.name source=T target=Values.T

    }

    pick<C>(this): C;
    /// @generic.template symbol=Values.pick parent=template#0 parameters=(C)
    /// @type.symbol symbol=Values.pick source="pick<C>(this): C" type=<C>(this: this) => C
    /// @type.symbol symbol=Values.pick.C source=C type=C
    /// @type.symbol symbol=Values.pick.this source=this type=this
    /// @resolution.name source=C target=Values.pick.C

}
"#, r#"

"#);
}
