# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property
import destack._generated.dir.tree.static
import destack._generated.dir.type.extension
import destack._generated.dir.type.generic
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class DefinitionSegment:
    """Declaration definitions added by one DIR phase."""

    # the module id of the definition segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # source declaration nodes keyed by declaring symbol
    sources: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        destack._generated.dir.tree.node.GlobalNodeIdAny,
    ]
    # definitions keyed by declaring symbol
    definitions: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId, Definition
    ]
    # extension symbols by target symbol
    extensions_by_target_symbol: Mapping[
        destack._generated.dir.symbol.symbol.GlobalSymbolId,
        Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId],
    ]
    # blanket extension symbols
    blanket_extensions: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DefinitionSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DefinitionSegment: ...

def encode_definition_segment(
    writer: BinaryWriter, value: DefinitionSegment
) -> None: ...
def decode_definition_segment(reader: BinaryReader) -> DefinitionSegment: ...
def to_json_definition_segment(value: DefinitionSegment) -> Json: ...
def from_json_definition_segment(value: Json) -> DefinitionSegment: ...

@dataclass(frozen=True, slots=True)
class DefinitionTypeAlias:
    """Transparent type alias declaration."""

    type_alias: TypeAliasDefinition
    kind: typing.Literal["typeAlias"] = "typeAlias"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionStruct:
    """Struct declaration."""

    struct: StructDefinition
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionClass:
    """Class declaration."""

    class_: ClassDefinition
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionInterface:
    """Nominal interface declaration."""

    interface: InterfaceDefinition
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionEnum:
    """Enum declaration."""

    enum: EnumDefinition
    kind: typing.Literal["enum"] = "enum"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionNewtype:
    """Newtype declaration."""

    newtype: NewtypeDefinition
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionExtension:
    """Extension declaration."""

    extension: destack._generated.dir.type.extension.Extension
    kind: typing.Literal["extension"] = "extension"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Checked declaration data for one symbol."""
Definition: typing.TypeAlias = (
    DefinitionTypeAlias
    | DefinitionStruct
    | DefinitionClass
    | DefinitionInterface
    | DefinitionEnum
    | DefinitionNewtype
    | DefinitionExtension
)

def encode_definition(writer: BinaryWriter, value: Definition) -> None: ...
def decode_definition(reader: BinaryReader) -> Definition: ...
def to_json_definition(value: Definition) -> Json: ...
def from_json_definition(value: Json) -> Definition: ...

@dataclass(frozen=True, slots=True)
class TypeAliasDefinition:
    """Checked declaration data for one transparent type alias."""

    # the generic template declared by the alias
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the checked alias value
    value: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeAliasDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TypeAliasDefinition: ...

def encode_type_alias_definition(
    writer: BinaryWriter, value: TypeAliasDefinition
) -> None: ...
def decode_type_alias_definition(reader: BinaryReader) -> TypeAliasDefinition: ...
def to_json_type_alias_definition(value: TypeAliasDefinition) -> Json: ...
def from_json_type_alias_definition(value: Json) -> TypeAliasDefinition: ...

@dataclass(frozen=True, slots=True)
class StructDefinition:
    """Checked declaration data for one nominal struct."""

    # the generic template declared by the struct
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> StructDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> StructDefinition: ...

def encode_struct_definition(writer: BinaryWriter, value: StructDefinition) -> None: ...
def decode_struct_definition(reader: BinaryReader) -> StructDefinition: ...
def to_json_struct_definition(value: StructDefinition) -> Json: ...
def from_json_struct_definition(value: Json) -> StructDefinition: ...

@dataclass(frozen=True, slots=True)
class NominalHeritage:
    """One nominal heritage."""

    # the source heritage node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the heritage nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments used at the relation site, empty when not applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NominalHeritage: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NominalHeritage: ...

def encode_nominal_heritage(writer: BinaryWriter, value: NominalHeritage) -> None: ...
def decode_nominal_heritage(reader: BinaryReader) -> NominalHeritage: ...
def to_json_nominal_heritage(value: NominalHeritage) -> Json: ...
def from_json_nominal_heritage(value: Json) -> NominalHeritage: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberField:
    """Field member with a checked type."""

    field: FieldDefinition
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberMethod:
    """Method member with a checked type."""

    method: MethodDefinition
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberAssociatedType:
    """Associated type member."""

    associated_type: AssociatedTypeDefinition
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberAssociatedConst:
    """Associated constant member."""

    associated_const: AssociatedConstDefinition
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberVariant:
    """Enum variant member."""

    variant: VariantDefinition
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberCallSignature:
    """Structural call signature member."""

    call_signature: SignatureDefinition
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberConstructSignature:
    """Structural construct signature member."""

    construct_signature: SignatureDefinition
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DefinitionMemberIndexSignature:
    """Structural index signature member."""

    index_signature: SignatureDefinition
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One checked declaration member."""
DefinitionMember: typing.TypeAlias = (
    DefinitionMemberField
    | DefinitionMemberMethod
    | DefinitionMemberAssociatedType
    | DefinitionMemberAssociatedConst
    | DefinitionMemberVariant
    | DefinitionMemberCallSignature
    | DefinitionMemberConstructSignature
    | DefinitionMemberIndexSignature
)

def encode_definition_member(writer: BinaryWriter, value: DefinitionMember) -> None: ...
def decode_definition_member(reader: BinaryReader) -> DefinitionMember: ...
def to_json_definition_member(value: DefinitionMember) -> Json: ...
def from_json_definition_member(value: Json) -> DefinitionMember: ...

@dataclass(frozen=True, slots=True)
class FieldDefinition:
    """One checked field member."""

    # the member space declaring the field
    space: MemberSpace
    # the field symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the field key
    key: destack._generated.dir.symbol.key.StaticKey
    # the checked field type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # whether subclasses must provide the field
    is_abstract: bool
    # whether the field overrides an inherited member
    is_override: bool
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FieldDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FieldDefinition: ...

def encode_field_definition(writer: BinaryWriter, value: FieldDefinition) -> None: ...
def decode_field_definition(reader: BinaryReader) -> FieldDefinition: ...
def to_json_field_definition(value: FieldDefinition) -> Json: ...
def from_json_field_definition(value: Json) -> FieldDefinition: ...

"""Member namespace selected by member lookup."""
MemberSpace: typing.TypeAlias = typing.Literal["instance"] | typing.Literal["static"]

def encode_member_space(writer: BinaryWriter, value: MemberSpace) -> None: ...
def decode_member_space(reader: BinaryReader) -> MemberSpace: ...
def to_json_member_space(value: MemberSpace) -> Json: ...
def from_json_member_space(value: Json) -> MemberSpace: ...

@dataclass(frozen=True, slots=True)
class MethodDefinition:
    """One method member."""

    # the member space declaring the method
    space: MemberSpace
    # the method symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the nominal member slot
    slot: destack._generated.dir.tree.property.MemberSlot
    # the method role
    role: destack._generated.dir.tree.property.FunctionRole | None
    # the checked method type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the abstraction mode governing overrides
    abstraction: destack._generated.dir.tree.property.MethodAbstraction
    # whether the method overrides an inherited member
    is_override: bool
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MethodDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MethodDefinition: ...

def encode_method_definition(writer: BinaryWriter, value: MethodDefinition) -> None: ...
def decode_method_definition(reader: BinaryReader) -> MethodDefinition: ...
def to_json_method_definition(value: MethodDefinition) -> Json: ...
def from_json_method_definition(value: Json) -> MethodDefinition: ...

@dataclass(frozen=True, slots=True)
class AssociatedTypeDefinition:
    """One checked associated type."""

    # the associated type symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the associated type key
    key: destack._generated.dir.symbol.key.StaticKey
    # the upper bound required by this associated type
    constraint: destack._generated.dir.type.type.GlobalTypeId | None
    # the concrete associated type value
    value: destack._generated.dir.type.type.GlobalTypeId | None
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssociatedTypeDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssociatedTypeDefinition: ...

def encode_associated_type_definition(
    writer: BinaryWriter, value: AssociatedTypeDefinition
) -> None: ...
def decode_associated_type_definition(
    reader: BinaryReader,
) -> AssociatedTypeDefinition: ...
def to_json_associated_type_definition(value: AssociatedTypeDefinition) -> Json: ...
def from_json_associated_type_definition(value: Json) -> AssociatedTypeDefinition: ...

@dataclass(frozen=True, slots=True)
class AssociatedConstDefinition:
    """One checked associated constant."""

    # the associated const symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the associated const key
    key: destack._generated.dir.symbol.key.StaticKey
    # the checked static type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the checked static value
    value: destack._generated.dir.tree.static.GlobalStaticId | None
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AssociatedConstDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AssociatedConstDefinition: ...

def encode_associated_const_definition(
    writer: BinaryWriter, value: AssociatedConstDefinition
) -> None: ...
def decode_associated_const_definition(
    reader: BinaryReader,
) -> AssociatedConstDefinition: ...
def to_json_associated_const_definition(value: AssociatedConstDefinition) -> Json: ...
def from_json_associated_const_definition(value: Json) -> AssociatedConstDefinition: ...

@dataclass(frozen=True, slots=True)
class VariantDefinition:
    """One checked enum variant."""

    # the variant symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source enum field node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the variant key
    key: destack._generated.dir.symbol.key.StaticKey
    # the checked variant value
    value: destack._generated.dir.tree.static.GlobalStaticId | None
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> VariantDefinition: ...

def encode_variant_definition(
    writer: BinaryWriter, value: VariantDefinition
) -> None: ...
def decode_variant_definition(reader: BinaryReader) -> VariantDefinition: ...
def to_json_variant_definition(value: VariantDefinition) -> Json: ...
def from_json_variant_definition(value: Json) -> VariantDefinition: ...

@dataclass(frozen=True, slots=True)
class SignatureDefinition:
    """One checked symbol-free signature member."""

    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the checked signature type
    ty: destack._generated.dir.type.type.GlobalTypeId
    # the @if availability condition guarding this member, when guarded
    condition: destack._generated.dir.type.type.GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureDefinition: ...

def encode_signature_definition(
    writer: BinaryWriter, value: SignatureDefinition
) -> None: ...
def decode_signature_definition(reader: BinaryReader) -> SignatureDefinition: ...
def to_json_signature_definition(value: SignatureDefinition) -> Json: ...
def from_json_signature_definition(value: Json) -> SignatureDefinition: ...

@dataclass(frozen=True, slots=True)
class ClassDefinition:
    """Checked declaration data for one nominal class."""

    # the generic template declared by the class
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # whether the class is abstract
    is_abstract: bool
    # whether the class rejects subclasses
    is_final: bool
    # the extended class
    extends: NominalHeritage | None
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ClassDefinition: ...

def encode_class_definition(writer: BinaryWriter, value: ClassDefinition) -> None: ...
def decode_class_definition(reader: BinaryReader) -> ClassDefinition: ...
def to_json_class_definition(value: ClassDefinition) -> Json: ...
def from_json_class_definition(value: Json) -> ClassDefinition: ...

@dataclass(frozen=True, slots=True)
class InterfaceDefinition:
    """Checked declaration data for one nominal interface."""

    # the generic template declared by the interface
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # whether the interface has nominal identity
    is_nominal: bool
    # the inherited interfaces
    extends: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InterfaceDefinition: ...

def encode_interface_definition(
    writer: BinaryWriter, value: InterfaceDefinition
) -> None: ...
def decode_interface_definition(reader: BinaryReader) -> InterfaceDefinition: ...
def to_json_interface_definition(value: InterfaceDefinition) -> Json: ...
def from_json_interface_definition(value: Json) -> InterfaceDefinition: ...

@dataclass(frozen=True, slots=True)
class EnumDefinition:
    """Checked declaration data for one nominal enum."""

    # the generic template declared by the enum
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EnumDefinition: ...

def encode_enum_definition(writer: BinaryWriter, value: EnumDefinition) -> None: ...
def decode_enum_definition(reader: BinaryReader) -> EnumDefinition: ...
def to_json_enum_definition(value: EnumDefinition) -> Json: ...
def from_json_enum_definition(value: Json) -> EnumDefinition: ...

@dataclass(frozen=True, slots=True)
class NewtypeDefinition:
    """Checked declaration data for one nominal type alias."""

    # the generic template declared by the newtype
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the nominal backing type
    value: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeDefinition: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NewtypeDefinition: ...

def encode_newtype_definition(
    writer: BinaryWriter, value: NewtypeDefinition
) -> None: ...
def decode_newtype_definition(reader: BinaryReader) -> NewtypeDefinition: ...
def to_json_newtype_definition(value: NewtypeDefinition) -> Json: ...
def from_json_newtype_definition(value: Json) -> NewtypeDefinition: ...

__all__ = [
    "DefinitionSegment",
    "encode_definition_segment",
    "decode_definition_segment",
    "to_json_definition_segment",
    "from_json_definition_segment",
    "Definition",
    "encode_definition",
    "decode_definition",
    "to_json_definition",
    "from_json_definition",
    "DefinitionTypeAlias",
    "DefinitionStruct",
    "DefinitionClass",
    "DefinitionInterface",
    "DefinitionEnum",
    "DefinitionNewtype",
    "DefinitionExtension",
    "TypeAliasDefinition",
    "encode_type_alias_definition",
    "decode_type_alias_definition",
    "to_json_type_alias_definition",
    "from_json_type_alias_definition",
    "StructDefinition",
    "encode_struct_definition",
    "decode_struct_definition",
    "to_json_struct_definition",
    "from_json_struct_definition",
    "NominalHeritage",
    "encode_nominal_heritage",
    "decode_nominal_heritage",
    "to_json_nominal_heritage",
    "from_json_nominal_heritage",
    "DefinitionMember",
    "encode_definition_member",
    "decode_definition_member",
    "to_json_definition_member",
    "from_json_definition_member",
    "DefinitionMemberField",
    "DefinitionMemberMethod",
    "DefinitionMemberAssociatedType",
    "DefinitionMemberAssociatedConst",
    "DefinitionMemberVariant",
    "DefinitionMemberCallSignature",
    "DefinitionMemberConstructSignature",
    "DefinitionMemberIndexSignature",
    "FieldDefinition",
    "encode_field_definition",
    "decode_field_definition",
    "to_json_field_definition",
    "from_json_field_definition",
    "MemberSpace",
    "encode_member_space",
    "decode_member_space",
    "to_json_member_space",
    "from_json_member_space",
    "MethodDefinition",
    "encode_method_definition",
    "decode_method_definition",
    "to_json_method_definition",
    "from_json_method_definition",
    "AssociatedTypeDefinition",
    "encode_associated_type_definition",
    "decode_associated_type_definition",
    "to_json_associated_type_definition",
    "from_json_associated_type_definition",
    "AssociatedConstDefinition",
    "encode_associated_const_definition",
    "decode_associated_const_definition",
    "to_json_associated_const_definition",
    "from_json_associated_const_definition",
    "VariantDefinition",
    "encode_variant_definition",
    "decode_variant_definition",
    "to_json_variant_definition",
    "from_json_variant_definition",
    "SignatureDefinition",
    "encode_signature_definition",
    "decode_signature_definition",
    "to_json_signature_definition",
    "from_json_signature_definition",
    "ClassDefinition",
    "encode_class_definition",
    "decode_class_definition",
    "to_json_class_definition",
    "from_json_class_definition",
    "InterfaceDefinition",
    "encode_interface_definition",
    "decode_interface_definition",
    "to_json_interface_definition",
    "from_json_interface_definition",
    "EnumDefinition",
    "encode_enum_definition",
    "decode_enum_definition",
    "to_json_enum_definition",
    "from_json_enum_definition",
    "NewtypeDefinition",
    "encode_newtype_definition",
    "decode_newtype_definition",
    "to_json_newtype_definition",
    "from_json_newtype_definition",
]
