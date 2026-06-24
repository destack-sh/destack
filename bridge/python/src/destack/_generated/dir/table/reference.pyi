# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ReferenceTable:
    """Name resolutions for one module, keyed by the reference node."""

    # the module id of the reference table
    module_id: destack._generated.source.file.model.module.ModuleId
    # resolved references keyed by their source node
    entries: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, Reference]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ReferenceTable: ...

def encode_reference_table(writer: BinaryWriter, value: ReferenceTable) -> None: ...
def decode_reference_table(reader: BinaryReader) -> ReferenceTable: ...
def to_json_reference_table(value: ReferenceTable) -> Json: ...
def from_json_reference_table(value: Json) -> ReferenceTable: ...

@dataclass(frozen=True, slots=True)
class ReferenceBound:
    """Resolved to declarations by name: lexical scope or a full namespace path."""

    bound: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]
    kind: typing.Literal["bound"] = "bound"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ReferenceNamespace:
    """Resolved to a namespace: a prefix awaiting a further segment, or a bare namespace value."""

    namespace: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ReferenceProjected:
    """A flat path named through its first segments; `segments[from..]` project as members off `base`."""

    # the declaration the leading segments name
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the segment index where member projection begins
    from_: int
    kind: typing.Literal["projected"] = "projected"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ReferenceAmbiguous:
    """Conflicting bindings with no single winner."""

    ambiguous: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]
    kind: typing.Literal["ambiguous"] = "ambiguous"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ReferenceMissing:
    """No binding by name."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""How one source reference resolves by name, before types and conditions apply."""
Reference: typing.TypeAlias = (
    ReferenceBound
    | ReferenceNamespace
    | ReferenceProjected
    | ReferenceAmbiguous
    | ReferenceMissing
)

def encode_reference(writer: BinaryWriter, value: Reference) -> None: ...
def decode_reference(reader: BinaryReader) -> Reference: ...
def to_json_reference(value: Reference) -> Json: ...
def from_json_reference(value: Json) -> Reference: ...

__all__ = [
    "ReferenceTable",
    "encode_reference_table",
    "decode_reference_table",
    "to_json_reference_table",
    "from_json_reference_table",
    "Reference",
    "encode_reference",
    "decode_reference",
    "to_json_reference",
    "from_json_reference",
    "ReferenceBound",
    "ReferenceNamespace",
    "ReferenceProjected",
    "ReferenceAmbiguous",
    "ReferenceMissing",
]
