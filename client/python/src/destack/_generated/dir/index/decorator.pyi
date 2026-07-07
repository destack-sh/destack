# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.postings
import destack._generated.dir.table.decorator
import destack._generated.dir.tree.node

@dataclass(frozen=True, slots=True)
class DecoratorIndex:
    """Indexed decorator applications."""

    # the decorator applications in stable order
    entries: Sequence[DecoratorEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorIndex: ...

def encode_decorator_index(writer: BinaryWriter, value: DecoratorIndex) -> None: ...
def decode_decorator_index(reader: BinaryReader) -> DecoratorIndex: ...
def to_json_decorator_index(value: DecoratorIndex) -> Json: ...
def from_json_decorator_index(value: Json) -> DecoratorIndex: ...

@dataclass(frozen=True, slots=True)
class DecoratorEntry:
    """One indexed decorator application."""

    # the decorator name when syntactically known
    name: str | None
    # the local decorator application id
    application: destack._generated.dir.table.decorator.LocalDecoratorId
    # the decorator node
    decorator: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator expression target
    expression: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorated target node
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved decorator target
    resolution: destack._generated.dir.table.decorator.DecoratorResolution

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorEntry: ...

def encode_decorator_entry(writer: BinaryWriter, value: DecoratorEntry) -> None: ...
def decode_decorator_entry(reader: BinaryReader) -> DecoratorEntry: ...
def to_json_decorator_entry(value: DecoratorEntry) -> Json: ...
def from_json_decorator_entry(value: Json) -> DecoratorEntry: ...

@dataclass(frozen=True, slots=True)
class DecoratorPostings:
    """Decorator postings by name."""

    # named decorator postings
    names: destack._generated.dir.index.postings.Postings
    # modules that contain unnamed decorators
    unnamed: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorPostings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DecoratorPostings: ...

def encode_decorator_postings(
    writer: BinaryWriter, value: DecoratorPostings
) -> None: ...
def decode_decorator_postings(reader: BinaryReader) -> DecoratorPostings: ...
def to_json_decorator_postings(value: DecoratorPostings) -> Json: ...
def from_json_decorator_postings(value: Json) -> DecoratorPostings: ...

__all__ = [
    "DecoratorIndex",
    "encode_decorator_index",
    "decode_decorator_index",
    "to_json_decorator_index",
    "from_json_decorator_index",
    "DecoratorEntry",
    "encode_decorator_entry",
    "decode_decorator_entry",
    "to_json_decorator_entry",
    "from_json_decorator_entry",
    "DecoratorPostings",
    "encode_decorator_postings",
    "decode_decorator_postings",
    "to_json_decorator_postings",
    "from_json_decorator_postings",
]
