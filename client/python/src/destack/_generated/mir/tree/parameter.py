# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionParameter:
        """Decode one FunctionParameter."""
        return decode_function_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionParameter:
        """Return one FunctionParameter from one JSON value."""
        return from_json_function_parameter(value)


def encode_function_parameter(writer: BinaryWriter, value: FunctionParameter) -> None:
    """Encode one FunctionParameter."""
    destack._generated.mir.tree.value.encode_value(writer, value.value)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_unsigned(len(value.obligations))
    for item_value_obligations_0 in value.obligations:
        encode_borrow_obligation(writer, item_value_obligations_0)


def decode_function_parameter(reader: BinaryReader) -> FunctionParameter:
    """Decode one FunctionParameter."""
    value_ = destack._generated.mir.tree.value.decode_value(reader)
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    obligations = [
        decode_borrow_obligation(reader) for _ in range(reader.read_number())
    ]

    return FunctionParameter(
        value=value_,
        ty=ty,
        obligations=obligations,
    )


def to_json_function_parameter(value: FunctionParameter) -> Json:
    """Return one JSON value for one FunctionParameter."""
    return {
        "value": destack._generated.mir.tree.value.to_json_value(value.value),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "obligations": [
            to_json_borrow_obligation(item_0) for item_0 in value.obligations
        ],
    }


def from_json_function_parameter(value: Json) -> FunctionParameter:
    """Return one FunctionParameter from one JSON value."""
    object_ = json_object(value)

    return FunctionParameter(
        value=destack._generated.mir.tree.value.from_json_value(
            json_field(object_, "value")
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        obligations=[
            from_json_borrow_obligation(item_0)
            for item_0 in json_array(json_field(object_, "obligations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class BorrowObligationSuspensionStable:
    """Borrow source must be stable across suspension."""

    # the lifetime whose source must be stable
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    kind: typing.Literal["suspensionStable"] = "suspensionStable"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_borrow_obligation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_borrow_obligation(self)


"""Borrow source proof required by a callable parameter."""
BorrowObligation: typing.TypeAlias = BorrowObligationSuspensionStable


def encode_borrow_obligation(writer: BinaryWriter, value: BorrowObligation) -> None:
    """Encode one BorrowObligation."""
    if value.kind == "suspensionStable":
        writer.write_unsigned(0)
        destack._generated.mir.tree.lifetime.encode_lifetime(writer, value.lifetime)
    else:
        raise SerdeError("unknown enum variant")


def decode_borrow_obligation(reader: BinaryReader) -> BorrowObligation:
    """Decode one BorrowObligation."""
    variant = reader.read_number()

    if variant == 0:
        lifetime = destack._generated.mir.tree.lifetime.decode_lifetime(reader)

        return BorrowObligationSuspensionStable(
            lifetime=lifetime,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_borrow_obligation(value: BorrowObligation) -> Json:
    """Return one JSON value for one BorrowObligation."""
    if value.kind == "suspensionStable":
        return {
            "kind": "suspensionStable",
            "lifetime": destack._generated.mir.tree.lifetime.to_json_lifetime(
                value.lifetime
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_borrow_obligation(value: Json) -> BorrowObligation:
    """Return one BorrowObligation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "suspensionStable":
        return BorrowObligationSuspensionStable(
            lifetime=destack._generated.mir.tree.lifetime.from_json_lifetime(
                json_field(object_, "lifetime")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class BlockParameter:
    """One block parameter."""

    # the SSA value
    value: destack._generated.mir.tree.value.Value
    # the parameter type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_block_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BlockParameter:
        """Decode one BlockParameter."""
        return decode_block_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_block_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> BlockParameter:
        """Return one BlockParameter from one JSON value."""
        return from_json_block_parameter(value)


def encode_block_parameter(writer: BinaryWriter, value: BlockParameter) -> None:
    """Encode one BlockParameter."""
    destack._generated.mir.tree.value.encode_value(writer, value.value)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)


def decode_block_parameter(reader: BinaryReader) -> BlockParameter:
    """Decode one BlockParameter."""
    value_ = destack._generated.mir.tree.value.decode_value(reader)
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return BlockParameter(
        value=value_,
        ty=ty,
    )


def to_json_block_parameter(value: BlockParameter) -> Json:
    """Return one JSON value for one BlockParameter."""
    return {
        "value": destack._generated.mir.tree.value.to_json_value(value.value),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
    }


def from_json_block_parameter(value: Json) -> BlockParameter:
    """Return one BlockParameter from one JSON value."""
    object_ = json_object(value)

    return BlockParameter(
        value=destack._generated.mir.tree.value.from_json_value(
            json_field(object_, "value")
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
    )


@dataclass(frozen=True, slots=True)
class SignatureParameter:
    """One callable signature parameter."""

    # the parameter type
    ty: destack._generated.mir.tree.node.LocalNodeId
    # borrow obligations callers must satisfy for this parameter
    obligations: Sequence[BorrowObligation]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_signature_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SignatureParameter:
        """Decode one SignatureParameter."""
        return decode_signature_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_signature_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> SignatureParameter:
        """Return one SignatureParameter from one JSON value."""
        return from_json_signature_parameter(value)


def encode_signature_parameter(writer: BinaryWriter, value: SignatureParameter) -> None:
    """Encode one SignatureParameter."""
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)
    writer.write_unsigned(len(value.obligations))
    for item_value_obligations_0 in value.obligations:
        encode_borrow_obligation(writer, item_value_obligations_0)


def decode_signature_parameter(reader: BinaryReader) -> SignatureParameter:
    """Decode one SignatureParameter."""
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)
    obligations = [
        decode_borrow_obligation(reader) for _ in range(reader.read_number())
    ]

    return SignatureParameter(
        ty=ty,
        obligations=obligations,
    )


def to_json_signature_parameter(value: SignatureParameter) -> Json:
    """Return one JSON value for one SignatureParameter."""
    return {
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
        "obligations": [
            to_json_borrow_obligation(item_0) for item_0 in value.obligations
        ],
    }


def from_json_signature_parameter(value: Json) -> SignatureParameter:
    """Return one SignatureParameter from one JSON value."""
    object_ = json_object(value)

    return SignatureParameter(
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
        obligations=[
            from_json_borrow_obligation(item_0)
            for item_0 in json_array(json_field(object_, "obligations"))
        ],
    )


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
