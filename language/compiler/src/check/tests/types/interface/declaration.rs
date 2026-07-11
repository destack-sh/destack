use crate::tests::{DirRows, TestSession};

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

    session.assert_dir_checked(
        "main.ds",
        DirRows::checked(),
        r#"
=== annotated ===
interface Person {
    readonly id: string;
    name?: string;

    rename(value: string): void;
}

=== checked ===
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

    session.assert_dir_checked(
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

=== checked ===
interface Serializer {
/// @type.symbol symbol=Serializer type=Serializer
/// @definition.interface symbol=Serializer
/// @definition.method symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" slot=serializeValue type=<T: Serialize<this>>(this: Serializer, T) => void

    serializeValue<T: Serialize<this>>(value: T): void;
    /// @generic.template symbol=Serializer.serializeValue parent=template#0 parameters=(T: Serialize<this>)
    /// @type.symbol symbol=Serializer.serializeValue source="serializeValue<T: Serialize<this>>(value: T): void" type=<T: Serialize<this>>(this: Serializer, T) => void
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
/// @definition.method symbol=Serialize.serialize source="serialize(target: S): void" slot=serialize type=(this: Serialize<S>, S) => void
/// @type.symbol symbol=Serialize.S source="S: Serializer" type=S
/// @resolution.name source=Serializer target=Serializer

    serialize(target: S): void;
    /// @type.symbol symbol=Serialize.serialize source="serialize(target: S): void" type=(this: Serialize<S>, S) => void
    /// @type.symbol symbol=Serialize.serialize.target source="target: S" type=S
    /// @resolution.name source=S target=Serialize.S

}

/// @generic.instance id=Serialize<S> template=Serialize arguments=(S)
"#,
    );
}
