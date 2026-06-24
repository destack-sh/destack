# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationType:
    """Type alias declaration."""

    type: TypeDeclaration
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationClass:
    """Class declaration."""

    class_: ClassDeclaration
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationInterface:
    """Interface declaration."""

    interface: InterfaceDeclaration
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationEnum:
    """Enum declaration."""

    enum: EnumDeclaration
    kind: typing.Literal["enum"] = "enum"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationFunction:
    """Function declaration."""

    function: FunctionDeclaration
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""A declaration item."""
Declaration: typing.TypeAlias = (
    DeclarationGlobal
    | DeclarationType
    | DeclarationClass
    | DeclarationInterface
    | DeclarationEnum
    | DeclarationFunction
)

def encode_declaration(writer: BinaryWriter, value: Declaration) -> None: ...
def decode_declaration(reader: BinaryReader) -> Declaration: ...
def to_json_declaration(value: Declaration) -> Json: ...
def from_json_declaration(value: Json) -> Declaration: ...

@dataclass(frozen=True, slots=True)
class GlobalDeclaration:
    """A global augmentation declaration."""

    # whether the declaration is ambient
    is_ambient: bool
    # the statements inside the global body
    statements: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalDeclaration: ...

def encode_global_declaration(
    writer: BinaryWriter, value: GlobalDeclaration
) -> None: ...
def decode_global_declaration(reader: BinaryReader) -> GlobalDeclaration: ...
def to_json_global_declaration(value: GlobalDeclaration) -> Json: ...
def from_json_global_declaration(value: Json) -> GlobalDeclaration: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeDeclaration: ...

def encode_type_declaration(writer: BinaryWriter, value: TypeDeclaration) -> None: ...
def decode_type_declaration(reader: BinaryReader) -> TypeDeclaration: ...
def to_json_type_declaration(value: TypeDeclaration) -> Json: ...
def from_json_type_declaration(value: Json) -> TypeDeclaration: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ClassDeclaration: ...

def encode_class_declaration(writer: BinaryWriter, value: ClassDeclaration) -> None: ...
def decode_class_declaration(reader: BinaryReader) -> ClassDeclaration: ...
def to_json_class_declaration(value: ClassDeclaration) -> Json: ...
def from_json_class_declaration(value: Json) -> ClassDeclaration: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InterfaceDeclaration: ...

def encode_interface_declaration(
    writer: BinaryWriter, value: InterfaceDeclaration
) -> None: ...
def decode_interface_declaration(reader: BinaryReader) -> InterfaceDeclaration: ...
def to_json_interface_declaration(value: InterfaceDeclaration) -> Json: ...
def from_json_interface_declaration(value: Json) -> InterfaceDeclaration: ...

@dataclass(frozen=True, slots=True)
class InterfaceHeritage:
    """One interface heritage clause item."""

    # the extended interface expression
    expression: destack._generated.js.tree.node.LocalNodeId
    # the type arguments applied to the extended interface expression
    type_arguments: Sequence[destack._generated.js.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceHeritage: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InterfaceHeritage: ...

def encode_interface_heritage(
    writer: BinaryWriter, value: InterfaceHeritage
) -> None: ...
def decode_interface_heritage(reader: BinaryReader) -> InterfaceHeritage: ...
def to_json_interface_heritage(value: InterfaceHeritage) -> Json: ...
def from_json_interface_heritage(value: Json) -> InterfaceHeritage: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EnumDeclaration: ...

def encode_enum_declaration(writer: BinaryWriter, value: EnumDeclaration) -> None: ...
def decode_enum_declaration(reader: BinaryReader) -> EnumDeclaration: ...
def to_json_enum_declaration(value: EnumDeclaration) -> Json: ...
def from_json_enum_declaration(value: Json) -> EnumDeclaration: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionDeclaration: ...

def encode_function_declaration(
    writer: BinaryWriter, value: FunctionDeclaration
) -> None: ...
def decode_function_declaration(reader: BinaryReader) -> FunctionDeclaration: ...
def to_json_function_declaration(value: FunctionDeclaration) -> Json: ...
def from_json_function_declaration(value: Json) -> FunctionDeclaration: ...

@dataclass(frozen=True, slots=True)
class EnumField:
    """An EnumField is a named field of an enum declaration."""

    # the name of the enum field
    name: destack._generated.core.string.StringId
    # the value of the enum field
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumField: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EnumField: ...

def encode_enum_field(writer: BinaryWriter, value: EnumField) -> None: ...
def decode_enum_field(reader: BinaryReader) -> EnumField: ...
def to_json_enum_field(value: EnumField) -> Json: ...
def from_json_enum_field(value: Json) -> EnumField: ...

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
