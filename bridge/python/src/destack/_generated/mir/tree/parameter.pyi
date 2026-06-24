# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value

@dataclass(frozen=True, slots=True)
class FunctionParameter:
    """One function entry parameter."""

    # the SSA value
    value: destack._generated.mir.tree.value.Value
    # the parameter type
    ty: destack._generated.mir.tree.node.LocalNodeId
    # borrow obligations callers must satisfy for this parameter
    obligations: Sequence[BorrowObligation]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionParameter: ...

def encode_function_parameter(
    writer: BinaryWriter, value: FunctionParameter
) -> None: ...
def decode_function_parameter(reader: BinaryReader) -> FunctionParameter: ...
def to_json_function_parameter(value: FunctionParameter) -> Json: ...
def from_json_function_parameter(value: Json) -> FunctionParameter: ...

@dataclass(frozen=True, slots=True)
class BorrowObligationSuspensionStable:
    """Borrow source must be stable across suspension."""

    # the lifetime whose source must be stable
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    kind: typing.Literal["suspensionStable"] = "suspensionStable"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Borrow source proof required by a callable parameter."""
BorrowObligation: typing.TypeAlias = BorrowObligationSuspensionStable

def encode_borrow_obligation(writer: BinaryWriter, value: BorrowObligation) -> None: ...
def decode_borrow_obligation(reader: BinaryReader) -> BorrowObligation: ...
def to_json_borrow_obligation(value: BorrowObligation) -> Json: ...
def from_json_borrow_obligation(value: Json) -> BorrowObligation: ...

@dataclass(frozen=True, slots=True)
class BlockParameter:
    """One block parameter."""

    # the SSA value
    value: destack._generated.mir.tree.value.Value
    # the parameter type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BlockParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BlockParameter: ...

def encode_block_parameter(writer: BinaryWriter, value: BlockParameter) -> None: ...
def decode_block_parameter(reader: BinaryReader) -> BlockParameter: ...
def to_json_block_parameter(value: BlockParameter) -> Json: ...
def from_json_block_parameter(value: Json) -> BlockParameter: ...

@dataclass(frozen=True, slots=True)
class SignatureParameter:
    """One callable signature parameter."""

    # the parameter type
    ty: destack._generated.mir.tree.node.LocalNodeId
    # borrow obligations callers must satisfy for this parameter
    obligations: Sequence[BorrowObligation]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SignatureParameter: ...

def encode_signature_parameter(
    writer: BinaryWriter, value: SignatureParameter
) -> None: ...
def decode_signature_parameter(reader: BinaryReader) -> SignatureParameter: ...
def to_json_signature_parameter(value: SignatureParameter) -> Json: ...
def from_json_signature_parameter(value: Json) -> SignatureParameter: ...

__all__ = [
    "FunctionParameter",
    "encode_function_parameter",
    "decode_function_parameter",
    "to_json_function_parameter",
    "from_json_function_parameter",
    "BorrowObligation",
    "encode_borrow_obligation",
    "decode_borrow_obligation",
    "to_json_borrow_obligation",
    "from_json_borrow_obligation",
    "BorrowObligationSuspensionStable",
    "BlockParameter",
    "encode_block_parameter",
    "decode_block_parameter",
    "to_json_block_parameter",
    "from_json_block_parameter",
    "SignatureParameter",
    "encode_signature_parameter",
    "decode_signature_parameter",
    "to_json_signature_parameter",
    "from_json_signature_parameter",
]
