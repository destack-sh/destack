# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class DecoratorSegment:
    """Decorator applications added by one DIR phase."""

    # the module id of the decorator segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first decorator application id owned by this table segment
    first_application_id: int
    # decorator applications owned by this segment
    applications: Sequence[DecoratorApplication]
    # decorator applications attached to each owner
    applications_by_owner: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny, Sequence[LocalDecoratorId]
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorSegment: ...

def encode_decorator_segment(writer: BinaryWriter, value: DecoratorSegment) -> None: ...
def decode_decorator_segment(reader: BinaryReader) -> DecoratorSegment: ...
def to_json_decorator_segment(value: DecoratorSegment) -> Json: ...
def from_json_decorator_segment(value: Json) -> DecoratorSegment: ...

@dataclass(frozen=True, slots=True)
class DecoratorApplication:
    """Checked decorator application attached to one owner node."""

    # the decorator node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the node decorated by this application
    owner: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator target expression
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved decorator target
    resolution: DecoratorResolution
    # the application arguments
    arguments: Sequence[DecoratorArgument]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorApplication: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorApplication: ...

def encode_decorator_application(
    writer: BinaryWriter, value: DecoratorApplication
) -> None: ...
def decode_decorator_application(reader: BinaryReader) -> DecoratorApplication: ...
def to_json_decorator_application(value: DecoratorApplication) -> Json: ...
def from_json_decorator_application(value: Json) -> DecoratorApplication: ...

@dataclass(frozen=True, slots=True)
class DecoratorResolutionLanguageItem:
    """Compiler language item decorator."""

    language_item: destack._generated.dir.symbol.language.LanguageItem
    kind: typing.Literal["languageItem"] = "languageItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DecoratorResolutionSymbol:
    """User-defined decorator symbol."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DecoratorResolutionUnresolved:
    """Unresolved or non-symbol decorator target."""

    kind: typing.Literal["unresolved"] = "unresolved"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Resolved decorator target."""
DecoratorResolution: typing.TypeAlias = (
    DecoratorResolutionLanguageItem
    | DecoratorResolutionSymbol
    | DecoratorResolutionUnresolved
)

def encode_decorator_resolution(
    writer: BinaryWriter, value: DecoratorResolution
) -> None: ...
def decode_decorator_resolution(reader: BinaryReader) -> DecoratorResolution: ...
def to_json_decorator_resolution(value: DecoratorResolution) -> Json: ...
def from_json_decorator_resolution(value: Json) -> DecoratorResolution: ...

@dataclass(frozen=True, slots=True)
class DecoratorArgument:
    """Checked decorator argument."""

    # the argument node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the committed static value when one exists
    value: destack._generated.dir.tree.static.GlobalStaticId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorArgument: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorArgument: ...

def encode_decorator_argument(
    writer: BinaryWriter, value: DecoratorArgument
) -> None: ...
def decode_decorator_argument(reader: BinaryReader) -> DecoratorArgument: ...
def to_json_decorator_argument(value: DecoratorArgument) -> Json: ...
def from_json_decorator_argument(value: Json) -> DecoratorArgument: ...

"""Unique identifier for a decorator application."""
LocalDecoratorId: typing.TypeAlias = int

def encode_local_decorator_id(
    writer: BinaryWriter, value: LocalDecoratorId
) -> None: ...
def decode_local_decorator_id(reader: BinaryReader) -> LocalDecoratorId: ...
def to_json_local_decorator_id(value: LocalDecoratorId) -> Json: ...
def from_json_local_decorator_id(value: Json) -> LocalDecoratorId: ...

__all__ = [
    "DecoratorSegment",
    "encode_decorator_segment",
    "decode_decorator_segment",
    "to_json_decorator_segment",
    "from_json_decorator_segment",
    "DecoratorApplication",
    "encode_decorator_application",
    "decode_decorator_application",
    "to_json_decorator_application",
    "from_json_decorator_application",
    "DecoratorResolution",
    "encode_decorator_resolution",
    "decode_decorator_resolution",
    "to_json_decorator_resolution",
    "from_json_decorator_resolution",
    "DecoratorResolutionLanguageItem",
    "DecoratorResolutionSymbol",
    "DecoratorResolutionUnresolved",
    "DecoratorArgument",
    "encode_decorator_argument",
    "decode_decorator_argument",
    "to_json_decorator_argument",
    "from_json_decorator_argument",
    "LocalDecoratorId",
    "encode_local_decorator_id",
    "decode_local_decorator_id",
    "to_json_local_decorator_id",
    "from_json_local_decorator_id",
]
