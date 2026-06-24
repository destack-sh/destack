# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.scope
import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GlobalSymbolId:
    """Global symbol id across modules."""

    # the module id of the global symbol
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global symbol
    local_id: LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_symbol_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalSymbolId:
        """Decode one GlobalSymbolId."""
        return decode_global_symbol_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_symbol_id(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalSymbolId:
        """Return one GlobalSymbolId from one JSON value."""
        return from_json_global_symbol_id(value)


def encode_global_symbol_id(writer: BinaryWriter, value: GlobalSymbolId) -> None:
    """Encode one GlobalSymbolId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_symbol_id(writer, value.local_id)


def decode_global_symbol_id(reader: BinaryReader) -> GlobalSymbolId:
    """Decode one GlobalSymbolId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = decode_local_symbol_id(reader)

    return GlobalSymbolId(
        module_id=module_id,
        local_id=local_id,
    )


def to_json_global_symbol_id(value: GlobalSymbolId) -> Json:
    """Return one JSON value for one GlobalSymbolId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": to_json_local_symbol_id(value.local_id),
    }


def from_json_global_symbol_id(value: Json) -> GlobalSymbolId:
    """Return one GlobalSymbolId from one JSON value."""
    object_ = json_object(value)

    return GlobalSymbolId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=from_json_local_symbol_id(json_field(object_, "localId")),
    )


@dataclass(frozen=True, slots=True)
class LocalSymbolId:
    """Unique identifier for Symbols."""

    # the numeric id
    id: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_symbol_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalSymbolId:
        """Decode one LocalSymbolId."""
        return decode_local_symbol_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_symbol_id(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalSymbolId:
        """Return one LocalSymbolId from one JSON value."""
        return from_json_local_symbol_id(value)


def encode_local_symbol_id(writer: BinaryWriter, value: LocalSymbolId) -> None:
    """Encode one LocalSymbolId."""
    writer.write_unsigned(value.id)


def decode_local_symbol_id(reader: BinaryReader) -> LocalSymbolId:
    """Decode one LocalSymbolId."""
    id = reader.read_number()

    return LocalSymbolId(
        id=id,
    )


def to_json_local_symbol_id(value: LocalSymbolId) -> Json:
    """Return one JSON value for one LocalSymbolId."""
    return {
        "id": value.id,
    }


def from_json_local_symbol_id(value: Json) -> LocalSymbolId:
    """Return one LocalSymbolId from one JSON value."""
    object_ = json_object(value)

    return LocalSymbolId(
        id=json_int(json_field(object_, "id")),
    )


@dataclass(frozen=True, slots=True)
class Symbol:
    """A bindable item or local in a scope."""

    # the scope lookup role of the symbol
    role: SymbolRole
    # the declaration kind of the symbol
    kind: SymbolKind
    # the mutability for value bindings when known
    binding_mutability: destack._generated.dir.tree.node.Mutability | None
    # where this symbol was introduced
    origin: SymbolOrigin
    # the key of the symbol
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the scope that introduces the symbol
    scope: destack._generated.dir.symbol.scope.LocalScope
    # the export kind of the symbol
    export_kind: destack._generated.dir.tree.dependency.ExportKind | None
    # the declaration node that introduced this symbol
    declaration: destack._generated.dir.tree.node.GlobalNodeIdAny | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Symbol:
        """Decode one Symbol."""
        return decode_symbol(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol(self)

    @classmethod
    def from_json(cls, value: Json) -> Symbol:
        """Return one Symbol from one JSON value."""
        return from_json_symbol(value)


def encode_symbol(writer: BinaryWriter, value: Symbol) -> None:
    """Encode one Symbol."""
    encode_symbol_role(writer, value.role)
    encode_symbol_kind(writer, value.kind)
    if value.binding_mutability is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_mutability(
            writer, value.binding_mutability
        )
    encode_symbol_origin(writer, value.origin)
    if value.key is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.symbol.scope.encode_local_scope(writer, value.scope)
    if value.export_kind is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.dependency.encode_export_kind(
            writer, value.export_kind
        )
    if value.declaration is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, value.declaration
        )


def decode_symbol(reader: BinaryReader) -> Symbol:
    """Decode one Symbol."""
    role = decode_symbol_role(reader)
    kind = decode_symbol_kind(reader)
    binding_mutability = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_mutability(reader)
    )
    origin = decode_symbol_origin(reader)
    key = reader.read_option(
        lambda: destack._generated.dir.symbol.key.decode_static_key(reader)
    )
    scope = destack._generated.dir.symbol.scope.decode_local_scope(reader)
    export_kind = reader.read_option(
        lambda: destack._generated.dir.tree.dependency.decode_export_kind(reader)
    )
    declaration = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    )

    return Symbol(
        role=role,
        kind=kind,
        binding_mutability=binding_mutability,
        origin=origin,
        key=key,
        scope=scope,
        export_kind=export_kind,
        declaration=declaration,
    )


def to_json_symbol(value: Symbol) -> Json:
    """Return one JSON value for one Symbol."""
    return {
        "role": to_json_symbol_role(value.role),
        "kind": to_json_symbol_kind(value.kind),
        **(
            {}
            if value.binding_mutability is None
            else {
                "bindingMutability": destack._generated.dir.tree.node.to_json_mutability(
                    value.binding_mutability
                )
            }
        ),
        "origin": to_json_symbol_origin(value.origin),
        **(
            {}
            if value.key is None
            else {
                "key": destack._generated.dir.symbol.key.to_json_static_key(value.key)
            }
        ),
        "scope": destack._generated.dir.symbol.scope.to_json_local_scope(value.scope),
        **(
            {}
            if value.export_kind is None
            else {
                "exportKind": destack._generated.dir.tree.dependency.to_json_export_kind(
                    value.export_kind
                )
            }
        ),
        **(
            {}
            if value.declaration is None
            else {
                "declaration": destack._generated.dir.tree.node.to_json_global_node_id_any(
                    value.declaration
                )
            }
        ),
    }


def from_json_symbol(value: Json) -> Symbol:
    """Return one Symbol from one JSON value."""
    object_ = json_object(value)

    return Symbol(
        role=from_json_symbol_role(json_field(object_, "role")),
        kind=from_json_symbol_kind(json_field(object_, "kind")),
        binding_mutability=json_optional(
            object_,
            "bindingMutability",
            lambda value: destack._generated.dir.tree.node.from_json_mutability(value),
        ),
        origin=from_json_symbol_origin(json_field(object_, "origin")),
        key=json_optional(
            object_,
            "key",
            lambda value: destack._generated.dir.symbol.key.from_json_static_key(value),
        ),
        scope=destack._generated.dir.symbol.scope.from_json_local_scope(
            json_field(object_, "scope")
        ),
        export_kind=json_optional(
            object_,
            "exportKind",
            lambda value: destack._generated.dir.tree.dependency.from_json_export_kind(
                value
            ),
        ),
        declaration=json_optional(
            object_,
            "declaration",
            lambda value: destack._generated.dir.tree.node.from_json_global_node_id_any(
                value
            ),
        ),
    )


"""The scope lookup role of a symbol."""
SymbolRole: typing.TypeAlias = (
    typing.Literal["namespace"] | typing.Literal["item"] | typing.Literal["local"]
)


def encode_symbol_role(writer: BinaryWriter, value: SymbolRole) -> None:
    """Encode one SymbolRole."""
    if value == "namespace":
        writer.write_unsigned(0)
    elif value == "item":
        writer.write_unsigned(1)
    elif value == "local":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_role(reader: BinaryReader) -> SymbolRole:
    """Decode one SymbolRole."""
    variant = reader.read_number()

    if variant == 0:
        return "namespace"
    elif variant == 1:
        return "item"
    elif variant == 2:
        return "local"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_role(value: SymbolRole) -> Json:
    """Return one JSON value for one SymbolRole."""
    return value


def from_json_symbol_role(value: Json) -> SymbolRole:
    """Return one SymbolRole from one JSON value."""
    variant = json_string(value)

    if variant == "namespace":
        return "namespace"
    elif variant == "item":
        return "item"
    elif variant == "local":
        return "local"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The declaration kind of a symbol."""
SymbolKind: typing.TypeAlias = (
    typing.Literal["variable"]
    | typing.Literal["import"]
    | typing.Literal["class"]
    | typing.Literal["struct"]
    | typing.Literal["interface"]
    | typing.Literal["newtypeInterface"]
    | typing.Literal["enum"]
    | typing.Literal["enumField"]
    | typing.Literal["function"]
    | typing.Literal["label"]
    | typing.Literal["extension"]
    | typing.Literal["typeAlias"]
    | typing.Literal["genericTypeParameter"]
    | typing.Literal["genericValueParameter"]
    | typing.Literal["associatedType"]
    | typing.Literal["associatedConst"]
    | typing.Literal["newtype"]
)


def encode_symbol_kind(writer: BinaryWriter, value: SymbolKind) -> None:
    """Encode one SymbolKind."""
    if value == "variable":
        writer.write_unsigned(0)
    elif value == "import":
        writer.write_unsigned(1)
    elif value == "class":
        writer.write_unsigned(2)
    elif value == "struct":
        writer.write_unsigned(3)
    elif value == "interface":
        writer.write_unsigned(4)
    elif value == "newtypeInterface":
        writer.write_unsigned(5)
    elif value == "enum":
        writer.write_unsigned(6)
    elif value == "enumField":
        writer.write_unsigned(7)
    elif value == "function":
        writer.write_unsigned(8)
    elif value == "label":
        writer.write_unsigned(9)
    elif value == "extension":
        writer.write_unsigned(10)
    elif value == "typeAlias":
        writer.write_unsigned(11)
    elif value == "genericTypeParameter":
        writer.write_unsigned(12)
    elif value == "genericValueParameter":
        writer.write_unsigned(13)
    elif value == "associatedType":
        writer.write_unsigned(14)
    elif value == "associatedConst":
        writer.write_unsigned(15)
    elif value == "newtype":
        writer.write_unsigned(16)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_kind(reader: BinaryReader) -> SymbolKind:
    """Decode one SymbolKind."""
    variant = reader.read_number()

    if variant == 0:
        return "variable"
    elif variant == 1:
        return "import"
    elif variant == 2:
        return "class"
    elif variant == 3:
        return "struct"
    elif variant == 4:
        return "interface"
    elif variant == 5:
        return "newtypeInterface"
    elif variant == 6:
        return "enum"
    elif variant == 7:
        return "enumField"
    elif variant == 8:
        return "function"
    elif variant == 9:
        return "label"
    elif variant == 10:
        return "extension"
    elif variant == 11:
        return "typeAlias"
    elif variant == 12:
        return "genericTypeParameter"
    elif variant == 13:
        return "genericValueParameter"
    elif variant == 14:
        return "associatedType"
    elif variant == 15:
        return "associatedConst"
    elif variant == 16:
        return "newtype"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_kind(value: SymbolKind) -> Json:
    """Return one JSON value for one SymbolKind."""
    return value


def from_json_symbol_kind(value: Json) -> SymbolKind:
    """Return one SymbolKind from one JSON value."""
    variant = json_string(value)

    if variant == "variable":
        return "variable"
    elif variant == "import":
        return "import"
    elif variant == "class":
        return "class"
    elif variant == "struct":
        return "struct"
    elif variant == "interface":
        return "interface"
    elif variant == "newtypeInterface":
        return "newtypeInterface"
    elif variant == "enum":
        return "enum"
    elif variant == "enumField":
        return "enumField"
    elif variant == "function":
        return "function"
    elif variant == "label":
        return "label"
    elif variant == "extension":
        return "extension"
    elif variant == "typeAlias":
        return "typeAlias"
    elif variant == "genericTypeParameter":
        return "genericTypeParameter"
    elif variant == "genericValueParameter":
        return "genericValueParameter"
    elif variant == "associatedType":
        return "associatedType"
    elif variant == "associatedConst":
        return "associatedConst"
    elif variant == "newtype":
        return "newtype"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Where a symbol originated in the source."""
SymbolOrigin: typing.TypeAlias = typing.Literal["module"] | typing.Literal["global"]


def encode_symbol_origin(writer: BinaryWriter, value: SymbolOrigin) -> None:
    """Encode one SymbolOrigin."""
    if value == "module":
        writer.write_unsigned(0)
    elif value == "global":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_origin(reader: BinaryReader) -> SymbolOrigin:
    """Decode one SymbolOrigin."""
    variant = reader.read_number()

    if variant == 0:
        return "module"
    elif variant == 1:
        return "global"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_origin(value: SymbolOrigin) -> Json:
    """Return one JSON value for one SymbolOrigin."""
    return value


def from_json_symbol_origin(value: Json) -> SymbolOrigin:
    """Return one SymbolOrigin from one JSON value."""
    variant = json_string(value)

    if variant == "module":
        return "module"
    elif variant == "global":
        return "global"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The space of a symbol."""
SymbolSpace: typing.TypeAlias = (
    typing.Literal["type"] | typing.Literal["value"] | typing.Literal["label"]
)


def encode_symbol_space(writer: BinaryWriter, value: SymbolSpace) -> None:
    """Encode one SymbolSpace."""
    if value == "type":
        writer.write_unsigned(0)
    elif value == "value":
        writer.write_unsigned(1)
    elif value == "label":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_space(reader: BinaryReader) -> SymbolSpace:
    """Decode one SymbolSpace."""
    variant = reader.read_number()

    if variant == 0:
        return "type"
    elif variant == 1:
        return "value"
    elif variant == 2:
        return "label"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_space(value: SymbolSpace) -> Json:
    """Return one JSON value for one SymbolSpace."""
    return value


def from_json_symbol_space(value: Json) -> SymbolSpace:
    """Return one SymbolSpace from one JSON value."""
    variant = json_string(value)

    if variant == "type":
        return "type"
    elif variant == "value":
        return "value"
    elif variant == "label":
        return "label"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "GlobalSymbolId",
    "encode_global_symbol_id",
    "decode_global_symbol_id",
    "to_json_global_symbol_id",
    "from_json_global_symbol_id",
    "LocalSymbolId",
    "encode_local_symbol_id",
    "decode_local_symbol_id",
    "to_json_local_symbol_id",
    "from_json_local_symbol_id",
    "Symbol",
    "encode_symbol",
    "decode_symbol",
    "to_json_symbol",
    "from_json_symbol",
    "SymbolRole",
    "encode_symbol_role",
    "decode_symbol_role",
    "to_json_symbol_role",
    "from_json_symbol_role",
    "SymbolKind",
    "encode_symbol_kind",
    "decode_symbol_kind",
    "to_json_symbol_kind",
    "from_json_symbol_kind",
    "SymbolOrigin",
    "encode_symbol_origin",
    "decode_symbol_origin",
    "to_json_symbol_origin",
    "from_json_symbol_origin",
    "SymbolSpace",
    "encode_symbol_space",
    "decode_symbol_space",
    "to_json_symbol_space",
    "from_json_symbol_space",
]
