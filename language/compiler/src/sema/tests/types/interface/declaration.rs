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
/// @type.symbol symbol=User type=User
/// @definition.interface symbol=User
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
/// @type.symbol symbol=ForeignProtocol type=ForeignProtocol
/// @definition.interface symbol=ForeignProtocol
/// @definition.method symbol=ForeignProtocol.snake_name source="snake_name(): void" slot=snake_name type=(this: ForeignProtocol) => void

    snake_name(): void;
    /// @type.symbol symbol=ForeignProtocol.snake_name source="snake_name(): void" type=(this: ForeignProtocol) => void

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

}
"#);
}

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
/// @type.symbol symbol=Person type=Person
/// @definition.interface symbol=Person
/// @definition.field symbol=Person.id source="readonly id: string" key=id type=string
/// @definition.field symbol=Person.name source="name?: string" key=name type=string
/// @definition.method symbol=Person.rename source="rename(value: string): void" slot=rename type=(this: Person, string) => void

    readonly id: string;
    /// @type.symbol symbol=Person.id source="readonly id: string" type=string

    name?: string;
    /// @type.symbol symbol=Person.name source="name?: string" type=string

    rename(value: string): void;
    /// @type.symbol symbol=Person.rename source="rename(value: string): void" type=(this: Person, string) => void
    /// @type.symbol symbol=Person.rename.value source="value: string" type=string

}
"#,
    );
}

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
/// @type.symbol symbol=Serializer type=Serializer
/// @definition.interface symbol=Serializer
/// @definition.method symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" slot=serializeValue type=<T: Serialize<this>>(this: this, T) => void

    serializeValue<T: Serialize<this>>(value: T): void;
    /// @generic.template symbol=Serializer.serializeValue parent=template#0 parameters=(T: Serialize<this>)
    /// @type.symbol symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" type=<T: Serialize<this>>(this: this, T) => void
    /// @type.symbol symbol=Serializer.serializeValue.T source="T: Serialize<this>" type=T
    /// @resolution.name source=Serialize target=Serialize
    /// @type.symbol symbol=Serializer.serializeValue.value source="value: T" type=T
    /// @resolution.name source=T target=Serializer.serializeValue.T

}

interface Serialize<S: Serializer> {
/// @generic.template symbol=Serialize parameters=(in S: Serializer)
/// @type.symbol symbol=Serialize type=Serialize
/// @definition.interface symbol=Serialize template=(in S: Serializer)
/// @definition.where symbol=Serialize relation=satisfies left=this right=Serialize<S>
/// @definition.method symbol=Serialize.serialize source="serialize(target: S): void" slot=serialize type=(this: this, S) => void
/// @type.symbol symbol=Serialize.S source="S: Serializer" type=S
/// @resolution.name source=Serializer target=Serializer

    serialize(target: S): void;
    /// @type.symbol symbol=Serialize.serialize source="serialize(target: S): void" type=(this: this, S) => void
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
    (name: string, body?: (value: &'a readonly T, context: &'b Context) => void): void;
    (
        name: string,
        options: int32,
        body?: (value: &'a readonly T, context: &'b Context) => void,
    ): void;
    readonly skip: Table<T, Context>;
}

=== dir ===
export newtype interface Table<T, Context> {
/// @generic.template symbol=Table parameters=(out T, in out Context)
/// @type.symbol symbol=Table type=Table
/// @definition.interface symbol=Table template=(out T, in out Context) nominal=true
/// @definition.where symbol=Table relation=satisfies left=this right=Table<T, Context>
/// @definition.field symbol=Table.skip source="readonly skip: Table<T, Context>" key=skip type=Table<T, Context>
/// @definition.signature kind=call source="(name: string, body?: (value: &readonly T, context: &Context) => void): void" type=Function<(string, Function<(&type_expression.'a readonly T, &type_expression.'b Context), void> | undefined?), void>
/// @definition.signature kind=call type=Function<(string, int32, Function<(&type_expression.'a readonly T, &type_expression.'b Context), void> | undefined?), void>
/// @type.symbol symbol=Table.T source=T type=T
/// @type.symbol symbol=Table.Context source=Context type=Context

    (name: string, body?: (value: &readonly T, context: &Context) => void): void;
    /// @type.symbol symbol=Table.name#1 source="name: string" type=string
    /// @type.symbol symbol=Table.body#1 source="body?: (value: &readonly T, context: &Context) => void" type=Function<(&type_expression.'a readonly T, &type_expression.'b Context), void> | undefined
    /// @generic.template source=type_expression parent=template#2 parameters=('a, 'b)
    /// @type.symbol symbol=Table.value#1 source="value: &readonly T" type=&type_expression.'a readonly T
    /// @resolution.name source=T target=Table.T
    /// @type.symbol symbol=Table.context#1 source="context: &Context" type=&type_expression.'b Context
    /// @resolution.name source=Context target=Table.Context

    (name: string, options: int32, body?: (value: &readonly T, context: &Context) => void): void;
    /// @type.symbol symbol=Table.name#2 source="name: string" type=string
    /// @type.symbol symbol=Table.options source="options: int32" type=int32
    /// @type.symbol symbol=Table.body#2 source="body?: (value: &readonly T, context: &Context) => void" type=Function<(&type_expression.'a readonly T, &type_expression.'b Context), void> | undefined
    /// @generic.template source=type_expression parent=template#4 parameters=('a, 'b)
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
"#, r#""#);
}
