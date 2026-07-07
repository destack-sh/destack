# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.dir.tree.declaration import (
    DeclarationImpl,
)

import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.function
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node

"""Explicit source placement modifier."""
PlaceModifier: typing.TypeAlias = typing.Literal["local"] | typing.Literal["shared"]

def encode_place_modifier(writer: BinaryWriter, value: PlaceModifier) -> None: ...
def decode_place_modifier(reader: BinaryReader) -> PlaceModifier: ...
def to_json_place_modifier(value: PlaceModifier) -> Json: ...
def from_json_place_modifier(value: Json) -> PlaceModifier: ...

@dataclass(frozen=True, slots=True)
class DeclarationGlobal(DeclarationImpl):
    """Global declaration block."""

    global_: GlobalDeclaration
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationModule(DeclarationImpl):
    """Module declaration block."""

    module: ModuleDeclaration
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationType(DeclarationImpl):
    """Type declaration."""

    type: TypeDeclaration
    kind: typing.Literal["type"] = "type"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationStruct(DeclarationImpl):
    """Struct declaration."""

    struct: StructDeclaration
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationClass(DeclarationImpl):
    """Class declaration."""

    class_: ClassDeclaration
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationEnum(DeclarationImpl):
    """Enum declaration."""

    enum: EnumDeclaration
    kind: typing.Literal["enum"] = "enum"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationInterface(DeclarationImpl):
    """Interface declaration."""

    interface: InterfaceDeclaration
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationExtension(DeclarationImpl):
    """Extension declaration."""

    extension: ExtensionDeclaration
    kind: typing.Literal["extension"] = "extension"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DeclarationFunction(DeclarationImpl):
    """Function declaration."""

    function: FunctionDeclaration
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

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

def encode_declaration(writer: BinaryWriter, value: Declaration) -> None: ...
def decode_declaration(reader: BinaryReader) -> Declaration: ...
def to_json_declaration(value: Declaration) -> Json: ...
def from_json_declaration(value: Json) -> Declaration: ...

@dataclass(frozen=True, slots=True)
class GlobalDeclaration:
    """A global declaration block."""

    # the expressions inside the global body
    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # whether the declaration is ambient
    is_ambient: bool

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
class ModuleDeclaration:
    """A module declaration block."""

    # the expressions inside the module body
    expressions: Sequence[destack._generated.dir.tree.node.LocalNodeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleDeclaration: ...

def encode_module_declaration(
    writer: BinaryWriter, value: ModuleDeclaration
) -> None: ...
def decode_module_declaration(reader: BinaryReader) -> ModuleDeclaration: ...
def to_json_module_declaration(value: ModuleDeclaration) -> Json: ...
def from_json_module_declaration(value: Json) -> ModuleDeclaration: ...

@dataclass(frozen=True, slots=True)
class TypeDeclaration:
    """A type declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the explicit placement modifier
    place: PlaceModifier | None
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
class StructDeclaration:
    """A struct declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the explicit placement modifier
    place: PlaceModifier | None
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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StructDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StructDeclaration: ...

def encode_struct_declaration(
    writer: BinaryWriter, value: StructDeclaration
) -> None: ...
def decode_struct_declaration(reader: BinaryReader) -> StructDeclaration: ...
def to_json_struct_declaration(value: StructDeclaration) -> Json: ...
def from_json_struct_declaration(value: Json) -> StructDeclaration: ...

@dataclass(frozen=True, slots=True)
class ClassDeclaration:
    """A class declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the explicit placement modifier
    place: PlaceModifier | None
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
class EnumDeclaration:
    """An enum declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the explicit placement modifier
    place: PlaceModifier | None
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

"""The kind of an enum declaration."""
EnumKind: typing.TypeAlias = typing.Literal["enum"] | typing.Literal["const"]

def encode_enum_kind(writer: BinaryWriter, value: EnumKind) -> None: ...
def decode_enum_kind(reader: BinaryReader) -> EnumKind: ...
def to_json_enum_kind(value: EnumKind) -> Json: ...
def from_json_enum_kind(value: Json) -> EnumKind: ...

@dataclass(frozen=True, slots=True)
class InterfaceDeclaration:
    """An interface declaration."""

    # the declared name
    name: destack._generated.dir.tree.key.Name | None
    # the export kind of the declaration
    export: destack._generated.dir.tree.dependency.ExportKind | None
    # the explicit placement modifier
    place: PlaceModifier | None
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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionDeclaration: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionDeclaration: ...

def encode_extension_declaration(
    writer: BinaryWriter, value: ExtensionDeclaration
) -> None: ...
def decode_extension_declaration(reader: BinaryReader) -> ExtensionDeclaration: ...
def to_json_extension_declaration(value: ExtensionDeclaration) -> Json: ...
def from_json_extension_declaration(value: Json) -> ExtensionDeclaration: ...

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
    """An enum field."""

    # the name of the enum field
    name: destack._generated.dir.tree.key.Name
    # the default value of the enum field
    value: destack._generated.dir.tree.node.LocalNodeId | None

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
    "PlaceModifier",
    "encode_place_modifier",
    "decode_place_modifier",
    "to_json_place_modifier",
    "from_json_place_modifier",
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
