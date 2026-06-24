# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.generic
import destack._generated.dir.type.type

@dataclass(frozen=True, slots=True)
class Extension:
    """A resolved extension declaration."""

    # the extension declaration's symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the extension declaration form
    form: ExtensionForm
    # the extension's generic template
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the checked receiver target
    target: ExtensionTarget
    # the implemented interfaces
    implements: Sequence[destack._generated.dir.table.definition.NominalHeritage]
    # the checked where clauses that gate this extension
    where_clauses: Sequence[ExtensionWhereClause]
    # the members in declaration order
    members: Sequence[destack._generated.dir.table.definition.DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Extension: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Extension: ...

def encode_extension(writer: BinaryWriter, value: Extension) -> None: ...
def decode_extension(reader: BinaryReader) -> Extension: ...
def to_json_extension(value: Extension) -> Json: ...
def from_json_extension(value: Json) -> Extension: ...

"""How an extension declaration relates to its target type."""
ExtensionForm: typing.TypeAlias = (
    typing.Literal["inherent"] | typing.Literal["local"] | typing.Literal["named"]
)

def encode_extension_form(writer: BinaryWriter, value: ExtensionForm) -> None: ...
def decode_extension_form(reader: BinaryReader) -> ExtensionForm: ...
def to_json_extension_form(value: ExtensionForm) -> Json: ...
def from_json_extension_form(value: Json) -> ExtensionForm: ...

@dataclass(frozen=True, slots=True)
class ExtensionTargetNominal:
    """Extension whose receiver type has a nominal root."""

    # the nominal root used for member lookup
    root: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ExtensionTargetBlanket:
    """Extension over an open receiver type."""

    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["blanket"] = "blanket"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Extension lookup target."""
ExtensionTarget: typing.TypeAlias = ExtensionTargetNominal | ExtensionTargetBlanket

def encode_extension_target(writer: BinaryWriter, value: ExtensionTarget) -> None: ...
def decode_extension_target(reader: BinaryReader) -> ExtensionTarget: ...
def to_json_extension_target(value: ExtensionTarget) -> Json: ...
def from_json_extension_target(value: Json) -> ExtensionTarget: ...

@dataclass(frozen=True, slots=True)
class ExtensionWhereClause:
    """A checked where clause attached to one extension."""

    # the source where clause node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the constrained type
    left: destack._generated.dir.type.type.GlobalTypeId
    # the required constraint type
    right: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionWhereClause: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExtensionWhereClause: ...

def encode_extension_where_clause(
    writer: BinaryWriter, value: ExtensionWhereClause
) -> None: ...
def decode_extension_where_clause(reader: BinaryReader) -> ExtensionWhereClause: ...
def to_json_extension_where_clause(value: ExtensionWhereClause) -> Json: ...
def from_json_extension_where_clause(value: Json) -> ExtensionWhereClause: ...

__all__ = [
    "Extension",
    "encode_extension",
    "decode_extension",
    "to_json_extension",
    "from_json_extension",
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
    "ExtensionTargetNominal",
    "ExtensionTargetBlanket",
    "ExtensionWhereClause",
    "encode_extension_where_clause",
    "decode_extension_where_clause",
    "to_json_extension_where_clause",
    "from_json_extension_where_clause",
]
