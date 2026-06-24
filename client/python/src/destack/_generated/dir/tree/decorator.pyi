# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node

@dataclass(frozen=True, slots=True)
class Decorator:
    """A decorator attached to an owner node."""

    # the decorator expression
    expression: destack._generated.dir.tree.node.LocalNodeId
    # the decorator position relative to its owner
    position: DecoratorPosition

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Decorator: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Decorator: ...

def encode_decorator(writer: BinaryWriter, value: Decorator) -> None: ...
def decode_decorator(reader: BinaryReader) -> Decorator: ...
def to_json_decorator(value: Decorator) -> Json: ...
def from_json_decorator(value: Json) -> Decorator: ...

"""The position of one decorator relative to its owner."""
DecoratorPosition: typing.TypeAlias = (
    typing.Literal["blockInfix"]
    | typing.Literal["blockPrefix"]
    | typing.Literal["blockPostfix"]
    | typing.Literal["linePrefix"]
    | typing.Literal["linePostfix"]
    | typing.Literal["linePostfixBoundary"]
)

def encode_decorator_position(
    writer: BinaryWriter, value: DecoratorPosition
) -> None: ...
def decode_decorator_position(reader: BinaryReader) -> DecoratorPosition: ...
def to_json_decorator_position(value: DecoratorPosition) -> Json: ...
def from_json_decorator_position(value: Json) -> DecoratorPosition: ...

__all__ = [
    "Decorator",
    "encode_decorator",
    "decode_decorator",
    "to_json_decorator",
    "from_json_decorator",
    "DecoratorPosition",
    "encode_decorator_position",
    "decode_decorator_position",
    "to_json_decorator_position",
    "from_json_decorator_position",
]
