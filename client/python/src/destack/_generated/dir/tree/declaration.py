# generated client target, do not edit

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

from destack._impl.dir.tree.declaration import (
    DeclarationImpl,
)

import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.function
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node


@dataclass(frozen=True, slots=True)
class DeclarationGlobal(DeclarationImpl):
    """Global declaration block."""

    global_: GlobalDeclaration
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationModule(DeclarationImpl):
    """Module declaration block."""

    module: ModuleDeclaration
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationType(DeclarationImpl):
    """Type declaration."""

    type: TypeDeclaration
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationStruct(DeclarationImpl):
    """Struct declaration."""

    struct: StructDeclaration
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationClass(DeclarationImpl):
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
class DeclarationEnum(DeclarationImpl):
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
class DeclarationInterface(DeclarationImpl):
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
class DeclarationExtension(DeclarationImpl):
    """Extension declaration."""

    extension: ExtensionDeclaration
    kind: typing.Literal["extension"] = "extension"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


@dataclass(frozen=True, slots=True)
class DeclarationFunction(DeclarationImpl):
    """Function declaration."""

    function: FunctionDeclaration
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_declaration(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_declaration(self)


"""Declaration introduces a type or such into a scope."""
Declaration: typing.TypeAlias = (
    DeclarationGlobal
    | DeclarationModule
    | DeclarationType
    | DeclarationStruct
    | DeclarationClass
    | DeclarationEnum
    | DeclarationInterface
    | DeclarationExtension
    | DeclarationFunction
)


def encode_declaration(writer: BinaryWriter, value: Declaration) -> None:
    """Encode one Declaration."""
    if value.kind == "global":
        writer.write_unsigned(0)
        encode_global_declaration(writer, value.global_)
    elif value.kind == "module":
        writer.write_unsigned(1)
        encode_module_declaration(writer, value.module)
    elif value.kind == "type":
        writer.write_unsigned(2)
        encode_type_declaration(writer, value.type)
    elif value.kind == "struct":
        writer.write_unsigned(3)
        encode_struct_declaration(writer, value.struct)
    elif value.kind == "class":
        writer.write_unsigned(4)
        encode_class_declaration(writer, value.class_)
    elif value.kind == "enum":
        writer.write_unsigned(5)
        encode_enum_declaration(writer, value.enum)
    elif value.kind == "interface":
        writer.write_unsigned(6)
        encode_interface_declaration(writer, value.interface)
    elif value.kind == "extension":
        writer.write_unsigned(7)
        encode_extension_declaration(writer, value.extension)
    elif value.kind == "function":
        writer.write_unsigned(8)
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
        module = decode_module_declaration(reader)

        return DeclarationModule(module=module)
    elif variant == 2:
        type = decode_type_declaration(reader)

        return DeclarationType(type=type)
    elif variant == 3:
        struct = decode_struct_declaration(reader)

        return DeclarationStruct(struct=struct)
    elif variant == 4:
        class_ = decode_class_declaration(reader)

        return DeclarationClass(class_=class_)
    elif variant == 5:
        enum = decode_enum_declaration(reader)

        return DeclarationEnum(enum=enum)
    elif variant == 6:
        interface = decode_interface_declaration(reader)

        return DeclarationInterface(interface=interface)
    elif variant == 7:
        extension = decode_extension_declaration(reader)

        return DeclarationExtension(extension=extension)
    elif variant == 8:
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
    elif value.kind == "module":
        return {
            "kind": "module",
            "module": to_json_module_declaration(value.module),
        }
    elif value.kind == "type":
        return {
            "kind": "type",
            "type": to_json_type_declaration(value.type),
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "struct": to_json_struct_declaration(value.struct),
        }
    elif value.kind == "class":
        return {
            "kind": "class",
            "class": to_json_class_declaration(value.class_),
        }
    elif value.kind == "enum":
        return {
            "kind": "enum",
            "enum": to_json_enum_declaration(value.enum),
        }
    elif value.kind == "interface":
        return {
            "kind": "interface",
            "interface": to_json_interface_declaration(value.interface),
        }
    elif value.kind == "extension":
        return {
            "kind": "extension",
            "extension": to_json_extension_declaration(value.extension),
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
    elif kind == "module":
        return DeclarationModule(
            module=from_json_module_declaration(json_field(object_, "module"))
        )
    elif kind == "type":
        return DeclarationType(
            type=from_json_type_declaration(json_field(object_, "type"))
        )
    elif kind == "struct":
        return DeclarationStruct(
            struct=from_json_struct_declaration(json_field(object_, "struct"))
        )
    elif kind == "class":
        return DeclarationClass(
            class_=from_json_class_declaration(json_field(object_, "class"))
        )
    elif kind == "enum":
        return DeclarationEnum(
            enum=from_json_enum_declaration(json_field(object_, "enum"))
        )
    elif kind == "interface":
        return DeclarationInterface(
            interface=from_json_interface_declaration(json_field(object_, "interface"))
        )
    elif kind == "extension":
        return DeclarationExtension(
            extension=from_json_extension_declaration(json_field(object_, "extension"))
        )
    elif kind == "function":
        return DeclarationFunction(
            function=from_json_function_declaration(json_field(object_, "function"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GlobalDeclaration:
    """A global declaration block."""

    # the expressions inside the global body
    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool

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
    writer.write_unsigned(len(value.expressions))
    for item_value_expressions_0 in value.expressions:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_expressions_0
        )
    writer.write_bool(value.is_ambient)


def decode_global_declaration(reader: BinaryReader) -> GlobalDeclaration:
    """Decode one GlobalDeclaration."""
    expressions = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()

    return GlobalDeclaration(
        expressions=expressions,
        is_ambient=is_ambient,
    )


def to_json_global_declaration(value: GlobalDeclaration) -> Json:
    """Return one JSON value for one GlobalDeclaration."""
    return {
        "expressions": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.expressions
        ],
        "isAmbient": value.is_ambient,
    }


def from_json_global_declaration(value: Json) -> GlobalDeclaration:
    """Return one GlobalDeclaration from one JSON value."""
    object_ = json_object(value)

    return GlobalDeclaration(
        expressions=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "expressions"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
    )


@dataclass(frozen=True, slots=True)
class ModuleDeclaration:
    """A module declaration block."""

    # the expressions inside the module body
    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleDeclaration:
        """Decode one ModuleDeclaration."""
        return decode_module_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleDeclaration:
        """Return one ModuleDeclaration from one JSON value."""
        return from_json_module_declaration(value)


def encode_module_declaration(writer: BinaryWriter, value: ModuleDeclaration) -> None:
    """Encode one ModuleDeclaration."""
    writer.write_unsigned(len(value.expressions))
    for item_value_expressions_0 in value.expressions:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_expressions_0
        )


def decode_module_declaration(reader: BinaryReader) -> ModuleDeclaration:
    """Decode one ModuleDeclaration."""
    expressions = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]

    return ModuleDeclaration(
        expressions=expressions,
    )


def to_json_module_declaration(value: ModuleDeclaration) -> Json:
    """Return one JSON value for one ModuleDeclaration."""
    return {
        "expressions": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.expressions
        ],
    }


def from_json_module_declaration(value: Json) -> ModuleDeclaration:
    """Return one ModuleDeclaration from one JSON value."""
    object_ = json_object(value)

    return ModuleDeclaration(
        expressions=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "expressions"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TypeDeclaration:
    """A type declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the optional mutability qualifier
    mutability: destack._generated.dir.tree.node.Mutability | None
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the declared type expression
    value: destack._generated.dir.tree.node.LocalNodeId
    # whether the declaration is ambient
    is_ambient: bool
    # whether the declaration is nominal
    is_nominal: bool

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
    destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    if value.mutability is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_mutability(writer, value.mutability)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    writer.write_bool(value.is_ambient)
    writer.write_bool(value.is_nominal)


def decode_type_declaration(reader: BinaryReader) -> TypeDeclaration:
    """Decode one TypeDeclaration."""
    name = destack._generated.dir.tree.key.decode_name(reader)
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    mutability = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_mutability(reader)
    )
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    value_ = destack._generated.dir.tree.node.decode_local_node_id(reader)
    is_ambient = reader.read_bool()
    is_nominal = reader.read_bool()

    return TypeDeclaration(
        name=name,
        export=export,
        mutability=mutability,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        value=value_,
        is_ambient=is_ambient,
        is_nominal=is_nominal,
    )


def to_json_type_declaration(value: TypeDeclaration) -> Json:
    """Return one JSON value for one TypeDeclaration."""
    return {
        "name": destack._generated.dir.tree.key.to_json_name(value.name),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        **(
            {}
            if value.mutability is None
            else {
                "mutability": destack._generated.dir.tree.node.to_json_mutability(
                    value.mutability
                )
            }
        ),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "value": destack._generated.dir.tree.node.to_json_local_node_id(value.value),
        "isAmbient": value.is_ambient,
        "isNominal": value.is_nominal,
    }


def from_json_type_declaration(value: Json) -> TypeDeclaration:
    """Return one TypeDeclaration from one JSON value."""
    object_ = json_object(value)

    return TypeDeclaration(
        name=destack._generated.dir.tree.key.from_json_name(
            json_field(object_, "name")
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        mutability=json_optional(
            object_,
            "mutability",
            lambda value: destack._generated.dir.tree.node.from_json_mutability(value),
        ),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        value=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "value")
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        is_nominal=json_bool(json_field(object_, "isNominal")),
    )


@dataclass(frozen=True, slots=True)
class StructDeclaration:
    """A struct declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the implemented interfaces
    implements_types: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the struct members
    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_struct_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StructDeclaration:
        """Decode one StructDeclaration."""
        return decode_struct_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_struct_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> StructDeclaration:
        """Return one StructDeclaration from one JSON value."""
        return from_json_struct_declaration(value)


def encode_struct_declaration(writer: BinaryWriter, value: StructDeclaration) -> None:
    """Encode one StructDeclaration."""
    destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    writer.write_unsigned(len(value.implements_types))
    for item_value_implements_types_0 in value.implements_types:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_implements_types_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )
    writer.write_bool(value.is_ambient)


def decode_struct_declaration(reader: BinaryReader) -> StructDeclaration:
    """Decode one StructDeclaration."""
    name = destack._generated.dir.tree.key.decode_name(reader)
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    implements_types = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()

    return StructDeclaration(
        name=name,
        export=export,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        implements_types=implements_types,
        members=members,
        is_ambient=is_ambient,
    )


def to_json_struct_declaration(value: StructDeclaration) -> Json:
    """Return one JSON value for one StructDeclaration."""
    return {
        "name": destack._generated.dir.tree.key.to_json_name(value.name),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "implementsTypes": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.implements_types
        ],
        "members": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
        "isAmbient": value.is_ambient,
    }


def from_json_struct_declaration(value: Json) -> StructDeclaration:
    """Return one StructDeclaration from one JSON value."""
    object_ = json_object(value)

    return StructDeclaration(
        name=destack._generated.dir.tree.key.from_json_name(
            json_field(object_, "name")
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        implements_types=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "implementsTypes"))
        ],
        members=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
    )


@dataclass(frozen=True, slots=True)
class ClassDeclaration:
    """A class declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the extended class type
    extends_type: destack._generated.dir.tree.node.LocalNodeId | None
    # the implemented interfaces
    implements_types: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the class members
    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool
    # whether the declaration is abstract
    is_abstract: bool
    # whether the declaration is final
    is_final: bool

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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    if value.extends_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, value.extends_type
        )
    writer.write_unsigned(len(value.implements_types))
    for item_value_implements_types_0 in value.implements_types:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_implements_types_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )
    writer.write_bool(value.is_ambient)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_final)


def decode_class_declaration(reader: BinaryReader) -> ClassDeclaration:
    """Decode one ClassDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.dir.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    extends_type = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    implements_types = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()
    is_abstract = reader.read_bool()
    is_final = reader.read_bool()

    return ClassDeclaration(
        name=name,
        export=export,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        extends_type=extends_type,
        implements_types=implements_types,
        members=members,
        is_ambient=is_ambient,
        is_abstract=is_abstract,
        is_final=is_final,
    )


def to_json_class_declaration(value: ClassDeclaration) -> Json:
    """Return one JSON value for one ClassDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        **(
            {}
            if value.extends_type is None
            else {
                "extendsType": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.extends_type
                )
            }
        ),
        "implementsTypes": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.implements_types
        ],
        "members": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
        "isAmbient": value.is_ambient,
        "isAbstract": value.is_abstract,
        "isFinal": value.is_final,
    }


def from_json_class_declaration(value: Json) -> ClassDeclaration:
    """Return one ClassDeclaration from one JSON value."""
    object_ = json_object(value)

    return ClassDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.dir.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        extends_type=json_optional(
            object_,
            "extendsType",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        implements_types=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "implementsTypes"))
        ],
        members=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_final=json_bool(json_field(object_, "isFinal")),
    )


@dataclass(frozen=True, slots=True)
class EnumDeclaration:
    """An enum declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the enum kind
    kind: EnumKind
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the implemented interfaces
    implements_types: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the enum fields
    fields: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the enum members
    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool

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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    encode_enum_kind(writer, value.kind)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    writer.write_unsigned(len(value.implements_types))
    for item_value_implements_types_0 in value.implements_types:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_implements_types_0
        )
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_fields_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )
    writer.write_bool(value.is_ambient)


def decode_enum_declaration(reader: BinaryReader) -> EnumDeclaration:
    """Decode one EnumDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.dir.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    kind = decode_enum_kind(reader)
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    implements_types = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    fields = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()

    return EnumDeclaration(
        name=name,
        export=export,
        kind=kind,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        implements_types=implements_types,
        fields=fields,
        members=members,
        is_ambient=is_ambient,
    )


def to_json_enum_declaration(value: EnumDeclaration) -> Json:
    """Return one JSON value for one EnumDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "kind": to_json_enum_kind(value.kind),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "implementsTypes": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.implements_types
        ],
        "fields": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.fields
        ],
        "members": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
        "isAmbient": value.is_ambient,
    }


def from_json_enum_declaration(value: Json) -> EnumDeclaration:
    """Return one EnumDeclaration from one JSON value."""
    object_ = json_object(value)

    return EnumDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.dir.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        kind=from_json_enum_kind(json_field(object_, "kind")),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        implements_types=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "implementsTypes"))
        ],
        fields=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        members=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
    )


"""The kind of an enum declaration."""
EnumKind: typing.TypeAlias = typing.Literal["enum"] | typing.Literal["const"]


def encode_enum_kind(writer: BinaryWriter, value: EnumKind) -> None:
    """Encode one EnumKind."""
    if value == "enum":
        writer.write_unsigned(0)
    elif value == "const":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_enum_kind(reader: BinaryReader) -> EnumKind:
    """Decode one EnumKind."""
    variant = reader.read_number()

    if variant == 0:
        return "enum"
    elif variant == 1:
        return "const"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_enum_kind(value: EnumKind) -> Json:
    """Return one JSON value for one EnumKind."""
    return value


def from_json_enum_kind(value: Json) -> EnumKind:
    """Return one EnumKind from one JSON value."""
    variant = json_string(value)

    if variant == "enum":
        return "enum"
    elif variant == "const":
        return "const"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class InterfaceDeclaration:
    """An interface declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the extended interface types
    extends_types: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the interface members
    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool
    # whether the interface is nominal
    is_nominal: bool

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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    writer.write_unsigned(len(value.extends_types))
    for item_value_extends_types_0 in value.extends_types:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_extends_types_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )
    writer.write_bool(value.is_ambient)
    writer.write_bool(value.is_nominal)


def decode_interface_declaration(reader: BinaryReader) -> InterfaceDeclaration:
    """Decode one InterfaceDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.dir.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    extends_types = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()
    is_nominal = reader.read_bool()

    return InterfaceDeclaration(
        name=name,
        export=export,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        extends_types=extends_types,
        members=members,
        is_ambient=is_ambient,
        is_nominal=is_nominal,
    )


def to_json_interface_declaration(value: InterfaceDeclaration) -> Json:
    """Return one JSON value for one InterfaceDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "extendsTypes": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.extends_types
        ],
        "members": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
        "isAmbient": value.is_ambient,
        "isNominal": value.is_nominal,
    }


def from_json_interface_declaration(value: Json) -> InterfaceDeclaration:
    """Return one InterfaceDeclaration from one JSON value."""
    object_ = json_object(value)

    return InterfaceDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.dir.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        extends_types=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "extendsTypes"))
        ],
        members=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
        is_nominal=json_bool(json_field(object_, "isNominal")),
    )


@dataclass(frozen=True, slots=True)
class ExtensionDeclaration:
    """An extension declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the generic parameters of the declaration
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the declaration
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the extended target type
    target_type: destack._generated.dir.tree.node.LocalNodeId
    # the implemented interfaces
    implements_types: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the extension members
    members: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_declaration(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionDeclaration:
        """Decode one ExtensionDeclaration."""
        return decode_extension_declaration(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_declaration(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionDeclaration:
        """Return one ExtensionDeclaration from one JSON value."""
        return from_json_extension_declaration(value)


def encode_extension_declaration(
    writer: BinaryWriter, value: ExtensionDeclaration
) -> None:
    """Encode one ExtensionDeclaration."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_where_clauses_0
        )
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.target_type)
    writer.write_unsigned(len(value.implements_types))
    for item_value_implements_types_0 in value.implements_types:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_implements_types_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.node.encode_local_node_id(
            writer, item_value_members_0
        )
    writer.write_bool(value.is_ambient)


def decode_extension_declaration(reader: BinaryReader) -> ExtensionDeclaration:
    """Decode one ExtensionDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.dir.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    generic_parameters = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    target_type = destack._generated.dir.tree.node.decode_local_node_id(reader)
    implements_types = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.node.decode_local_node_id(reader)
        for _ in range(reader.read_number())
    ]
    is_ambient = reader.read_bool()

    return ExtensionDeclaration(
        name=name,
        export=export,
        generic_parameters=generic_parameters,
        where_clauses=where_clauses,
        target_type=target_type,
        implements_types=implements_types,
        members=members,
        is_ambient=is_ambient,
    )


def to_json_extension_declaration(value: ExtensionDeclaration) -> Json:
    """Return one JSON value for one ExtensionDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "genericParameters": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.generic_parameters
        ],
        "whereClauses": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.where_clauses
        ],
        "targetType": destack._generated.dir.tree.node.to_json_local_node_id(
            value.target_type
        ),
        "implementsTypes": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.implements_types
        ],
        "members": [
            destack._generated.dir.tree.node.to_json_local_node_id(item_0)
            for item_0 in value.members
        ],
        "isAmbient": value.is_ambient,
    }


def from_json_extension_declaration(value: Json) -> ExtensionDeclaration:
    """Return one ExtensionDeclaration from one JSON value."""
    object_ = json_object(value)

    return ExtensionDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.dir.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        generic_parameters=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        where_clauses=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        target_type=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "targetType")
        ),
        implements_types=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "implementsTypes"))
        ],
        members=[
            destack._generated.dir.tree.node.from_json_local_node_id(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        is_ambient=json_bool(json_field(object_, "isAmbient")),
    )


@dataclass(frozen=True, slots=True)
class FunctionDeclaration:
    """A function declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the function signature
    signature: destack._generated.dir.tree.function.FunctionSignature
    # the optional function body
    body: destack._generated.dir.tree.node.LocalNodeId | None
    # whether the declaration is ambient
    is_ambient: bool

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
        destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.export is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(writer, value.export)
    destack._generated.dir.tree.function.encode_function_signature(
        writer, value.signature
    )
    if value.body is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.body)
    writer.write_bool(value.is_ambient)


def decode_function_declaration(reader: BinaryReader) -> FunctionDeclaration:
    """Decode one FunctionDeclaration."""
    name = reader.read_option(
        lambda: destack._generated.dir.tree.key.decode_name(reader)
    )
    export = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    signature = destack._generated.dir.tree.function.decode_function_signature(reader)
    body = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )
    is_ambient = reader.read_bool()

    return FunctionDeclaration(
        name=name,
        export=export,
        signature=signature,
        body=body,
        is_ambient=is_ambient,
    )


def to_json_function_declaration(value: FunctionDeclaration) -> Json:
    """Return one JSON value for one FunctionDeclaration."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.export is None
            else {
                "export": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export
                )
            }
        ),
        "signature": destack._generated.dir.tree.function.to_json_function_signature(
            value.signature
        ),
        **(
            {}
            if value.body is None
            else {
                "body": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.body
                )
            }
        ),
        "isAmbient": value.is_ambient,
    }


def from_json_function_declaration(value: Json) -> FunctionDeclaration:
    """Return one FunctionDeclaration from one JSON value."""
    object_ = json_object(value)

    return FunctionDeclaration(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.dir.tree.key.from_json_name(value),
        ),
        export=json_optional(
            object_,
            "export",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        signature=destack._generated.dir.tree.function.from_json_function_signature(
            json_field(object_, "signature")
        ),
        body=json_optional(
            object_,
            "body",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
        is_ambient=json_bool(json_field(object_, "isAmbient")),
    )


@dataclass(frozen=True, slots=True)
class EnumField:
    """An enum field."""

    # the name of the enum field
    name: destack._generated.dir.tree.key.Name
    # the default value of the enum field
    value: destack._generated.dir.tree.node.LocalNodeId | None

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
    destack._generated.dir.tree.key.encode_name(writer, value.name)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)


def decode_enum_field(reader: BinaryReader) -> EnumField:
    """Decode one EnumField."""
    name = destack._generated.dir.tree.key.decode_name(reader)
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )

    return EnumField(
        name=name,
        value=value_,
    )


def to_json_enum_field(value: EnumField) -> Json:
    """Return one JSON value for one EnumField."""
    return {
        "name": destack._generated.dir.tree.key.to_json_name(value.name),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.value
                )
            }
        ),
    }


def from_json_enum_field(value: Json) -> EnumField:
    """Return one EnumField from one JSON value."""
    object_ = json_object(value)

    return EnumField(
        name=destack._generated.dir.tree.key.from_json_name(
            json_field(object_, "name")
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
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
    "DeclarationModule",
    "DeclarationType",
    "DeclarationStruct",
    "DeclarationClass",
    "DeclarationEnum",
    "DeclarationInterface",
    "DeclarationExtension",
    "DeclarationFunction",
    "GlobalDeclaration",
    "encode_global_declaration",
    "decode_global_declaration",
    "to_json_global_declaration",
    "from_json_global_declaration",
    "ModuleDeclaration",
    "encode_module_declaration",
    "decode_module_declaration",
    "to_json_module_declaration",
    "from_json_module_declaration",
    "TypeDeclaration",
    "encode_type_declaration",
    "decode_type_declaration",
    "to_json_type_declaration",
    "from_json_type_declaration",
    "StructDeclaration",
    "encode_struct_declaration",
    "decode_struct_declaration",
    "to_json_struct_declaration",
    "from_json_struct_declaration",
    "ClassDeclaration",
    "encode_class_declaration",
    "decode_class_declaration",
    "to_json_class_declaration",
    "from_json_class_declaration",
    "EnumDeclaration",
    "encode_enum_declaration",
    "decode_enum_declaration",
    "to_json_enum_declaration",
    "from_json_enum_declaration",
    "EnumKind",
    "encode_enum_kind",
    "decode_enum_kind",
    "to_json_enum_kind",
    "from_json_enum_kind",
    "InterfaceDeclaration",
    "encode_interface_declaration",
    "decode_interface_declaration",
    "to_json_interface_declaration",
    "from_json_interface_declaration",
    "ExtensionDeclaration",
    "encode_extension_declaration",
    "decode_extension_declaration",
    "to_json_extension_declaration",
    "from_json_extension_declaration",
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
