# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    nested_bytes,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property
import destack._generated.dir.tree.static
import destack._generated.dir.type.generic
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ClassConstructorDeclared:
    """Constructor explicitly declared by this class."""

    # the declared constructor symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["declared"] = "declared"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_constructor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_constructor(self)


@dataclass(frozen=True, slots=True)
class ClassConstructorDefault:
    """Default `new T()` candidate for a class with no declared constructor."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_constructor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_constructor(self)


@dataclass(frozen=True, slots=True)
class ClassConstructorForwardedDeclared:
    """Constructor forwarded to an explicit base class constructor."""

    # the base class symbol
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the selected base constructor symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["forwardedDeclared"] = "forwardedDeclared"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_constructor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_constructor(self)


@dataclass(frozen=True, slots=True)
class ClassConstructorForwardedDefault:
    """Constructor forwarded to a base class default constructor."""

    # the base class symbol
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["forwardedDefault"] = "forwardedDefault"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_constructor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_constructor(self)


"""Class construct candidate origin."""
ClassConstructor: typing.TypeAlias = (
    ClassConstructorDeclared
    | ClassConstructorDefault
    | ClassConstructorForwardedDeclared
    | ClassConstructorForwardedDefault
)


def encode_class_constructor(writer: BinaryWriter, value: ClassConstructor) -> None:
    """Encode one ClassConstructor."""
    if value.kind == "declared":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "default":
        writer.write_unsigned(1)
    elif value.kind == "forwardedDeclared":
        writer.write_unsigned(2)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.base)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "forwardedDefault":
        writer.write_unsigned(3)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.base)
    else:
        raise SerdeError("unknown enum variant")


def decode_class_constructor(reader: BinaryReader) -> ClassConstructor:
    """Decode one ClassConstructor."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ClassConstructorDeclared(
            symbol=symbol,
        )
    elif variant == 1:
        return ClassConstructorDefault()
    elif variant == 2:
        base = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ClassConstructorForwardedDeclared(
            base=base,
            symbol=symbol,
        )
    elif variant == 3:
        base = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ClassConstructorForwardedDefault(
            base=base,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_class_constructor(value: ClassConstructor) -> Json:
    """Return one JSON value for one ClassConstructor."""
    if value.kind == "declared":
        return {
            "kind": "declared",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "default":
        return {
            "kind": "default",
        }
    elif value.kind == "forwardedDeclared":
        return {
            "kind": "forwardedDeclared",
            "base": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.base
            ),
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "forwardedDefault":
        return {
            "kind": "forwardedDefault",
            "base": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.base
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_class_constructor(value: Json) -> ClassConstructor:
    """Return one ClassConstructor from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "declared":
        return ClassConstructorDeclared(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
        )
    elif kind == "default":
        return ClassConstructorDefault()
    elif kind == "forwardedDeclared":
        return ClassConstructorForwardedDeclared(
            base=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "base")
            ),
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            ),
        )
    elif kind == "forwardedDefault":
        return ClassConstructorForwardedDefault(
            base=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "base")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DefinitionSegment:
        """Decode one DefinitionSegment."""
        return decode_definition_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> DefinitionSegment:
        """Return one DefinitionSegment from one JSON value."""
        return from_json_definition_segment(value)


def encode_definition_segment(writer: BinaryWriter, value: DefinitionSegment) -> None:
    """Encode one DefinitionSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_sources_0 = []
    for key_value_sources_0, item_value_sources_0 in value.sources.items():

        def write_key_value_sources_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_sources_0
            )

        key_bytes = nested_bytes(write_key_value_sources_0)
        entries_value_sources_0.append(
            (key_value_sources_0, item_value_sources_0, key_bytes)
        )
    entries_value_sources_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_sources_0))
    for entry_value_sources_0 in entries_value_sources_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_sources_0[0]
        )
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_sources_0[1]
        )
    entries_value_definitions_0 = []
    for key_value_definitions_0, item_value_definitions_0 in value.definitions.items():

        def write_key_value_definitions_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_definitions_0
            )

        key_bytes = nested_bytes(write_key_value_definitions_0)
        entries_value_definitions_0.append(
            (key_value_definitions_0, item_value_definitions_0, key_bytes)
        )
    entries_value_definitions_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_definitions_0))
    for entry_value_definitions_0 in entries_value_definitions_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_definitions_0[0]
        )
        encode_definition(writer, entry_value_definitions_0[1])
    entries_value_extensions_by_target_symbol_0 = []
    for (
        key_value_extensions_by_target_symbol_0,
        item_value_extensions_by_target_symbol_0,
    ) in value.extensions_by_target_symbol.items():

        def write_key_value_extensions_by_target_symbol_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, key_value_extensions_by_target_symbol_0
            )

        key_bytes = nested_bytes(write_key_value_extensions_by_target_symbol_0)
        entries_value_extensions_by_target_symbol_0.append(
            (
                key_value_extensions_by_target_symbol_0,
                item_value_extensions_by_target_symbol_0,
                key_bytes,
            )
        )
    entries_value_extensions_by_target_symbol_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_extensions_by_target_symbol_0))
    for (
        entry_value_extensions_by_target_symbol_0
    ) in entries_value_extensions_by_target_symbol_0:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, entry_value_extensions_by_target_symbol_0[0]
        )
        writer.write_unsigned(len(entry_value_extensions_by_target_symbol_0[1]))
        for (
            item_entry_value_extensions_by_target_symbol_0_1_1
        ) in entry_value_extensions_by_target_symbol_0[1]:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, item_entry_value_extensions_by_target_symbol_0_1_1
            )
    writer.write_unsigned(len(value.blanket_extensions))
    for item_value_blanket_extensions_0 in value.blanket_extensions:
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, item_value_blanket_extensions_0
        )


def decode_definition_segment(reader: BinaryReader) -> DefinitionSegment:
    """Decode one DefinitionSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    sources = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): destack._generated.dir.tree.node.decode_global_node_id_any(reader)
        for _ in range(reader.read_number())
    }
    definitions = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(
            reader
        ): decode_definition(reader)
        for _ in range(reader.read_number())
    }
    extensions_by_target_symbol = {
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader): [
            destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    blanket_extensions = [
        destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        for _ in range(reader.read_number())
    ]

    return DefinitionSegment(
        module_id=module_id,
        sources=sources,
        definitions=definitions,
        extensions_by_target_symbol=extensions_by_target_symbol,
        blanket_extensions=blanket_extensions,
    )


def to_json_definition_segment(value: DefinitionSegment) -> Json:
    """Return one JSON value for one DefinitionSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "sources": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                destack._generated.dir.tree.node.to_json_global_node_id_any(item_0),
            ]
            for key_0, item_0 in value.sources.items()
        ],
        "definitions": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                to_json_definition(item_0),
            ]
            for key_0, item_0 in value.definitions.items()
        ],
        "extensionsByTargetSymbol": [
            [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(key_0),
                [
                    destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                        item_1
                    )
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.extensions_by_target_symbol.items()
        ],
        "blanketExtensions": [
            destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0)
            for item_0 in value.blanket_extensions
        ],
    }


def from_json_definition_segment(value: Json) -> DefinitionSegment:
    """Return one DefinitionSegment from one JSON value."""
    object_ = json_object(value)

    return DefinitionSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        sources={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): destack._generated.dir.tree.node.from_json_global_node_id_any(item_0)
            for key_0, item_0 in json_array(json_field(object_, "sources"))
        },
        definitions={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                key_0
            ): from_json_definition(item_0)
            for key_0, item_0 in json_array(json_field(object_, "definitions"))
        },
        extensions_by_target_symbol={
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(key_0): [
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "extensionsByTargetSymbol")
            )
        },
        blanket_extensions=[
            destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
            for item_0 in json_array(json_field(object_, "blanketExtensions"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DefinitionTypeAlias:
    """Transparent type alias declaration."""

    type_alias: TypeAliasDefinition
    kind: typing.Literal["typeAlias"] = "typeAlias"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionStruct:
    """Struct declaration."""

    struct: StructDefinition
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionClass:
    """Class declaration."""

    class_: ClassDefinition
    kind: typing.Literal["class"] = "class"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionInterface:
    """Nominal interface declaration."""

    interface: InterfaceDefinition
    kind: typing.Literal["interface"] = "interface"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionEnum:
    """Enum declaration."""

    enum: EnumDefinition
    kind: typing.Literal["enum"] = "enum"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionNewtype:
    """Newtype declaration."""

    newtype: NewtypeDefinition
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


@dataclass(frozen=True, slots=True)
class DefinitionExtension:
    """Extension declaration."""

    extension: ExtensionDefinition
    kind: typing.Literal["extension"] = "extension"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition(self)


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


def encode_definition(writer: BinaryWriter, value: Definition) -> None:
    """Encode one Definition."""
    if value.kind == "typeAlias":
        writer.write_unsigned(0)
        encode_type_alias_definition(writer, value.type_alias)
    elif value.kind == "struct":
        writer.write_unsigned(1)
        encode_struct_definition(writer, value.struct)
    elif value.kind == "class":
        writer.write_unsigned(2)
        encode_class_definition(writer, value.class_)
    elif value.kind == "interface":
        writer.write_unsigned(3)
        encode_interface_definition(writer, value.interface)
    elif value.kind == "enum":
        writer.write_unsigned(4)
        encode_enum_definition(writer, value.enum)
    elif value.kind == "newtype":
        writer.write_unsigned(5)
        encode_newtype_definition(writer, value.newtype)
    elif value.kind == "extension":
        writer.write_unsigned(6)
        encode_extension_definition(writer, value.extension)
    else:
        raise SerdeError("unknown enum variant")


def decode_definition(reader: BinaryReader) -> Definition:
    """Decode one Definition."""
    variant = reader.read_number()

    if variant == 0:
        type_alias = decode_type_alias_definition(reader)

        return DefinitionTypeAlias(type_alias=type_alias)
    elif variant == 1:
        struct = decode_struct_definition(reader)

        return DefinitionStruct(struct=struct)
    elif variant == 2:
        class_ = decode_class_definition(reader)

        return DefinitionClass(class_=class_)
    elif variant == 3:
        interface = decode_interface_definition(reader)

        return DefinitionInterface(interface=interface)
    elif variant == 4:
        enum = decode_enum_definition(reader)

        return DefinitionEnum(enum=enum)
    elif variant == 5:
        newtype = decode_newtype_definition(reader)

        return DefinitionNewtype(newtype=newtype)
    elif variant == 6:
        extension = decode_extension_definition(reader)

        return DefinitionExtension(extension=extension)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_definition(value: Definition) -> Json:
    """Return one JSON value for one Definition."""
    if value.kind == "typeAlias":
        return {
            "kind": "typeAlias",
            "type_alias": to_json_type_alias_definition(value.type_alias),
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "struct": to_json_struct_definition(value.struct),
        }
    elif value.kind == "class":
        return {
            "kind": "class",
            "class": to_json_class_definition(value.class_),
        }
    elif value.kind == "interface":
        return {
            "kind": "interface",
            "interface": to_json_interface_definition(value.interface),
        }
    elif value.kind == "enum":
        return {
            "kind": "enum",
            "enum": to_json_enum_definition(value.enum),
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "newtype": to_json_newtype_definition(value.newtype),
        }
    elif value.kind == "extension":
        return {
            "kind": "extension",
            "extension": to_json_extension_definition(value.extension),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_definition(value: Json) -> Definition:
    """Return one Definition from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "typeAlias":
        return DefinitionTypeAlias(
            type_alias=from_json_type_alias_definition(
                json_field(object_, "type_alias")
            )
        )
    elif kind == "struct":
        return DefinitionStruct(
            struct=from_json_struct_definition(json_field(object_, "struct"))
        )
    elif kind == "class":
        return DefinitionClass(
            class_=from_json_class_definition(json_field(object_, "class"))
        )
    elif kind == "interface":
        return DefinitionInterface(
            interface=from_json_interface_definition(json_field(object_, "interface"))
        )
    elif kind == "enum":
        return DefinitionEnum(
            enum=from_json_enum_definition(json_field(object_, "enum"))
        )
    elif kind == "newtype":
        return DefinitionNewtype(
            newtype=from_json_newtype_definition(json_field(object_, "newtype"))
        )
    elif kind == "extension":
        return DefinitionExtension(
            extension=from_json_extension_definition(json_field(object_, "extension"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TypeAliasDefinition:
    """Checked declaration data for one transparent type alias."""

    # the generic template declared by the alias
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the checked alias value
    value: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_alias_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeAliasDefinition:
        """Decode one TypeAliasDefinition."""
        return decode_type_alias_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_alias_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeAliasDefinition:
        """Return one TypeAliasDefinition from one JSON value."""
        return from_json_type_alias_definition(value)


def encode_type_alias_definition(
    writer: BinaryWriter, value: TypeAliasDefinition
) -> None:
    """Encode one TypeAliasDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.value)


def decode_type_alias_definition(reader: BinaryReader) -> TypeAliasDefinition:
    """Decode one TypeAliasDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    value_ = destack._generated.dir.type.type.decode_global_type_id(reader)

    return TypeAliasDefinition(
        template=template,
        value=value_,
    )


def to_json_type_alias_definition(value: TypeAliasDefinition) -> Json:
    """Return one JSON value for one TypeAliasDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "value": destack._generated.dir.type.type.to_json_global_type_id(value.value),
    }


def from_json_type_alias_definition(value: Json) -> TypeAliasDefinition:
    """Return one TypeAliasDefinition from one JSON value."""
    object_ = json_object(value)

    return TypeAliasDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        value=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "value")
        ),
    )


@dataclass(frozen=True, slots=True)
class StructDefinition:
    """Checked declaration data for one nominal struct."""

    # the generic template declared by the struct
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_struct_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StructDefinition:
        """Decode one StructDefinition."""
        return decode_struct_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_struct_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> StructDefinition:
        """Return one StructDefinition from one JSON value."""
        return from_json_struct_definition(value)


def encode_struct_definition(writer: BinaryWriter, value: StructDefinition) -> None:
    """Encode one StructDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    writer.write_unsigned(len(value.implements))
    for item_value_implements_0 in value.implements:
        encode_nominal_heritage(writer, item_value_implements_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_struct_definition(reader: BinaryReader) -> StructDefinition:
    """Decode one StructDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    implements = [decode_nominal_heritage(reader) for _ in range(reader.read_number())]
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return StructDefinition(
        template=template,
        implements=implements,
        members=members,
    )


def to_json_struct_definition(value: StructDefinition) -> Json:
    """Return one JSON value for one StructDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "implements": [to_json_nominal_heritage(item_0) for item_0 in value.implements],
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_struct_definition(value: Json) -> StructDefinition:
    """Return one StructDefinition from one JSON value."""
    object_ = json_object(value)

    return StructDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        implements=[
            from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "implements"))
        ],
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NominalHeritage:
    """One nominal heritage."""

    # the source heritage node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the heritage nominal symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generic arguments used at the relation site, empty when not applied
    arguments: Sequence[destack._generated.dir.type.type.GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_nominal_heritage(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NominalHeritage:
        """Decode one NominalHeritage."""
        return decode_nominal_heritage(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_nominal_heritage(self)

    @classmethod
    def from_json(cls, value: Json) -> NominalHeritage:
        """Return one NominalHeritage from one JSON value."""
        return from_json_nominal_heritage(value)


def encode_nominal_heritage(writer: BinaryWriter, value: NominalHeritage) -> None:
    """Encode one NominalHeritage."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.type.type.encode_global_type_id(
            writer, item_value_arguments_0
        )


def decode_nominal_heritage(reader: BinaryReader) -> NominalHeritage:
    """Decode one NominalHeritage."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [
        destack._generated.dir.type.type.decode_global_type_id(reader)
        for _ in range(reader.read_number())
    ]

    return NominalHeritage(
        source=source,
        symbol=symbol,
        arguments=arguments,
    )


def to_json_nominal_heritage(value: NominalHeritage) -> Json:
    """Return one JSON value for one NominalHeritage."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [
            destack._generated.dir.type.type.to_json_global_type_id(item_0)
            for item_0 in value.arguments
        ],
    }


def from_json_nominal_heritage(value: Json) -> NominalHeritage:
    """Return one NominalHeritage from one JSON value."""
    object_ = json_object(value)

    return NominalHeritage(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            destack._generated.dir.type.type.from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DefinitionMemberField:
    """Field member."""

    field: FieldDefinition
    kind: typing.Literal["field"] = "field"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberMethod:
    """Method member."""

    method: MethodDefinition
    kind: typing.Literal["method"] = "method"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberAssociatedType:
    """Associated type member."""

    associated_type: AssociatedTypeDefinition
    kind: typing.Literal["associatedType"] = "associatedType"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberAssociatedConst:
    """Associated constant member."""

    associated_const: AssociatedConstDefinition
    kind: typing.Literal["associatedConst"] = "associatedConst"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberVariant:
    """Enum variant member."""

    variant: VariantDefinition
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberCallSignature:
    """Structural call signature member."""

    call_signature: SignatureDefinition
    kind: typing.Literal["callSignature"] = "callSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberConstructSignature:
    """Structural construct signature member."""

    construct_signature: SignatureDefinition
    kind: typing.Literal["constructSignature"] = "constructSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


@dataclass(frozen=True, slots=True)
class DefinitionMemberIndexSignature:
    """Structural index signature member."""

    index_signature: SignatureDefinition
    kind: typing.Literal["indexSignature"] = "indexSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_definition_member(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_definition_member(self)


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


def encode_definition_member(writer: BinaryWriter, value: DefinitionMember) -> None:
    """Encode one DefinitionMember."""
    if value.kind == "field":
        writer.write_unsigned(0)
        encode_field_definition(writer, value.field)
    elif value.kind == "method":
        writer.write_unsigned(1)
        encode_method_definition(writer, value.method)
    elif value.kind == "associatedType":
        writer.write_unsigned(2)
        encode_associated_type_definition(writer, value.associated_type)
    elif value.kind == "associatedConst":
        writer.write_unsigned(3)
        encode_associated_const_definition(writer, value.associated_const)
    elif value.kind == "variant":
        writer.write_unsigned(4)
        encode_variant_definition(writer, value.variant)
    elif value.kind == "callSignature":
        writer.write_unsigned(5)
        encode_signature_definition(writer, value.call_signature)
    elif value.kind == "constructSignature":
        writer.write_unsigned(6)
        encode_signature_definition(writer, value.construct_signature)
    elif value.kind == "indexSignature":
        writer.write_unsigned(7)
        encode_signature_definition(writer, value.index_signature)
    else:
        raise SerdeError("unknown enum variant")


def decode_definition_member(reader: BinaryReader) -> DefinitionMember:
    """Decode one DefinitionMember."""
    variant = reader.read_number()

    if variant == 0:
        field = decode_field_definition(reader)

        return DefinitionMemberField(field=field)
    elif variant == 1:
        method = decode_method_definition(reader)

        return DefinitionMemberMethod(method=method)
    elif variant == 2:
        associated_type = decode_associated_type_definition(reader)

        return DefinitionMemberAssociatedType(associated_type=associated_type)
    elif variant == 3:
        associated_const = decode_associated_const_definition(reader)

        return DefinitionMemberAssociatedConst(associated_const=associated_const)
    elif variant == 4:
        variant = decode_variant_definition(reader)

        return DefinitionMemberVariant(variant=variant)
    elif variant == 5:
        call_signature = decode_signature_definition(reader)

        return DefinitionMemberCallSignature(call_signature=call_signature)
    elif variant == 6:
        construct_signature = decode_signature_definition(reader)

        return DefinitionMemberConstructSignature(
            construct_signature=construct_signature
        )
    elif variant == 7:
        index_signature = decode_signature_definition(reader)

        return DefinitionMemberIndexSignature(index_signature=index_signature)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_definition_member(value: DefinitionMember) -> Json:
    """Return one JSON value for one DefinitionMember."""
    if value.kind == "field":
        return {
            "kind": "field",
            "field": to_json_field_definition(value.field),
        }
    elif value.kind == "method":
        return {
            "kind": "method",
            "method": to_json_method_definition(value.method),
        }
    elif value.kind == "associatedType":
        return {
            "kind": "associatedType",
            "associated_type": to_json_associated_type_definition(
                value.associated_type
            ),
        }
    elif value.kind == "associatedConst":
        return {
            "kind": "associatedConst",
            "associated_const": to_json_associated_const_definition(
                value.associated_const
            ),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "variant": to_json_variant_definition(value.variant),
        }
    elif value.kind == "callSignature":
        return {
            "kind": "callSignature",
            "call_signature": to_json_signature_definition(value.call_signature),
        }
    elif value.kind == "constructSignature":
        return {
            "kind": "constructSignature",
            "construct_signature": to_json_signature_definition(
                value.construct_signature
            ),
        }
    elif value.kind == "indexSignature":
        return {
            "kind": "indexSignature",
            "index_signature": to_json_signature_definition(value.index_signature),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_definition_member(value: Json) -> DefinitionMember:
    """Return one DefinitionMember from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "field":
        return DefinitionMemberField(
            field=from_json_field_definition(json_field(object_, "field"))
        )
    elif kind == "method":
        return DefinitionMemberMethod(
            method=from_json_method_definition(json_field(object_, "method"))
        )
    elif kind == "associatedType":
        return DefinitionMemberAssociatedType(
            associated_type=from_json_associated_type_definition(
                json_field(object_, "associated_type")
            )
        )
    elif kind == "associatedConst":
        return DefinitionMemberAssociatedConst(
            associated_const=from_json_associated_const_definition(
                json_field(object_, "associated_const")
            )
        )
    elif kind == "variant":
        return DefinitionMemberVariant(
            variant=from_json_variant_definition(json_field(object_, "variant"))
        )
    elif kind == "callSignature":
        return DefinitionMemberCallSignature(
            call_signature=from_json_signature_definition(
                json_field(object_, "call_signature")
            )
        )
    elif kind == "constructSignature":
        return DefinitionMemberConstructSignature(
            construct_signature=from_json_signature_definition(
                json_field(object_, "construct_signature")
            )
        )
    elif kind == "indexSignature":
        return DefinitionMemberIndexSignature(
            index_signature=from_json_signature_definition(
                json_field(object_, "index_signature")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
    # the field initializer expression, when one is declared
    initializer: destack._generated.dir.tree.node.GlobalNodeIdAny | None
    # whether the field asserts definite assignment outside constructors
    is_definite: bool
    # whether subclasses must provide the field
    is_abstract: bool
    # whether the field overrides an inherited member
    is_override: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_field_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FieldDefinition:
        """Decode one FieldDefinition."""
        return decode_field_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_field_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> FieldDefinition:
        """Return one FieldDefinition from one JSON value."""
        return from_json_field_definition(value)


def encode_field_definition(writer: BinaryWriter, value: FieldDefinition) -> None:
    """Encode one FieldDefinition."""
    encode_member_space(writer, value.space)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    if value.initializer is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.initializer
        )
    writer.write_bool(value.is_definite)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_override)


def decode_field_definition(reader: BinaryReader) -> FieldDefinition:
    """Decode one FieldDefinition."""
    space = decode_member_space(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    initializer = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )
    is_definite = reader.read_bool()
    is_abstract = reader.read_bool()
    is_override = reader.read_bool()

    return FieldDefinition(
        space=space,
        symbol=symbol,
        source=source,
        key=key,
        initializer=initializer,
        is_definite=is_definite,
        is_abstract=is_abstract,
        is_override=is_override,
    )


def to_json_field_definition(value: FieldDefinition) -> Json:
    """Return one JSON value for one FieldDefinition."""
    return {
        "space": to_json_member_space(value.space),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        **(
            {}
            if value.initializer is None
            else {
                "initializer": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.initializer
                )
            }
        ),
        "isDefinite": value.is_definite,
        "isAbstract": value.is_abstract,
        "isOverride": value.is_override,
    }


def from_json_field_definition(value: Json) -> FieldDefinition:
    """Return one FieldDefinition from one JSON value."""
    object_ = json_object(value)

    return FieldDefinition(
        space=from_json_member_space(json_field(object_, "space")),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        initializer=json_optional(
            object_,
            "initializer",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
        is_definite=json_bool(json_field(object_, "isDefinite")),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_override=json_bool(json_field(object_, "isOverride")),
    )


"""Member namespace selected by member lookup."""
MemberSpace: typing.TypeAlias = typing.Literal["instance"] | typing.Literal["static"]


def encode_member_space(writer: BinaryWriter, value: MemberSpace) -> None:
    """Encode one MemberSpace."""
    if value == "instance":
        writer.write_unsigned(0)
    elif value == "static":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_space(reader: BinaryReader) -> MemberSpace:
    """Decode one MemberSpace."""
    variant = reader.read_number()

    if variant == 0:
        return "instance"
    elif variant == 1:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_space(value: MemberSpace) -> Json:
    """Return one JSON value for one MemberSpace."""
    return value


def from_json_member_space(value: Json) -> MemberSpace:
    """Return one MemberSpace from one JSON value."""
    variant = json_string(value)

    if variant == "instance":
        return "instance"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MethodDefinition:
    """One method member."""

    # the member space declaring the method
    space: MemberSpace
    # the method symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the nominal member slot
    slot: destack._generated.dir.tree.property.MemberSlot
    # the method role
    role: destack._generated.dir.tree.property.FunctionRole | None
    # the abstraction mode governing overrides
    abstraction: destack._generated.dir.tree.property.MethodAbstraction
    # whether the method overrides an inherited member
    is_override: bool
    # how the method receives its implementation
    implementation: MethodImplementation

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_method_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MethodDefinition:
        """Decode one MethodDefinition."""
        return decode_method_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_method_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> MethodDefinition:
        """Return one MethodDefinition from one JSON value."""
        return from_json_method_definition(value)


def encode_method_definition(writer: BinaryWriter, value: MethodDefinition) -> None:
    """Encode one MethodDefinition."""
    encode_member_space(writer, value.space)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.tree.property.encode_member_slot(writer, value.slot)
    if value.role is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.property.encode_function_role(writer, value.role)
    destack._generated.dir.tree.property.encode_method_abstraction(
        writer, value.abstraction
    )
    writer.write_bool(value.is_override)
    encode_method_implementation(writer, value.implementation)


def decode_method_definition(reader: BinaryReader) -> MethodDefinition:
    """Decode one MethodDefinition."""
    space = decode_member_space(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    slot = destack._generated.dir.tree.property.decode_member_slot(reader)
    role = reader.read_option(
        lambda: destack._generated.dir.tree.property.decode_function_role(reader)
    )
    abstraction = destack._generated.dir.tree.property.decode_method_abstraction(reader)
    is_override = reader.read_bool()
    implementation = decode_method_implementation(reader)

    return MethodDefinition(
        space=space,
        symbol=symbol,
        source=source,
        slot=slot,
        role=role,
        abstraction=abstraction,
        is_override=is_override,
        implementation=implementation,
    )


def to_json_method_definition(value: MethodDefinition) -> Json:
    """Return one JSON value for one MethodDefinition."""
    return {
        "space": to_json_member_space(value.space),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "slot": destack._generated.dir.tree.property.to_json_member_slot(value.slot),
        **(
            {}
            if value.role is None
            else {
                "role": destack._generated.dir.tree.property.to_json_function_role(
                    value.role
                )
            }
        ),
        "abstraction": destack._generated.dir.tree.property.to_json_method_abstraction(
            value.abstraction
        ),
        "isOverride": value.is_override,
        "implementation": to_json_method_implementation(value.implementation),
    }


def from_json_method_definition(value: Json) -> MethodDefinition:
    """Return one MethodDefinition from one JSON value."""
    object_ = json_object(value)

    return MethodDefinition(
        space=from_json_member_space(json_field(object_, "space")),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        slot=destack._generated.dir.tree.property.from_json_member_slot(
            json_field(object_, "slot")
        ),
        role=json_optional(
            object_,
            "role",
            lambda value: destack._generated.dir.tree.property.from_json_function_role(
                value
            ),
        ),
        abstraction=destack._generated.dir.tree.property.from_json_method_abstraction(
            json_field(object_, "abstraction")
        ),
        is_override=json_bool(json_field(object_, "isOverride")),
        implementation=from_json_method_implementation(
            json_field(object_, "implementation")
        ),
    )


"""How one method receives its implementation."""
MethodImplementation: typing.TypeAlias = (
    typing.Literal["required"]
    | typing.Literal["body"]
    | typing.Literal["default"]
    | typing.Literal["intrinsic"]
)


def encode_method_implementation(
    writer: BinaryWriter, value: MethodImplementation
) -> None:
    """Encode one MethodImplementation."""
    if value == "required":
        writer.write_unsigned(0)
    elif value == "body":
        writer.write_unsigned(1)
    elif value == "default":
        writer.write_unsigned(2)
    elif value == "intrinsic":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_method_implementation(reader: BinaryReader) -> MethodImplementation:
    """Decode one MethodImplementation."""
    variant = reader.read_number()

    if variant == 0:
        return "required"
    elif variant == 1:
        return "body"
    elif variant == 2:
        return "default"
    elif variant == 3:
        return "intrinsic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_method_implementation(value: MethodImplementation) -> Json:
    """Return one JSON value for one MethodImplementation."""
    return value


def from_json_method_implementation(value: Json) -> MethodImplementation:
    """Return one MethodImplementation from one JSON value."""
    variant = json_string(value)

    if variant == "required":
        return "required"
    elif variant == "body":
        return "body"
    elif variant == "default":
        return "default"
    elif variant == "intrinsic":
        return "intrinsic"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_associated_type_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssociatedTypeDefinition:
        """Decode one AssociatedTypeDefinition."""
        return decode_associated_type_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_associated_type_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> AssociatedTypeDefinition:
        """Return one AssociatedTypeDefinition from one JSON value."""
        return from_json_associated_type_definition(value)


def encode_associated_type_definition(
    writer: BinaryWriter, value: AssociatedTypeDefinition
) -> None:
    """Encode one AssociatedTypeDefinition."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    if value.constraint is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.constraint)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.value)


def decode_associated_type_definition(reader: BinaryReader) -> AssociatedTypeDefinition:
    """Decode one AssociatedTypeDefinition."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    constraint = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    value_ = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )

    return AssociatedTypeDefinition(
        symbol=symbol,
        source=source,
        key=key,
        constraint=constraint,
        value=value_,
    )


def to_json_associated_type_definition(value: AssociatedTypeDefinition) -> Json:
    """Return one JSON value for one AssociatedTypeDefinition."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        **(
            {}
            if value.constraint is None
            else {
                "constraint": destack._generated.dir.type.type.to_json_global_type_id(
                    value.constraint
                )
            }
        ),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.type.type.to_json_global_type_id(
                    value.value
                )
            }
        ),
    }


def from_json_associated_type_definition(value: Json) -> AssociatedTypeDefinition:
    """Return one AssociatedTypeDefinition from one JSON value."""
    object_ = json_object(value)

    return AssociatedTypeDefinition(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        constraint=json_optional(
            object_,
            "constraint",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class AssociatedConstDefinition:
    """One checked associated constant."""

    # the associated const symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the associated const key
    key: destack._generated.dir.symbol.key.StaticKey
    # the checked static value
    value: destack._generated.dir.tree.static.GlobalStaticId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_associated_const_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AssociatedConstDefinition:
        """Decode one AssociatedConstDefinition."""
        return decode_associated_const_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_associated_const_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> AssociatedConstDefinition:
        """Return one AssociatedConstDefinition from one JSON value."""
        return from_json_associated_const_definition(value)


def encode_associated_const_definition(
    writer: BinaryWriter, value: AssociatedConstDefinition
) -> None:
    """Encode one AssociatedConstDefinition."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.static.encode_global_static_id(writer, value.value)


def decode_associated_const_definition(
    reader: BinaryReader,
) -> AssociatedConstDefinition:
    """Decode one AssociatedConstDefinition."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.static.decode_global_static_id(reader)
    )

    return AssociatedConstDefinition(
        symbol=symbol,
        source=source,
        key=key,
        value=value_,
    )


def to_json_associated_const_definition(value: AssociatedConstDefinition) -> Json:
    """Return one JSON value for one AssociatedConstDefinition."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.tree.static.to_json_global_static_id(
                    value.value
                )
            }
        ),
    }


def from_json_associated_const_definition(value: Json) -> AssociatedConstDefinition:
    """Return one AssociatedConstDefinition from one JSON value."""
    object_ = json_object(value)

    return AssociatedConstDefinition(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.tree.static.from_json_global_static_id(
                value
            ),
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantDefinition:
        """Decode one VariantDefinition."""
        return decode_variant_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantDefinition:
        """Return one VariantDefinition from one JSON value."""
        return from_json_variant_definition(value)


def encode_variant_definition(writer: BinaryWriter, value: VariantDefinition) -> None:
    """Encode one VariantDefinition."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.static.encode_global_static_id(writer, value.value)


def decode_variant_definition(reader: BinaryReader) -> VariantDefinition:
    """Decode one VariantDefinition."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    value_ = reader.read_option(
        lambda: destack._generated.dir.tree.static.decode_global_static_id(reader)
    )

    return VariantDefinition(
        symbol=symbol,
        source=source,
        key=key,
        value=value_,
    )


def to_json_variant_definition(value: VariantDefinition) -> Json:
    """Return one JSON value for one VariantDefinition."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.dir.tree.static.to_json_global_static_id(
                    value.value
                )
            }
        ),
    }


def from_json_variant_definition(value: Json) -> VariantDefinition:
    """Return one VariantDefinition from one JSON value."""
    object_ = json_object(value)

    return VariantDefinition(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.dir.tree.static.from_json_global_static_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class SignatureDefinition:
    """One checked symbol-free signature member."""

    # the source member node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the checked signature type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureDefinition:
        """Decode one SignatureDefinition."""
        return decode_signature_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureDefinition:
        """Return one SignatureDefinition from one JSON value."""
        return from_json_signature_definition(value)


def encode_signature_definition(
    writer: BinaryWriter, value: SignatureDefinition
) -> None:
    """Encode one SignatureDefinition."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_signature_definition(reader: BinaryReader) -> SignatureDefinition:
    """Decode one SignatureDefinition."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return SignatureDefinition(
        source=source,
        ty=ty,
    )


def to_json_signature_definition(value: SignatureDefinition) -> Json:
    """Return one JSON value for one SignatureDefinition."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_signature_definition(value: Json) -> SignatureDefinition:
    """Return one SignatureDefinition from one JSON value."""
    object_ = json_object(value)

    return SignatureDefinition(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


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
    # the class's direct construct candidates
    constructors: Sequence[ClassConstructorDefinition]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassDefinition:
        """Decode one ClassDefinition."""
        return decode_class_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> ClassDefinition:
        """Return one ClassDefinition from one JSON value."""
        return from_json_class_definition(value)


def encode_class_definition(writer: BinaryWriter, value: ClassDefinition) -> None:
    """Encode one ClassDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_final)
    if value.extends is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_nominal_heritage(writer, value.extends)
    writer.write_unsigned(len(value.implements))
    for item_value_implements_0 in value.implements:
        encode_nominal_heritage(writer, item_value_implements_0)
    writer.write_unsigned(len(value.constructors))
    for item_value_constructors_0 in value.constructors:
        encode_class_constructor_definition(writer, item_value_constructors_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_class_definition(reader: BinaryReader) -> ClassDefinition:
    """Decode one ClassDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    is_abstract = reader.read_bool()
    is_final = reader.read_bool()
    extends = reader.read_option(lambda: decode_nominal_heritage(reader))
    implements = [decode_nominal_heritage(reader) for _ in range(reader.read_number())]
    constructors = [
        decode_class_constructor_definition(reader) for _ in range(reader.read_number())
    ]
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return ClassDefinition(
        template=template,
        is_abstract=is_abstract,
        is_final=is_final,
        extends=extends,
        implements=implements,
        constructors=constructors,
        members=members,
    )


def to_json_class_definition(value: ClassDefinition) -> Json:
    """Return one JSON value for one ClassDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "isAbstract": value.is_abstract,
        "isFinal": value.is_final,
        **(
            {}
            if value.extends is None
            else {"extends": to_json_nominal_heritage(value.extends)}
        ),
        "implements": [to_json_nominal_heritage(item_0) for item_0 in value.implements],
        "constructors": [
            to_json_class_constructor_definition(item_0)
            for item_0 in value.constructors
        ],
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_class_definition(value: Json) -> ClassDefinition:
    """Return one ClassDefinition from one JSON value."""
    object_ = json_object(value)

    return ClassDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_final=json_bool(json_field(object_, "isFinal")),
        extends=json_optional(
            object_, "extends", lambda value: from_json_nominal_heritage(value)
        ),
        implements=[
            from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "implements"))
        ],
        constructors=[
            from_json_class_constructor_definition(item_0)
            for item_0 in json_array(json_field(object_, "constructors"))
        ],
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ClassConstructorDefinition:
    """One class construct candidate."""

    # the selected constructor
    constructor: ClassConstructor
    # the checked constructor signature
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_class_constructor_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ClassConstructorDefinition:
        """Decode one ClassConstructorDefinition."""
        return decode_class_constructor_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_class_constructor_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> ClassConstructorDefinition:
        """Return one ClassConstructorDefinition from one JSON value."""
        return from_json_class_constructor_definition(value)


def encode_class_constructor_definition(
    writer: BinaryWriter, value: ClassConstructorDefinition
) -> None:
    """Encode one ClassConstructorDefinition."""
    encode_class_constructor(writer, value.constructor)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_class_constructor_definition(
    reader: BinaryReader,
) -> ClassConstructorDefinition:
    """Decode one ClassConstructorDefinition."""
    constructor = decode_class_constructor(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ClassConstructorDefinition(
        constructor=constructor,
        ty=ty,
    )


def to_json_class_constructor_definition(value: ClassConstructorDefinition) -> Json:
    """Return one JSON value for one ClassConstructorDefinition."""
    return {
        "constructor": to_json_class_constructor(value.constructor),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_class_constructor_definition(value: Json) -> ClassConstructorDefinition:
    """Return one ClassConstructorDefinition from one JSON value."""
    object_ = json_object(value)

    return ClassConstructorDefinition(
        constructor=from_json_class_constructor(json_field(object_, "constructor")),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_interface_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InterfaceDefinition:
        """Decode one InterfaceDefinition."""
        return decode_interface_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_interface_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> InterfaceDefinition:
        """Return one InterfaceDefinition from one JSON value."""
        return from_json_interface_definition(value)


def encode_interface_definition(
    writer: BinaryWriter, value: InterfaceDefinition
) -> None:
    """Encode one InterfaceDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    writer.write_bool(value.is_nominal)
    writer.write_unsigned(len(value.extends))
    for item_value_extends_0 in value.extends:
        encode_nominal_heritage(writer, item_value_extends_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_interface_definition(reader: BinaryReader) -> InterfaceDefinition:
    """Decode one InterfaceDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    is_nominal = reader.read_bool()
    extends = [decode_nominal_heritage(reader) for _ in range(reader.read_number())]
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return InterfaceDefinition(
        template=template,
        is_nominal=is_nominal,
        extends=extends,
        members=members,
    )


def to_json_interface_definition(value: InterfaceDefinition) -> Json:
    """Return one JSON value for one InterfaceDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "isNominal": value.is_nominal,
        "extends": [to_json_nominal_heritage(item_0) for item_0 in value.extends],
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_interface_definition(value: Json) -> InterfaceDefinition:
    """Return one InterfaceDefinition from one JSON value."""
    object_ = json_object(value)

    return InterfaceDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        is_nominal=json_bool(json_field(object_, "isNominal")),
        extends=[
            from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "extends"))
        ],
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class EnumDefinition:
    """Checked declaration data for one nominal enum."""

    # the generic template declared by the enum
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_enum_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EnumDefinition:
        """Decode one EnumDefinition."""
        return decode_enum_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_enum_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> EnumDefinition:
        """Return one EnumDefinition from one JSON value."""
        return from_json_enum_definition(value)


def encode_enum_definition(writer: BinaryWriter, value: EnumDefinition) -> None:
    """Encode one EnumDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    writer.write_unsigned(len(value.implements))
    for item_value_implements_0 in value.implements:
        encode_nominal_heritage(writer, item_value_implements_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_enum_definition(reader: BinaryReader) -> EnumDefinition:
    """Decode one EnumDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    implements = [decode_nominal_heritage(reader) for _ in range(reader.read_number())]
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return EnumDefinition(
        template=template,
        implements=implements,
        members=members,
    )


def to_json_enum_definition(value: EnumDefinition) -> Json:
    """Return one JSON value for one EnumDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "implements": [to_json_nominal_heritage(item_0) for item_0 in value.implements],
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_enum_definition(value: Json) -> EnumDefinition:
    """Return one EnumDefinition from one JSON value."""
    object_ = json_object(value)

    return EnumDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        implements=[
            from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "implements"))
        ],
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NewtypeDefinition:
    """Checked declaration data for one nominal type alias."""

    # the generic template declared by the newtype
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the nominal backing type
    value: destack._generated.dir.type.type.GlobalTypeId
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_newtype_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NewtypeDefinition:
        """Decode one NewtypeDefinition."""
        return decode_newtype_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_newtype_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> NewtypeDefinition:
        """Return one NewtypeDefinition from one JSON value."""
        return from_json_newtype_definition(value)


def encode_newtype_definition(writer: BinaryWriter, value: NewtypeDefinition) -> None:
    """Encode one NewtypeDefinition."""
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    destack._generated.dir.type.type.encode_global_type_id(writer, value.value)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_newtype_definition(reader: BinaryReader) -> NewtypeDefinition:
    """Decode one NewtypeDefinition."""
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    value_ = destack._generated.dir.type.type.decode_global_type_id(reader)
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return NewtypeDefinition(
        template=template,
        value=value_,
        members=members,
    )


def to_json_newtype_definition(value: NewtypeDefinition) -> Json:
    """Return one JSON value for one NewtypeDefinition."""
    return {
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "value": destack._generated.dir.type.type.to_json_global_type_id(value.value),
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_newtype_definition(value: Json) -> NewtypeDefinition:
    """Return one NewtypeDefinition from one JSON value."""
    object_ = json_object(value)

    return NewtypeDefinition(
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        value=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "value")
        ),
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ExtensionDefinition:
    """Checked declaration data for one extension."""

    # the extension declaration's symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the extension declaration form
    form: ExtensionForm
    # the extension's generic template
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the checked receiver target
    target: ExtensionTarget
    # the implemented interfaces
    implements: Sequence[NominalHeritage]
    # the members in declaration order
    members: Sequence[DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_definition(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionDefinition:
        """Decode one ExtensionDefinition."""
        return decode_extension_definition(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_definition(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionDefinition:
        """Return one ExtensionDefinition from one JSON value."""
        return from_json_extension_definition(value)


def encode_extension_definition(
    writer: BinaryWriter, value: ExtensionDefinition
) -> None:
    """Encode one ExtensionDefinition."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    encode_extension_form(writer, value.form)
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    encode_extension_target(writer, value.target)
    writer.write_unsigned(len(value.implements))
    for item_value_implements_0 in value.implements:
        encode_nominal_heritage(writer, item_value_implements_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_definition_member(writer, item_value_members_0)


def decode_extension_definition(reader: BinaryReader) -> ExtensionDefinition:
    """Decode one ExtensionDefinition."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    form = decode_extension_form(reader)
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    target = decode_extension_target(reader)
    implements = [decode_nominal_heritage(reader) for _ in range(reader.read_number())]
    members = [decode_definition_member(reader) for _ in range(reader.read_number())]

    return ExtensionDefinition(
        symbol=symbol,
        form=form,
        template=template,
        target=target,
        implements=implements,
        members=members,
    )


def to_json_extension_definition(value: ExtensionDefinition) -> Json:
    """Return one JSON value for one ExtensionDefinition."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "form": to_json_extension_form(value.form),
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "target": to_json_extension_target(value.target),
        "implements": [to_json_nominal_heritage(item_0) for item_0 in value.implements],
        "members": [to_json_definition_member(item_0) for item_0 in value.members],
    }


def from_json_extension_definition(value: Json) -> ExtensionDefinition:
    """Return one ExtensionDefinition from one JSON value."""
    object_ = json_object(value)

    return ExtensionDefinition(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        form=from_json_extension_form(json_field(object_, "form")),
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        target=from_json_extension_target(json_field(object_, "target")),
        implements=[
            from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "implements"))
        ],
        members=[
            from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


"""How an extension declaration relates to its target type."""
ExtensionForm: typing.TypeAlias = typing.Literal["local"] | typing.Literal["exported"]


def encode_extension_form(writer: BinaryWriter, value: ExtensionForm) -> None:
    """Encode one ExtensionForm."""
    if value == "local":
        writer.write_unsigned(0)
    elif value == "exported":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_extension_form(reader: BinaryReader) -> ExtensionForm:
    """Decode one ExtensionForm."""
    variant = reader.read_number()

    if variant == 0:
        return "local"
    elif variant == 1:
        return "exported"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_extension_form(value: ExtensionForm) -> Json:
    """Return one JSON value for one ExtensionForm."""
    return value


def from_json_extension_form(value: Json) -> ExtensionForm:
    """Return one ExtensionForm from one JSON value."""
    variant = json_string(value)

    if variant == "local":
        return "local"
    elif variant == "exported":
        return "exported"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ExtensionTargetRooted:
    """Extension whose receiver type has a lookup root."""

    # the declaration root used for member lookup
    root: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["rooted"] = "rooted"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_target(self)


@dataclass(frozen=True, slots=True)
class ExtensionTargetBlanket:
    """Extension over an open receiver type."""

    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["blanket"] = "blanket"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_target(self)


"""Extension lookup target."""
ExtensionTarget: typing.TypeAlias = ExtensionTargetRooted | ExtensionTargetBlanket


def encode_extension_target(writer: BinaryWriter, value: ExtensionTarget) -> None:
    """Encode one ExtensionTarget."""
    if value.kind == "rooted":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.root)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "blanket":
        writer.write_unsigned(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    else:
        raise SerdeError("unknown enum variant")


def decode_extension_target(reader: BinaryReader) -> ExtensionTarget:
    """Decode one ExtensionTarget."""
    variant = reader.read_number()

    if variant == 0:
        root = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ExtensionTargetRooted(
            root=root,
            ty=ty,
        )
    elif variant == 1:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ExtensionTargetBlanket(
            ty=ty,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_extension_target(value: ExtensionTarget) -> Json:
    """Return one JSON value for one ExtensionTarget."""
    if value.kind == "rooted":
        return {
            "kind": "rooted",
            "root": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.root
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "blanket":
        return {
            "kind": "blanket",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_extension_target(value: Json) -> ExtensionTarget:
    """Return one ExtensionTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "rooted":
        return ExtensionTargetRooted(
            root=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "root")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "blanket":
        return ExtensionTargetBlanket(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ClassConstructor",
    "encode_class_constructor",
    "decode_class_constructor",
    "to_json_class_constructor",
    "from_json_class_constructor",
    "ClassConstructorDeclared",
    "ClassConstructorDefault",
    "ClassConstructorForwardedDeclared",
    "ClassConstructorForwardedDefault",
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
    "MethodImplementation",
    "encode_method_implementation",
    "decode_method_implementation",
    "to_json_method_implementation",
    "from_json_method_implementation",
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
    "ClassConstructorDefinition",
    "encode_class_constructor_definition",
    "decode_class_constructor_definition",
    "to_json_class_constructor_definition",
    "from_json_class_constructor_definition",
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
    "ExtensionDefinition",
    "encode_extension_definition",
    "decode_extension_definition",
    "to_json_extension_definition",
    "from_json_extension_definition",
    "ExtensionForm",
    "encode_extension_form",
    "decode_extension_form",
    "to_json_extension_form",
    "from_json_extension_form",
    "ExtensionTarget",
    "encode_extension_target",
    "decode_extension_target",
    "to_json_extension_target",
    "from_json_extension_target",
    "ExtensionTargetRooted",
    "ExtensionTargetBlanket",
]
