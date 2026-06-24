# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.js.tree.dependency
import destack._generated.js.tree.function
import destack._generated.js.tree.key
import destack._generated.js.tree.node


@dataclass(frozen=True, slots=True)
class DeclarationGlobal:
    """Global augmentation declaration."""

    global_: GlobalDeclaration
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationType:
    """Type alias declaration."""

    type: TypeDeclaration
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationClass:
    """Class declaration."""

    class_: ClassDeclaration
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationInterface:
    """Interface declaration."""

    interface: InterfaceDeclaration
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationEnum:
    """Enum declaration."""

    enum: EnumDeclaration
    kind: typing.Literal["enum"] = "enum"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationFunction:
    """Function declaration."""

    function: FunctionDeclaration
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


"""A declaration item."""
Declaration: typing.TypeAlias = (
    DeclarationGlobal
    | DeclarationType
    | DeclarationClass
    | DeclarationInterface
    | DeclarationEnum
    | DeclarationFunction
)


def encode_declaration(writer: BinaryWriter, value: Declaration) -> None:
    """Encode one Declaration."""
    if value.kind == "global":
        writer.write_unsigned(0)
        encode_global_declaration(writer, value.global_)
    elif value.kind == "type":
        writer.write_unsigned(1)
        encode_type_declaration(writer, value.type)
    elif value.kind == "class":
        writer.write_unsigned(2)
        encode_class_declaration(writer, value.class_)
    elif value.kind == "interface":
        writer.write_unsigned(3)
        encode_interface_declaration(writer, value.interface)
    elif value.kind == "enum":
        writer.write_unsigned(4)
        encode_enum_declaration(writer, value.enum)
    elif value.kind == "function":
        writer.write_unsigned(5)
        encode_function_declaration(writer, value.function)
    else:
        raise SerdeError("unknown enum variant")


def decode_declaration(reader: BinaryReader) -> Declaration:
    """Decode one Declaration."""
    variant = reader.read_number()

    if variant == 0:
        global_ = decode_global_declaration(reader)

        return DeclarationGlobal(global_=global_)
    elif variant == 1:
        type = decode_type_declaration(reader)

        return DeclarationType(type=type)
    elif variant == 2:
        class_ = decode_class_declaration(reader)

        return DeclarationClass(class_=class_)
    elif variant == 3:
        interface = decode_interface_declaration(reader)

        return DeclarationInterface(interface=interface)
    elif variant == 4:
        enum = decode_enum_declaration(reader)

        return DeclarationEnum(enum=enum)
    elif variant == 5:
        function = decode_function_declaration(reader)

        return DeclarationFunction(function=function)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_declaration(value: Declaration) -> Json:
    """Return one JSON value for one Declaration."""
    if value.kind == "global":
        return {
            "kind": "global",
            "global": to_json_global_declaration(value.global_),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "type": to_json_type_declaration(value.type),
        }
    elif value.kind == "class":
        return {
            "kind": "class",
            "class": to_json_class_declaration(value.class_),
        }
    elif value.kind == "interface":
        return {
            "kind": "interface",
            "interface": to_json_interface_declaration(value.interface),
        }
    elif value.kind == "enum":
        return {
            "kind": "enum",
            "enum": to_json_enum_declaration(value.enum),
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "function": to_json_function_declaration(value.function),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_declaration(value: Json) -> Declaration:
    """Return one Declaration from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "global":
        return DeclarationGlobal(
            global_=from_json_global_declaration(json_field(object_, "global"))
        )
    elif kind == "type":
        return DeclarationType(
            type=from_json_type_declaration(json_field(object_, "type"))
        )
    elif kind == "class":
        return DeclarationClass(
            class_=from_json_class_declaration(json_field(object_, "class"))
        )
    elif kind == "interface":
        return DeclarationInterface(
            interface=from_json_interface_declaration(json_field(object_, "interface"))
        )
    elif kind == "enum":
        return DeclarationEnum(
            enum=from_json_enum_declaration(json_field(object_, "enum"))
        )
    elif kind == "function":
        return DeclarationFunction(
            function=from_json_function_declaration(json_field(object_, "function"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GlobalDeclaration:
    """A global augmentation declaration."""

    # whether the declaration is ambient
    is_ambient: bool
    # the statements inside the global body
    statements: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalDeclaration:
        """Decode one GlobalDeclaration."""
        return decode_global_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalDeclaration:
        """Return one GlobalDeclaration from one JSON value."""
        return from_json_global_declaration(value)


def encode_global_declaration(writer: BinaryWriter, value: GlobalDeclaration) -> None:
    """Encode one GlobalDeclaration."""
    writer.write_bool(value.is_ambient)
    writer.write_unsigned(len(value.statements))
    for item_value_statements_0 in value.statements:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_statements_0
        )


def decode_global_declaration(reader: BinaryReader) -> GlobalDeclaration:
    """Decode one GlobalDeclaration."""
    is_ambient = reader.read_bool()
    statements = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return GlobalDeclaration(
        is_ambient=is_ambient,
        statements=statements,
    )


def to_json_global_declaration(value: GlobalDeclaration) -> Json:
    """Return one JSON value for one GlobalDeclaration."""
    return {
        "isAmbient": value.is_ambient,
        "statements": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.statements
        ],
    }


def from_json_global_declaration(value: Json) -> GlobalDeclaration:
    """Return one GlobalDeclaration from one JSON value."""
    object_ = json_object(value)

    return GlobalDeclaration(
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        statements=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "statements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TypeDeclaration:
    """A type alias declaration."""

    # the declared name
    name: destack._generated.js.tree.key.Name | None
    # the export binding of the declaration
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    # whether the declaration is ambient
    is_ambient: bool
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the declared type value
    value: destack._generated.js.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeDeclaration:
        """Decode one TypeDeclaration."""
        return decode_type_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeDeclaration:
        """Return one TypeDeclaration from one JSON value."""
        return from_json_type_declaration(value)


def encode_type_declaration(writer: BinaryWriter, value: TypeDeclaration) -> None:
    """Encode one TypeDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.dependency.encode_dependency_binding(
            writer, value.export
        )
    writer.write_bool(value.is_ambient)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    destack._generated.js.tree.node.encode_local_node_id(writer, value.value)


def decode_type_declaration(reader: BinaryReader) -> TypeDeclaration:
    """Decode one TypeDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.js.tree.dependency.decode_dependency_binding(reader)
    )
    is_ambient = reader.read_bool()
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    value_ = destack._generated.js.tree.node.decode_local_node_id(reader)

    return TypeDeclaration(
        name=name,
        export=export,
        is_ambient=is_ambient,
        generic_parameters=generic_parameters,
        value=value_,
    )


def to_json_type_declaration(value: TypeDeclaration) -> Json:
    """Return one JSON value for one TypeDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                    value.export
                )
            }
        ),
        "isAmbient": value.is_ambient,
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "value": destack._generated.js.tree.node.to_json_local_node_id(value.value),
    }


def from_json_type_declaration(value: Json) -> TypeDeclaration:
    """Return one TypeDeclaration from one JSON value."""
    object_ = json_object(value)

    return TypeDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: (
                destack._generated.js.tree.dependency.from_json_dependency_binding(
                    value
                )
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        value=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class ClassDeclaration:
    """A class declaration."""

    # the declared name
    name: destack._generated.js.tree.key.Name | None
    # the export binding of the declaration
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    # whether the declaration is ambient
    is_ambient: bool
    # whether the declaration is abstract
    is_abstract: bool
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the optional extended class expression
    extends_expression: destack._generated.js.tree.node.LocalNodeId | None
    # the generic arguments applied to the extended class expression
    extends_generic_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the implemented interface types
    implements_types: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the class members
    members: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassDeclaration:
        """Decode one ClassDeclaration."""
        return decode_class_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> ClassDeclaration:
        """Return one ClassDeclaration from one JSON value."""
        return from_json_class_declaration(value)


def encode_class_declaration(writer: BinaryWriter, value: ClassDeclaration) -> None:
    """Encode one ClassDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.dependency.encode_dependency_binding(
            writer, value.export
        )
    writer.write_bool(value.is_ambient)
    writer.write_bool(value.is_abstract)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    if value.extends_expression is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(
            writer, value.extends_expression
        )
    writer.write_unsigned(len(value.extends_generic_arguments))
    for item_value_extends_generic_arguments_0 in value.extends_generic_arguments:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_extends_generic_arguments_0
        )
    writer.write_unsigned(len(value.implements_types))
    for item_value_implements_types_0 in value.implements_types:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_implements_types_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )


def decode_class_declaration(reader: BinaryReader) -> ClassDeclaration:
    """Decode one ClassDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.js.tree.dependency.decode_dependency_binding(reader)
    )
    is_ambient = reader.read_bool()
    is_abstract = reader.read_bool()
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    extends_expression = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )
    extends_generic_arguments = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    implements_types = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return ClassDeclaration(
        name=name,
        export=export,
        is_ambient=is_ambient,
        is_abstract=is_abstract,
        generic_parameters=generic_parameters,
        extends_expression=extends_expression,
        extends_generic_arguments=extends_generic_arguments,
        implements_types=implements_types,
        members=members,
    )


def to_json_class_declaration(value: ClassDeclaration) -> Json:
    """Return one JSON value for one ClassDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                    value.export
                )
            }
        ),
        "isAmbient": value.is_ambient,
        "isAbstract": value.is_abstract,
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        **(
            {}
            if value.extends_expression is None
            else {
                "extendsExpression": destack._generated.js.tree.node.to_json_local_node_id(
                    value.extends_expression
                )
            }
        ),
        "extendsGenericArguments": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.extends_generic_arguments
        ],
        "implementsTypes": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.implements_types
        ],
        "members": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
    }


def from_json_class_declaration(value: Json) -> ClassDeclaration:
    """Return one ClassDeclaration from one JSON value."""
    object_ = json_object(value)

    return ClassDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: (
                destack._generated.js.tree.dependency.from_json_dependency_binding(
                    value
                )
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        extends_expression=json_optional(
            object_,
            "extendsExpression",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
        extends_generic_arguments=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "extendsGenericArguments"))
        ],
        implements_types=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "implementsTypes"))
        ],
        members=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InterfaceDeclaration:
    """An interface declaration."""

    # the declared name
    name: destack._generated.js.tree.key.Name | None
    # the export binding of the declaration
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    # whether the declaration is ambient
    is_ambient: bool
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the extended interfaces
    extends: Sequence[InterfaceHeritage]
    # the interface members
    members: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_interface_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceDeclaration:
        """Decode one InterfaceDeclaration."""
        return decode_interface_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_interface_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> InterfaceDeclaration:
        """Return one InterfaceDeclaration from one JSON value."""
        return from_json_interface_declaration(value)


def encode_interface_declaration(
    writer: BinaryWriter, value: InterfaceDeclaration
) -> None:
    """Encode one InterfaceDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.dependency.encode_dependency_binding(
            writer, value.export
        )
    writer.write_bool(value.is_ambient)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.extends))
    for item_value_extends_0 in value.extends:
        encode_interface_heritage(writer, item_value_extends_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )


def decode_interface_declaration(reader: BinaryReader) -> InterfaceDeclaration:
    """Decode one InterfaceDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.js.tree.dependency.decode_dependency_binding(reader)
    )
    is_ambient = reader.read_bool()
    generic_parameters = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    extends = [decode_interface_heritage(reader) for _ in range(reader.read_number())]
    members = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return InterfaceDeclaration(
        name=name,
        export=export,
        is_ambient=is_ambient,
        generic_parameters=generic_parameters,
        extends=extends,
        members=members,
    )


def to_json_interface_declaration(value: InterfaceDeclaration) -> Json:
    """Return one JSON value for one InterfaceDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                    value.export
                )
            }
        ),
        "isAmbient": value.is_ambient,
        "genericParameters": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "extends": [to_json_interface_heritage(item_0) for item_0 in value.extends],
        "members": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
    }


def from_json_interface_declaration(value: Json) -> InterfaceDeclaration:
    """Return one InterfaceDeclaration from one JSON value."""
    object_ = json_object(value)

    return InterfaceDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: (
                destack._generated.js.tree.dependency.from_json_dependency_binding(
                    value
                )
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        generic_parameters=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        extends=[
            from_json_interface_heritage(item_0)
            for item_0 in json_array(json_field(object_, "extends"))
        ],
        members=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InterfaceHeritage:
    """One interface heritage clause item."""

    # the extended interface expression
    expression: destack._generated.js.tree.node.LocalNodeId
    # the type arguments applied to the extended interface expression
    type_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_interface_heritage(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceHeritage:
        """Decode one InterfaceHeritage."""
        return decode_interface_heritage(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_interface_heritage(self)

    @classmethod
    def from_json(cls, value: Json) -> InterfaceHeritage:
        """Return one InterfaceHeritage from one JSON value."""
        return from_json_interface_heritage(value)


def encode_interface_heritage(writer: BinaryWriter, value: InterfaceHeritage) -> None:
    """Encode one InterfaceHeritage."""
    destack._generated.js.tree.node.encode_local_node_id(writer, value.expression)
    writer.write_unsigned(len(value.type_arguments))
    for item_value_type_arguments_0 in value.type_arguments:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_type_arguments_0
        )


def decode_interface_heritage(reader: BinaryReader) -> InterfaceHeritage:
    """Decode one InterfaceHeritage."""
    expression = destack._generated.js.tree.node.decode_local_node_id(reader)
    type_arguments = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return InterfaceHeritage(
        expression=expression,
        type_arguments=type_arguments,
    )


def to_json_interface_heritage(value: InterfaceHeritage) -> Json:
    """Return one JSON value for one InterfaceHeritage."""
    return {
        "expression": destack._generated.js.tree.node.to_json_local_node_id(
            value.expression
        ),
        "typeArguments": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.type_arguments
        ],
    }


def from_json_interface_heritage(value: Json) -> InterfaceHeritage:
    """Return one InterfaceHeritage from one JSON value."""
    object_ = json_object(value)

    return InterfaceHeritage(
        expression=destack._generated.js.tree.node.from_json_local_node_id(
            json_field(object_, "expression")
        ),
        type_arguments=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "typeArguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class EnumDeclaration:
    """An enum declaration."""

    # the declared name
    name: destack._generated.js.tree.key.Name | None
    # the export binding of the declaration
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    # whether the declaration is ambient
    is_ambient: bool
    # the enum fields
    fields: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_enum_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumDeclaration:
        """Decode one EnumDeclaration."""
        return decode_enum_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_enum_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> EnumDeclaration:
        """Return one EnumDeclaration from one JSON value."""
        return from_json_enum_declaration(value)


def encode_enum_declaration(writer: BinaryWriter, value: EnumDeclaration) -> None:
    """Encode one EnumDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.dependency.encode_dependency_binding(
            writer, value.export
        )
    writer.write_bool(value.is_ambient)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        destack._generated.js.tree.node.encode_local_node_id(
            writer, item_value_fields_0
        )


def decode_enum_declaration(reader: BinaryReader) -> EnumDeclaration:
    """Decode one EnumDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.js.tree.dependency.decode_dependency_binding(reader)
    )
    is_ambient = reader.read_bool()
    fields = [
        destack._generated.js.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return EnumDeclaration(
        name=name,
        export=export,
        is_ambient=is_ambient,
        fields=fields,
    )


def to_json_enum_declaration(value: EnumDeclaration) -> Json:
    """Return one JSON value for one EnumDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                    value.export
                )
            }
        ),
        "isAmbient": value.is_ambient,
        "fields": [
            destack._generated.js.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.fields
        ],
    }


def from_json_enum_declaration(value: Json) -> EnumDeclaration:
    """Return one EnumDeclaration from one JSON value."""
    object_ = json_object(value)

    return EnumDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: (
                destack._generated.js.tree.dependency.from_json_dependency_binding(
                    value
                )
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        fields=[
            destack._generated.js.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FunctionDeclaration:
    """A function declaration."""

    # the declared name
    name: destack._generated.js.tree.key.Name | None
    # the export binding of the declaration
    export: destack._generated.js.tree.dependency.DependencyBinding | None
    # whether the declaration is ambient
    is_ambient: bool
    # whether the declaration is abstract
    is_abstract: bool
    # the function signature
    signature: destack._generated.js.tree.function.FunctionSignature
    # the optional function body
    body: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionDeclaration:
        """Decode one FunctionDeclaration."""
        return decode_function_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionDeclaration:
        """Return one FunctionDeclaration from one JSON value."""
        return from_json_function_declaration(value)


def encode_function_declaration(
    writer: BinaryWriter, value: FunctionDeclaration
) -> None:
    """Encode one FunctionDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.dependency.encode_dependency_binding(
            writer, value.export
        )
    writer.write_bool(value.is_ambient)
    writer.write_bool(value.is_abstract)
    destack._generated.js.tree.function.encode_function_signature(
        writer, value.signature
    )
    if value.body is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.body)


def decode_function_declaration(reader: BinaryReader) -> FunctionDeclaration:
    """Decode one FunctionDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.js.tree.dependency.decode_dependency_binding(reader)
    )
    is_ambient = reader.read_bool()
    is_abstract = reader.read_bool()
    signature = destack._generated.js.tree.function.decode_function_signature(reader)
    body = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return FunctionDeclaration(
        name=name,
        export=export,
        is_ambient=is_ambient,
        is_abstract=is_abstract,
        signature=signature,
        body=body,
    )


def to_json_function_declaration(value: FunctionDeclaration) -> Json:
    """Return one JSON value for one FunctionDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.js.tree.dependency.to_json_dependency_binding(
                    value.export
                )
            }
        ),
        "isAmbient": value.is_ambient,
        "isAbstract": value.is_abstract,
        "signature": destack._generated.js.tree.function.to_json_function_signature(
            value.signature
        ),
        **(
            {}
            if value.body is None
            else {
                "body": destack._generated.js.tree.node.to_json_local_node_id(
                    value.body
                )
            }
        ),
    }


def from_json_function_declaration(value: Json) -> FunctionDeclaration:
    """Return one FunctionDeclaration from one JSON value."""
    object_ = json_object(value)

    return FunctionDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: (
                destack._generated.js.tree.dependency.from_json_dependency_binding(
                    value
                )
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        signature=destack._generated.js.tree.function.from_json_function_signature(
            json_field(object_, "signature")
        ),
        body=json_optional(
            object_,
            "body",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class EnumField:
    """An EnumField is a named field of an enum declaration."""

    # the name of the enum field
    name: destack._generated.core.string.StringId
    # the value of the enum field
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_enum_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumField:
        """Decode one EnumField."""
        return decode_enum_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_enum_field(self)

    @classmethod
    def from_json(cls, value: Json) -> EnumField:
        """Return one EnumField from one JSON value."""
        return from_json_enum_field(value)


def encode_enum_field(writer: BinaryWriter, value: EnumField) -> None:
    """Encode one EnumField."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)


def decode_enum_field(reader: BinaryReader) -> EnumField:
    """Decode one EnumField."""
    name = destack._generated.core.string.decode_string_id(reader)
    value_ = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return EnumField(
        name=name,
        value=value_,
    )


def to_json_enum_field(value: EnumField) -> Json:
    """Return one JSON value for one EnumField."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.js.tree.node.to_json_local_node_id(
                    value.value
                )
            }
        ),
    }


def from_json_enum_field(value: Json) -> EnumField:
    """Return one EnumField from one JSON value."""
    object_ = json_object(value)

    return EnumField(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


__all__ = [
    "Declaration",
    "encode_declaration",
    "decode_declaration",
    "to_json_declaration",
    "from_json_declaration",
    "DeclarationGlobal",
    "DeclarationType",
    "DeclarationClass",
    "DeclarationInterface",
    "DeclarationEnum",
    "DeclarationFunction",
    "GlobalDeclaration",
    "encode_global_declaration",
    "decode_global_declaration",
    "to_json_global_declaration",
    "from_json_global_declaration",
    "TypeDeclaration",
    "encode_type_declaration",
    "decode_type_declaration",
    "to_json_type_declaration",
    "from_json_type_declaration",
    "ClassDeclaration",
    "encode_class_declaration",
    "decode_class_declaration",
    "to_json_class_declaration",
    "from_json_class_declaration",
    "InterfaceDeclaration",
    "encode_interface_declaration",
    "decode_interface_declaration",
    "to_json_interface_declaration",
    "from_json_interface_declaration",
    "InterfaceHeritage",
    "encode_interface_heritage",
    "decode_interface_heritage",
    "to_json_interface_heritage",
    "from_json_interface_heritage",
    "EnumDeclaration",
    "encode_enum_declaration",
    "decode_enum_declaration",
    "to_json_enum_declaration",
    "from_json_enum_declaration",
    "FunctionDeclaration",
    "encode_function_declaration",
    "decode_function_declaration",
    "to_json_function_declaration",
    "from_json_function_declaration",
    "EnumField",
    "encode_enum_field",
    "decode_enum_field",
    "to_json_enum_field",
    "from_json_enum_field",
]
