# generated bridge target, do not edit

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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter

"""A concrete MIR floating-point type."""
FloatType: typing.TypeAlias = (
    typing.Literal["float16"]
    | typing.Literal["bfloat16"]
    | typing.Literal["float32"]
    | typing.Literal["float64"]
)


def encode_float_type(writer: BinaryWriter, value: FloatType) -> None:
    """Encode one FloatType."""
    if value == "float16":
        writer.write_unsigned(0)
    elif value == "bfloat16":
        writer.write_unsigned(1)
    elif value == "float32":
        writer.write_unsigned(2)
    elif value == "float64":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_float_type(reader: BinaryReader) -> FloatType:
    """Decode one FloatType."""
    variant = reader.read_number()

    if variant == 0:
        return "float16"
    elif variant == 1:
        return "bfloat16"
    elif variant == 2:
        return "float32"
    elif variant == 3:
        return "float64"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_float_type(value: FloatType) -> Json:
    """Return one JSON value for one FloatType."""
    return value


def from_json_float_type(value: Json) -> FloatType:
    """Return one FloatType from one JSON value."""
    variant = json_string(value)

    if variant == "float16":
        return "float16"
    elif variant == "bfloat16":
        return "bfloat16"
    elif variant == "float32":
        return "float32"
    elif variant == "float64":
        return "float64"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Mutability of a storage binding."""
Mutability: typing.TypeAlias = typing.Literal["immutable"] | typing.Literal["mutable"]


def encode_mutability(writer: BinaryWriter, value: Mutability) -> None:
    """Encode one Mutability."""
    if value == "immutable":
        writer.write_unsigned(0)
    elif value == "mutable":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_mutability(reader: BinaryReader) -> Mutability:
    """Decode one Mutability."""
    variant = reader.read_number()

    if variant == 0:
        return "immutable"
    elif variant == 1:
        return "mutable"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_mutability(value: Mutability) -> Json:
    """Return one JSON value for one Mutability."""
    return value


def from_json_mutability(value: Json) -> Mutability:
    """Return one Mutability from one JSON value."""
    variant = json_string(value)

    if variant == "immutable":
        return "immutable"
    elif variant == "mutable":
        return "mutable"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Kind of reference in MIR."""
ReferenceKind: typing.TypeAlias = (
    typing.Literal["managed"]
    | typing.Literal["unique"]
    | typing.Literal["borrowed"]
    | typing.Literal["raw"]
)


def encode_reference_kind(writer: BinaryWriter, value: ReferenceKind) -> None:
    """Encode one ReferenceKind."""
    if value == "managed":
        writer.write_unsigned(0)
    elif value == "unique":
        writer.write_unsigned(1)
    elif value == "borrowed":
        writer.write_unsigned(2)
    elif value == "raw":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_reference_kind(reader: BinaryReader) -> ReferenceKind:
    """Decode one ReferenceKind."""
    variant = reader.read_number()

    if variant == 0:
        return "managed"
    elif variant == 1:
        return "unique"
    elif variant == 2:
        return "borrowed"
    elif variant == 3:
        return "raw"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_reference_kind(value: ReferenceKind) -> Json:
    """Return one JSON value for one ReferenceKind."""
    return value


def from_json_reference_kind(value: Json) -> ReferenceKind:
    """Return one ReferenceKind from one JSON value."""
    variant = json_string(value)

    if variant == "managed":
        return "managed"
    elif variant == "unique":
        return "unique"
    elif variant == "borrowed":
        return "borrowed"
    elif variant == "raw":
        return "raw"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Space for a reference."""
Space: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["frame"]
    | typing.Literal["static"]
)


def encode_space(writer: BinaryWriter, value: Space) -> None:
    """Encode one Space."""
    if value == "local":
        writer.write_unsigned(0)
    elif value == "shared":
        writer.write_unsigned(1)
    elif value == "frame":
        writer.write_unsigned(2)
    elif value == "static":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_space(reader: BinaryReader) -> Space:
    """Decode one Space."""
    variant = reader.read_number()

    if variant == 0:
        return "local"
    elif variant == 1:
        return "shared"
    elif variant == 2:
        return "frame"
    elif variant == 3:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_space(value: Space) -> Json:
    """Return one JSON value for one Space."""
    return value


def from_json_space(value: Json) -> Space:
    """Return one Space from one JSON value."""
    variant = json_string(value)

    if variant == "local":
        return "local"
    elif variant == "shared":
        return "shared"
    elif variant == "frame":
        return "frame"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Access exposed by a reference-like value."""
Access: typing.TypeAlias = (
    typing.Literal["readonly"] | typing.Literal["mutable"] | typing.Literal["exclusive"]
)


def encode_access(writer: BinaryWriter, value: Access) -> None:
    """Encode one Access."""
    if value == "readonly":
        writer.write_unsigned(0)
    elif value == "mutable":
        writer.write_unsigned(1)
    elif value == "exclusive":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_access(reader: BinaryReader) -> Access:
    """Decode one Access."""
    variant = reader.read_number()

    if variant == 0:
        return "readonly"
    elif variant == 1:
        return "mutable"
    elif variant == 2:
        return "exclusive"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_access(value: Access) -> Json:
    """Return one JSON value for one Access."""
    return value


def from_json_access(value: Json) -> Access:
    """Return one Access from one JSON value."""
    variant = json_string(value)

    if variant == "readonly":
        return "readonly"
    elif variant == "mutable":
        return "mutable"
    elif variant == "exclusive":
        return "exclusive"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Invalid-value niches carried by pointer-like values."""
Nullability: typing.TypeAlias = (
    typing.Literal["none"]
    | typing.Literal["null"]
    | typing.Literal["undefined"]
    | typing.Literal["nullOrUndefined"]
)


def encode_nullability(writer: BinaryWriter, value: Nullability) -> None:
    """Encode one Nullability."""
    if value == "none":
        writer.write_unsigned(0)
    elif value == "null":
        writer.write_unsigned(1)
    elif value == "undefined":
        writer.write_unsigned(2)
    elif value == "nullOrUndefined":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_nullability(reader: BinaryReader) -> Nullability:
    """Decode one Nullability."""
    variant = reader.read_number()

    if variant == 0:
        return "none"
    elif variant == 1:
        return "null"
    elif variant == 2:
        return "undefined"
    elif variant == 3:
        return "nullOrUndefined"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_nullability(value: Nullability) -> Json:
    """Return one JSON value for one Nullability."""
    return value


def from_json_nullability(value: Json) -> Nullability:
    """Return one Nullability from one JSON value."""
    variant = json_string(value)

    if variant == "none":
        return "none"
    elif variant == "null":
        return "null"
    elif variant == "undefined":
        return "undefined"
    elif variant == "nullOrUndefined":
        return "nullOrUndefined"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Copyability of a type."""
Copy: typing.TypeAlias = typing.Literal["yes"] | typing.Literal["no"]


def encode_copy(writer: BinaryWriter, value: Copy) -> None:
    """Encode one Copy."""
    if value == "yes":
        writer.write_unsigned(0)
    elif value == "no":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_copy(reader: BinaryReader) -> Copy:
    """Decode one Copy."""
    variant = reader.read_number()

    if variant == 0:
        return "yes"
    elif variant == 1:
        return "no"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_copy(value: Copy) -> Json:
    """Return one JSON value for one Copy."""
    return value


def from_json_copy(value: Json) -> Copy:
    """Return one Copy from one JSON value."""
    variant = json_string(value)

    if variant == "yes":
        return "yes"
    elif variant == "no":
        return "no"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class VariantCase:
    """One physical tagged sum case."""

    # the tag constant selecting this case
    tag: destack._generated.mir.tree.constant.Constant
    # the logical payload type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_variant_case(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VariantCase:
        """Decode one VariantCase."""
        return decode_variant_case(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_variant_case(self)

    @classmethod
    def from_json(cls, value: Json) -> VariantCase:
        """Return one VariantCase from one JSON value."""
        return from_json_variant_case(value)


def encode_variant_case(writer: BinaryWriter, value: VariantCase) -> None:
    """Encode one VariantCase."""
    destack._generated.mir.tree.constant.encode_constant(writer, value.tag)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)


def decode_variant_case(reader: BinaryReader) -> VariantCase:
    """Decode one VariantCase."""
    tag = destack._generated.mir.tree.constant.decode_constant(reader)
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return VariantCase(
        tag=tag,
        ty=ty,
    )


def to_json_variant_case(value: VariantCase) -> Json:
    """Return one JSON value for one VariantCase."""
    return {
        "tag": destack._generated.mir.tree.constant.to_json_constant(value.tag),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
    }


def from_json_variant_case(value: Json) -> VariantCase:
    """Return one VariantCase from one JSON value."""
    object_ = json_object(value)

    return VariantCase(
        tag=destack._generated.mir.tree.constant.from_json_constant(
            json_field(object_, "tag")
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorDimensionStatic:
    """Compile time static dimension size."""

    static: int
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_dimension(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_dimension(self)


@dataclass(frozen=True, slots=True)
class TensorDimensionSymbol:
    """Symbolic runtime dimension shared across tensors."""

    symbol: str
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_dimension(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_dimension(self)


@dataclass(frozen=True, slots=True)
class TensorDimensionDynamic:
    """Runtime dynamic dimension size."""

    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_dimension(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_dimension(self)


"""Dimension size for tensor shapes and formats."""
TensorDimension: typing.TypeAlias = (
    TensorDimensionStatic | TensorDimensionSymbol | TensorDimensionDynamic
)


def encode_tensor_dimension(writer: BinaryWriter, value: TensorDimension) -> None:
    """Encode one TensorDimension."""
    if value.kind == "static":
        writer.write_unsigned(0)
        writer.write_unsigned(value.static)
    elif value.kind == "symbol":
        writer.write_unsigned(1)
        writer.write_string(value.symbol)
    elif value.kind == "dynamic":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_dimension(reader: BinaryReader) -> TensorDimension:
    """Decode one TensorDimension."""
    variant = reader.read_number()

    if variant == 0:
        static = reader.read_number()

        return TensorDimensionStatic(static=static)
    elif variant == 1:
        symbol = reader.read_string()

        return TensorDimensionSymbol(symbol=symbol)
    elif variant == 2:
        return TensorDimensionDynamic()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_dimension(value: TensorDimension) -> Json:
    """Return one JSON value for one TensorDimension."""
    if value.kind == "static":
        return {
            "kind": "static",
            "static": value.static,
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": value.symbol,
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_dimension(value: Json) -> TensorDimension:
    """Return one TensorDimension from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "static":
        return TensorDimensionStatic(static=json_int(json_field(object_, "static")))
    elif kind == "symbol":
        return TensorDimensionSymbol(symbol=json_string(json_field(object_, "symbol")))
    elif kind == "dynamic":
        return TensorDimensionDynamic()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TensorFormatDense:
    """Dense contiguous format."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_format(self)


"""Format for an owning tensor value."""
TensorFormat: typing.TypeAlias = TensorFormatDense


def encode_tensor_format(writer: BinaryWriter, value: TensorFormat) -> None:
    """Encode one TensorFormat."""
    if value.kind == "dense":
        writer.write_unsigned(0)
        encode_tensor_dimension_order(writer, value.order)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_format(reader: BinaryReader) -> TensorFormat:
    """Decode one TensorFormat."""
    variant = reader.read_number()

    if variant == 0:
        order = decode_tensor_dimension_order(reader)

        return TensorFormatDense(
            order=order,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_format(value: TensorFormat) -> Json:
    """Return one JSON value for one TensorFormat."""
    if value.kind == "dense":
        return {
            "kind": "dense",
            "order": to_json_tensor_dimension_order(value.order),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_format(value: Json) -> TensorFormat:
    """Return one TensorFormat from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dense":
        return TensorFormatDense(
            order=from_json_tensor_dimension_order(json_field(object_, "order")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Dimension order for dense tensor storage."""
TensorDimensionOrder: typing.TypeAlias = (
    typing.Literal["rowMajor"] | typing.Literal["columnMajor"]
)


def encode_tensor_dimension_order(
    writer: BinaryWriter, value: TensorDimensionOrder
) -> None:
    """Encode one TensorDimensionOrder."""
    if value == "rowMajor":
        writer.write_unsigned(0)
    elif value == "columnMajor":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_dimension_order(reader: BinaryReader) -> TensorDimensionOrder:
    """Decode one TensorDimensionOrder."""
    variant = reader.read_number()

    if variant == 0:
        return "rowMajor"
    elif variant == 1:
        return "columnMajor"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_dimension_order(value: TensorDimensionOrder) -> Json:
    """Return one JSON value for one TensorDimensionOrder."""
    return value


def from_json_tensor_dimension_order(value: Json) -> TensorDimensionOrder:
    """Return one TensorDimensionOrder from one JSON value."""
    variant = json_string(value)

    if variant == "rowMajor":
        return "rowMajor"
    elif variant == "columnMajor":
        return "columnMajor"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TensorShardingUnsharded:
    """Tensor storage is not partitioned across a mesh."""

    kind: typing.Literal["unsharded"] = "unsharded"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding(self)


@dataclass(frozen=True, slots=True)
class TensorShardingSharding:
    """Tensor storage is mapped across a mesh axis by axis."""

    # the per-axis placement descriptors
    axes: Sequence[TensorShardingAxis]
    kind: typing.Literal["sharding"] = "sharding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding(self)


"""Placement descriptor for tensor storage."""
TensorSharding: typing.TypeAlias = TensorShardingUnsharded | TensorShardingSharding


def encode_tensor_sharding(writer: BinaryWriter, value: TensorSharding) -> None:
    """Encode one TensorSharding."""
    if value.kind == "unsharded":
        writer.write_unsigned(0)
    elif value.kind == "sharding":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.axes))
        for item_value_axes_0 in value.axes:
            encode_tensor_sharding_axis(writer, item_value_axes_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_sharding(reader: BinaryReader) -> TensorSharding:
    """Decode one TensorSharding."""
    variant = reader.read_number()

    if variant == 0:
        return TensorShardingUnsharded()
    elif variant == 1:
        axes = [
            decode_tensor_sharding_axis(reader) for _ in range(reader.read_number())
        ]

        return TensorShardingSharding(
            axes=axes,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_sharding(value: TensorSharding) -> Json:
    """Return one JSON value for one TensorSharding."""
    if value.kind == "unsharded":
        return {
            "kind": "unsharded",
        }
    elif value.kind == "sharding":
        return {
            "kind": "sharding",
            "axes": [to_json_tensor_sharding_axis(item_0) for item_0 in value.axes],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_sharding(value: Json) -> TensorSharding:
    """Return one TensorSharding from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unsharded":
        return TensorShardingUnsharded()
    elif kind == "sharding":
        return TensorShardingSharding(
            axes=[
                from_json_tensor_sharding_axis(item_0)
                for item_0 in json_array(json_field(object_, "axes"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TensorShardingAxisShard:
    """Split one tensor axis across one mesh axis."""

    # the tensor axis being split
    axis: int
    kind: typing.Literal["shard"] = "shard"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


@dataclass(frozen=True, slots=True)
class TensorShardingAxisReplicate:
    """Replicate values across one mesh axis."""

    kind: typing.Literal["replicate"] = "replicate"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


@dataclass(frozen=True, slots=True)
class TensorShardingAxisPartial:
    """Store partial results across one mesh axis."""

    # the reduction used to combine partial values
    reduction: TensorReduction
    kind: typing.Literal["partial"] = "partial"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_sharding_axis(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_sharding_axis(self)


"""Per-axis placement descriptor for a sharded tensor."""
TensorShardingAxis: typing.TypeAlias = (
    TensorShardingAxisShard | TensorShardingAxisReplicate | TensorShardingAxisPartial
)


def encode_tensor_sharding_axis(
    writer: BinaryWriter, value: TensorShardingAxis
) -> None:
    """Encode one TensorShardingAxis."""
    if value.kind == "shard":
        writer.write_unsigned(0)
        writer.write_signed(value.axis)
    elif value.kind == "replicate":
        writer.write_unsigned(1)
    elif value.kind == "partial":
        writer.write_unsigned(2)
        encode_tensor_reduction(writer, value.reduction)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_sharding_axis(reader: BinaryReader) -> TensorShardingAxis:
    """Decode one TensorShardingAxis."""
    variant = reader.read_number()

    if variant == 0:
        axis = reader.read_signed_number()

        return TensorShardingAxisShard(
            axis=axis,
        )
    elif variant == 1:
        return TensorShardingAxisReplicate()
    elif variant == 2:
        reduction = decode_tensor_reduction(reader)

        return TensorShardingAxisPartial(
            reduction=reduction,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_sharding_axis(value: TensorShardingAxis) -> Json:
    """Return one JSON value for one TensorShardingAxis."""
    if value.kind == "shard":
        return {
            "kind": "shard",
            "axis": value.axis,
        }
    elif value.kind == "replicate":
        return {
            "kind": "replicate",
        }
    elif value.kind == "partial":
        return {
            "kind": "partial",
            "reduction": to_json_tensor_reduction(value.reduction),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_sharding_axis(value: Json) -> TensorShardingAxis:
    """Return one TensorShardingAxis from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "shard":
        return TensorShardingAxisShard(
            axis=json_int(json_field(object_, "axis")),
        )
    elif kind == "replicate":
        return TensorShardingAxisReplicate()
    elif kind == "partial":
        return TensorShardingAxisPartial(
            reduction=from_json_tensor_reduction(json_field(object_, "reduction")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Reduction used when partial tensor shards are combined."""
TensorReduction: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["multiply"]
    | typing.Literal["minimum"]
    | typing.Literal["maximum"]
    | typing.Literal["and"]
    | typing.Literal["or"]
)


def encode_tensor_reduction(writer: BinaryWriter, value: TensorReduction) -> None:
    """Encode one TensorReduction."""
    if value == "add":
        writer.write_unsigned(0)
    elif value == "multiply":
        writer.write_unsigned(1)
    elif value == "minimum":
        writer.write_unsigned(2)
    elif value == "maximum":
        writer.write_unsigned(3)
    elif value == "and":
        writer.write_unsigned(4)
    elif value == "or":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_reduction(reader: BinaryReader) -> TensorReduction:
    """Decode one TensorReduction."""
    variant = reader.read_number()

    if variant == 0:
        return "add"
    elif variant == 1:
        return "multiply"
    elif variant == 2:
        return "minimum"
    elif variant == 3:
        return "maximum"
    elif variant == 4:
        return "and"
    elif variant == 5:
        return "or"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_reduction(value: TensorReduction) -> Json:
    """Return one JSON value for one TensorReduction."""
    return value


def from_json_tensor_reduction(value: Json) -> TensorReduction:
    """Return one TensorReduction from one JSON value."""
    variant = json_string(value)

    if variant == "add":
        return "add"
    elif variant == "multiply":
        return "multiply"
    elif variant == "minimum":
        return "minimum"
    elif variant == "maximum":
        return "maximum"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TensorViewFormatDense:
    """Dense contiguous view."""

    # the dimension order
    order: TensorDimensionOrder
    kind: typing.Literal["dense"] = "dense"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_format(self)


@dataclass(frozen=True, slots=True)
class TensorViewFormatStrided:
    """Explicit strided view."""

    kind: typing.Literal["strided"] = "strided"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_format(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_format(self)


"""Format descriptor for a tensor view."""
TensorViewFormat: typing.TypeAlias = TensorViewFormatDense | TensorViewFormatStrided


def encode_tensor_view_format(writer: BinaryWriter, value: TensorViewFormat) -> None:
    """Encode one TensorViewFormat."""
    if value.kind == "dense":
        writer.write_unsigned(0)
        encode_tensor_dimension_order(writer, value.order)
    elif value.kind == "strided":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tensor_view_format(reader: BinaryReader) -> TensorViewFormat:
    """Decode one TensorViewFormat."""
    variant = reader.read_number()

    if variant == 0:
        order = decode_tensor_dimension_order(reader)

        return TensorViewFormatDense(
            order=order,
        )
    elif variant == 1:
        return TensorViewFormatStrided()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tensor_view_format(value: TensorViewFormat) -> Json:
    """Return one JSON value for one TensorViewFormat."""
    if value.kind == "dense":
        return {
            "kind": "dense",
            "order": to_json_tensor_dimension_order(value.order),
        }
    elif value.kind == "strided":
        return {
            "kind": "strided",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_tensor_view_format(value: Json) -> TensorViewFormat:
    """Return one TensorViewFormat from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "dense":
        return TensorViewFormatDense(
            order=from_json_tensor_dimension_order(json_field(object_, "order")),
        )
    elif kind == "strided":
        return TensorViewFormatStrided()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TypeAlias:
    """A named type alias in MIR text format."""

    # alias name (without the leading `@`)
    name: destack._generated.core.string.StringId
    # lifetime parameters in type-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # the aliased type
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_alias(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeAlias:
        """Decode one TypeAlias."""
        return decode_type_alias(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_alias(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeAlias:
        """Return one TypeAlias from one JSON value."""
        return from_json_type_alias(value)


def encode_type_alias(writer: BinaryWriter, value: TypeAlias) -> None:
    """Encode one TypeAlias."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    writer.write_unsigned(len(value.lifetimes))
    for item_value_lifetimes_0 in value.lifetimes:
        destack._generated.mir.tree.lifetime.encode_lifetime_parameter(
            writer, item_value_lifetimes_0
        )
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)


def decode_type_alias(reader: BinaryReader) -> TypeAlias:
    """Decode one TypeAlias."""
    name = destack._generated.core.string.decode_string_id(reader)
    lifetimes = [
        destack._generated.mir.tree.lifetime.decode_lifetime_parameter(reader)
        for _ in range(reader.read_number())
    ]
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return TypeAlias(
        name=name,
        lifetimes=lifetimes,
        ty=ty,
    )


def to_json_type_alias(value: TypeAlias) -> Json:
    """Return one JSON value for one TypeAlias."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "lifetimes": [
            destack._generated.mir.tree.lifetime.to_json_lifetime_parameter(item_0)
            for item_0 in value.lifetimes
        ],
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
    }


def from_json_type_alias(value: Json) -> TypeAlias:
    """Return one TypeAlias from one JSON value."""
    object_ = json_object(value)

    return TypeAlias(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        lifetimes=[
            destack._generated.mir.tree.lifetime.from_json_lifetime_parameter(item_0)
            for item_0 in json_array(json_field(object_, "lifetimes"))
        ],
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
    )


@dataclass(frozen=True, slots=True)
class Field:
    """A field in a struct type."""

    # name (optional)
    name: destack._generated.core.string.StringId | None
    # type of the field
    ty: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Field:
        """Decode one Field."""
        return decode_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_field(self)

    @classmethod
    def from_json(cls, value: Json) -> Field:
        """Return one Field from one JSON value."""
        return from_json_field(value)


def encode_field(writer: BinaryWriter, value: Field) -> None:
    """Encode one Field."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.ty)


def decode_field(reader: BinaryReader) -> Field:
    """Decode one Field."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = destack._generated.mir.tree.node.decode_local_node_id(reader)

    return Field(
        name=name,
        ty=ty,
    )


def to_json_field(value: Field) -> Json:
    """Return one JSON value for one Field."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        "ty": destack._generated.mir.tree.node.to_json_local_node_id(value.ty),
    }


def from_json_field(value: Json) -> Field:
    """Return one Field from one JSON value."""
    object_ = json_object(value)

    return Field(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "ty")
        ),
    )


@dataclass(frozen=True, slots=True)
class TypeError:
    """Invalid type produced while recovering malformed MIR text."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeVoid:
    """Void / unit type (no value)."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeBoolean:
    """Boolean (1 bit logical, typically 1 byte)."""

    kind: typing.Literal["boolean"] = "boolean"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeInt:
    """Integer with explicit width and signedness."""

    width: int
    is_signed: bool
    kind: typing.Literal["int"] = "int"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeIsize:
    """Pointer-sized signed integer."""

    kind: typing.Literal["isize"] = "isize"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeUsize:
    """Pointer-sized unsigned integer."""

    kind: typing.Literal["usize"] = "usize"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFloat:
    """Floating point with concrete representation."""

    float: FloatType
    kind: typing.Literal["float"] = "float"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTypeDescriptor:
    """Runtime type descriptor handle."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTypeId:
    """Compact runtime type identity token."""

    kind: typing.Literal["typeId"] = "typeId"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeAtomic:
    """Atomic storage cell for one value type."""

    # the stored value type
    value: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["atomic"] = "atomic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeDynamic:
    """Runtime-erased value satisfying one dynamic constraint."""

    # the lowered dynamic constraint type
    constraint: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeWithLifetimes:
    """Type use with applied lifetime arguments."""

    # the type being applied
    base: destack._generated.mir.tree.node.LocalNodeId
    # the applied lifetime arguments
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.Lifetime]
    kind: typing.Literal["withLifetimes"] = "withLifetimes"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeReference:
    """Reference with explicit kind and access."""

    # the reference kind (managed, unique, borrowed, raw)
    kind_value: ReferenceKind
    # lifetime roots for borrowed references
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the space for this reference
    space: Space
    # the access exposed through this reference
    access: Access
    # the referenced type
    pointee: destack._generated.mir.tree.node.LocalNodeId
    # the nullish values allowed by this reference
    nullability: Nullability
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeSlice:
    """Slice into memory, a repeated element with explicit kind and access."""

    # the reference kind of the slice base
    kind_value: ReferenceKind
    # lifetime roots for borrowed slices
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the element type of the slice
    element: destack._generated.mir.tree.node.LocalNodeId
    # the space of the slice base
    space: Space
    # the element access exposed by the slice
    access: Access
    # the nullish values allowed by this slice descriptor
    nullability: Nullability
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeUninit:
    """Linear token for one uninitialized allocation."""

    # the value under construction
    value: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["uninit"] = "uninit"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFixedArray:
    """Fixed array: `[T; N]`."""

    # the element type of the array
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of elements in the array
    length: int
    # copy of this fixed array type
    copy: Copy
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTuple:
    """Tuple: `(T1, T2, ...)`."""

    # the element types of the tuple
    elements: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # copy of this tuple type
    copy: Copy
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeStruct:
    """Struct."""

    # the fields of the struct
    fields: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # copy of this struct type
    copy: Copy
    kind: typing.Literal["struct"] = "struct"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeNewtype:
    """Nominal newtype wrapping an inner type."""

    # the wrapped inner type
    inner: destack._generated.mir.tree.node.LocalNodeId
    # copy of this newtype
    copy: Copy
    kind: typing.Literal["newtype"] = "newtype"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeVariant:
    """Physical tagged sum value."""

    # the tag value type
    tag: destack._generated.mir.tree.node.LocalNodeId
    # the physical payload storage type
    storage: destack._generated.mir.tree.node.LocalNodeId
    # the cases keyed by tag value
    cases: Sequence[VariantCase]
    # copy of this variant type
    copy: Copy
    kind: typing.Literal["variant"] = "variant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeVector:
    """Fixed-width vector value."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the number of lanes
    lanes: int
    # copy of this vector type
    copy: Copy
    kind: typing.Literal["vector"] = "vector"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTensor:
    """Ranked tensor value with static or dynamic shape."""

    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the static shape
    shape: Sequence[TensorDimension]
    # the tensor format
    format: TensorFormat
    # the tensor placement
    sharding: TensorSharding
    # copy of this tensor type
    copy: Copy
    kind: typing.Literal["tensor"] = "tensor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTensorView:
    """Reference-like view into tensor-shaped memory."""

    # the reference kind (managed, unique, borrowed, raw)
    kind_value: ReferenceKind
    # lifetime roots for borrowed tensor views
    lifetime: destack._generated.mir.tree.lifetime.Lifetime
    # the space for this view
    space: Space
    # the access exposed through this view
    access: Access
    # the element type
    element: destack._generated.mir.tree.node.LocalNodeId
    # the static shape
    shape: Sequence[TensorDimension]
    # the tensor view format
    format: TensorViewFormat
    # the tensor placement
    sharding: TensorSharding
    # the nullish values allowed by this view descriptor
    nullability: Nullability
    kind: typing.Literal["tensorView"] = "tensorView"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunctionSignature:
    """Bare function signature."""

    # lifetime parameters in signature-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # the parameters of the function
    parameters: Sequence[destack._generated.mir.tree.parameter.SignatureParameter]
    # the result type of the function
    result: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunction:
    """Function value type."""

    # the bare function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the captured environment reference
    environment: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunctionPointer:
    """Function pointer type."""

    # the bare function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


"""Concrete type in MIR (post-monomorphization)."""
Type: typing.TypeAlias = (
    TypeError
    | TypeVoid
    | TypeBoolean
    | TypeInt
    | TypeIsize
    | TypeUsize
    | TypeFloat
    | TypeTypeDescriptor
    | TypeTypeId
    | TypeAtomic
    | TypeDynamic
    | TypeWithLifetimes
    | TypeReference
    | TypeSlice
    | TypeUninit
    | TypeFixedArray
    | TypeTuple
    | TypeStruct
    | TypeNewtype
    | TypeVariant
    | TypeVector
    | TypeTensor
    | TypeTensorView
    | TypeFunctionSignature
    | TypeFunction
    | TypeFunctionPointer
)


def encode_type(writer: BinaryWriter, value: Type) -> None:
    """Encode one Type."""
    if value.kind == "error":
        writer.write_unsigned(0)
    elif value.kind == "void":
        writer.write_unsigned(1)
    elif value.kind == "boolean":
        writer.write_unsigned(2)
    elif value.kind == "int":
        writer.write_unsigned(3)
        writer.write_unsigned(value.width)
        writer.write_bool(value.is_signed)
    elif value.kind == "isize":
        writer.write_unsigned(4)
    elif value.kind == "usize":
        writer.write_unsigned(5)
    elif value.kind == "float":
        writer.write_unsigned(6)
        encode_float_type(writer, value.float)
    elif value.kind == "typeDescriptor":
        writer.write_unsigned(7)
    elif value.kind == "typeId":
        writer.write_unsigned(8)
    elif value.kind == "atomic":
        writer.write_unsigned(9)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "dynamic":
        writer.write_unsigned(10)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.constraint)
    elif value.kind == "withLifetimes":
        writer.write_unsigned(11)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.base)
        writer.write_unsigned(len(value.lifetimes))
        for item_value_lifetimes_0 in value.lifetimes:
            destack._generated.mir.tree.lifetime.encode_lifetime(
                writer, item_value_lifetimes_0
            )
    elif value.kind == "reference":
        writer.write_unsigned(12)
        encode_reference_kind(writer, value.kind_value)
        destack._generated.mir.tree.lifetime.encode_lifetime(writer, value.lifetime)
        encode_space(writer, value.space)
        encode_access(writer, value.access)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.pointee)
        encode_nullability(writer, value.nullability)
    elif value.kind == "slice":
        writer.write_unsigned(13)
        encode_reference_kind(writer, value.kind_value)
        destack._generated.mir.tree.lifetime.encode_lifetime(writer, value.lifetime)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        encode_space(writer, value.space)
        encode_access(writer, value.access)
        encode_nullability(writer, value.nullability)
    elif value.kind == "uninit":
        writer.write_unsigned(14)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "fixedArray":
        writer.write_unsigned(15)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        writer.write_unsigned(value.length)
        encode_copy(writer, value.copy)
    elif value.kind == "tuple":
        writer.write_unsigned(16)
        writer.write_unsigned(len(value.elements))
        for item_value_elements_0 in value.elements:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, item_value_elements_0
            )
        encode_copy(writer, value.copy)
    elif value.kind == "struct":
        writer.write_unsigned(17)
        writer.write_unsigned(len(value.fields))
        for item_value_fields_0 in value.fields:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, item_value_fields_0
            )
        encode_copy(writer, value.copy)
    elif value.kind == "newtype":
        writer.write_unsigned(18)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.inner)
        encode_copy(writer, value.copy)
    elif value.kind == "variant":
        writer.write_unsigned(19)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.tag)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.storage)
        writer.write_unsigned(len(value.cases))
        for item_value_cases_0 in value.cases:
            encode_variant_case(writer, item_value_cases_0)
        encode_copy(writer, value.copy)
    elif value.kind == "vector":
        writer.write_unsigned(20)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        writer.write_unsigned(value.lanes)
        encode_copy(writer, value.copy)
    elif value.kind == "tensor":
        writer.write_unsigned(21)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        writer.write_unsigned(len(value.shape))
        for item_value_shape_0 in value.shape:
            encode_tensor_dimension(writer, item_value_shape_0)
        encode_tensor_format(writer, value.format)
        encode_tensor_sharding(writer, value.sharding)
        encode_copy(writer, value.copy)
    elif value.kind == "tensorView":
        writer.write_unsigned(22)
        encode_reference_kind(writer, value.kind_value)
        destack._generated.mir.tree.lifetime.encode_lifetime(writer, value.lifetime)
        encode_space(writer, value.space)
        encode_access(writer, value.access)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.element)
        writer.write_unsigned(len(value.shape))
        for item_value_shape_0 in value.shape:
            encode_tensor_dimension(writer, item_value_shape_0)
        encode_tensor_view_format(writer, value.format)
        encode_tensor_sharding(writer, value.sharding)
        encode_nullability(writer, value.nullability)
    elif value.kind == "functionSignature":
        writer.write_unsigned(23)
        writer.write_unsigned(len(value.lifetimes))
        for item_value_lifetimes_0 in value.lifetimes:
            destack._generated.mir.tree.lifetime.encode_lifetime_parameter(
                writer, item_value_lifetimes_0
            )
        writer.write_unsigned(len(value.parameters))
        for item_value_parameters_0 in value.parameters:
            destack._generated.mir.tree.parameter.encode_signature_parameter(
                writer, item_value_parameters_0
            )
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.result)
    elif value.kind == "function":
        writer.write_unsigned(24)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.environment)
    elif value.kind == "functionPointer":
        writer.write_unsigned(25)
        destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    else:
        raise SerdeError("unknown enum variant")


def decode_type(reader: BinaryReader) -> Type:
    """Decode one Type."""
    variant = reader.read_number()

    if variant == 0:
        return TypeError()
    elif variant == 1:
        return TypeVoid()
    elif variant == 2:
        return TypeBoolean()
    elif variant == 3:
        width = reader.read_number()
        is_signed = reader.read_bool()

        return TypeInt(
            width=width,
            is_signed=is_signed,
        )
    elif variant == 4:
        return TypeIsize()
    elif variant == 5:
        return TypeUsize()
    elif variant == 6:
        float = decode_float_type(reader)

        return TypeFloat(float=float)
    elif variant == 7:
        return TypeTypeDescriptor()
    elif variant == 8:
        return TypeTypeId()
    elif variant == 9:
        value_ = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeAtomic(
            value=value_,
        )
    elif variant == 10:
        constraint = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeDynamic(
            constraint=constraint,
        )
    elif variant == 11:
        base = destack._generated.mir.tree.node.decode_local_node_id(reader)
        lifetimes = [
            destack._generated.mir.tree.lifetime.decode_lifetime(reader)
            for _ in range(reader.read_number())
        ]

        return TypeWithLifetimes(
            base=base,
            lifetimes=lifetimes,
        )
    elif variant == 12:
        kind_value = decode_reference_kind(reader)
        lifetime = destack._generated.mir.tree.lifetime.decode_lifetime(reader)
        space = decode_space(reader)
        access = decode_access(reader)
        pointee = destack._generated.mir.tree.node.decode_local_node_id(reader)
        nullability = decode_nullability(reader)

        return TypeReference(
            kind_value=kind_value,
            lifetime=lifetime,
            space=space,
            access=access,
            pointee=pointee,
            nullability=nullability,
        )
    elif variant == 13:
        kind_value = decode_reference_kind(reader)
        lifetime = destack._generated.mir.tree.lifetime.decode_lifetime(reader)
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        space = decode_space(reader)
        access = decode_access(reader)
        nullability = decode_nullability(reader)

        return TypeSlice(
            kind_value=kind_value,
            lifetime=lifetime,
            element=element,
            space=space,
            access=access,
            nullability=nullability,
        )
    elif variant == 14:
        value_ = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeUninit(
            value=value_,
        )
    elif variant == 15:
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        length = reader.read_number()
        copy = decode_copy(reader)

        return TypeFixedArray(
            element=element,
            length=length,
            copy=copy,
        )
    elif variant == 16:
        elements = [
            destack._generated.mir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        copy = decode_copy(reader)

        return TypeTuple(
            elements=elements,
            copy=copy,
        )
    elif variant == 17:
        fields = [
            destack._generated.mir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        copy = decode_copy(reader)

        return TypeStruct(
            fields=fields,
            copy=copy,
        )
    elif variant == 18:
        inner = destack._generated.mir.tree.node.decode_local_node_id(reader)
        copy = decode_copy(reader)

        return TypeNewtype(
            inner=inner,
            copy=copy,
        )
    elif variant == 19:
        tag = destack._generated.mir.tree.node.decode_local_node_id(reader)
        storage = destack._generated.mir.tree.node.decode_local_node_id(reader)
        cases = [decode_variant_case(reader) for _ in range(reader.read_number())]
        copy = decode_copy(reader)

        return TypeVariant(
            tag=tag,
            storage=storage,
            cases=cases,
            copy=copy,
        )
    elif variant == 20:
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        lanes = reader.read_number()
        copy = decode_copy(reader)

        return TypeVector(
            element=element,
            lanes=lanes,
            copy=copy,
        )
    elif variant == 21:
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        shape = [decode_tensor_dimension(reader) for _ in range(reader.read_number())]
        format = decode_tensor_format(reader)
        sharding = decode_tensor_sharding(reader)
        copy = decode_copy(reader)

        return TypeTensor(
            element=element,
            shape=shape,
            format=format,
            sharding=sharding,
            copy=copy,
        )
    elif variant == 22:
        kind_value = decode_reference_kind(reader)
        lifetime = destack._generated.mir.tree.lifetime.decode_lifetime(reader)
        space = decode_space(reader)
        access = decode_access(reader)
        element = destack._generated.mir.tree.node.decode_local_node_id(reader)
        shape = [decode_tensor_dimension(reader) for _ in range(reader.read_number())]
        format = decode_tensor_view_format(reader)
        sharding = decode_tensor_sharding(reader)
        nullability = decode_nullability(reader)

        return TypeTensorView(
            kind_value=kind_value,
            lifetime=lifetime,
            space=space,
            access=access,
            element=element,
            shape=shape,
            format=format,
            sharding=sharding,
            nullability=nullability,
        )
    elif variant == 23:
        lifetimes = [
            destack._generated.mir.tree.lifetime.decode_lifetime_parameter(reader)
            for _ in range(reader.read_number())
        ]
        parameters = [
            destack._generated.mir.tree.parameter.decode_signature_parameter(reader)
            for _ in range(reader.read_number())
        ]
        result = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeFunctionSignature(
            lifetimes=lifetimes,
            parameters=parameters,
            result=result,
        )
    elif variant == 24:
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)
        environment = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeFunction(
            signature=signature,
            environment=environment,
        )
    elif variant == 25:
        signature = destack._generated.mir.tree.node.decode_local_node_id(reader)

        return TypeFunctionPointer(
            signature=signature,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type(value: Type) -> Json:
    """Return one JSON value for one Type."""
    if value.kind == "error":
        return {
            "kind": "error",
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "boolean":
        return {
            "kind": "boolean",
        }
    elif value.kind == "int":
        return {
            "kind": "int",
            "width": value.width,
            "isSigned": value.is_signed,
        }
    elif value.kind == "isize":
        return {
            "kind": "isize",
        }
    elif value.kind == "usize":
        return {
            "kind": "usize",
        }
    elif value.kind == "float":
        return {
            "kind": "float",
            "float": to_json_float_type(value.float),
        }
    elif value.kind == "typeDescriptor":
        return {
            "kind": "typeDescriptor",
        }
    elif value.kind == "typeId":
        return {
            "kind": "typeId",
        }
    elif value.kind == "atomic":
        return {
            "kind": "atomic",
            "value": destack._generated.mir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
            "constraint": destack._generated.mir.tree.node.to_json_local_node_id(
                value.constraint
            ),
        }
    elif value.kind == "withLifetimes":
        return {
            "kind": "withLifetimes",
            "base": destack._generated.mir.tree.node.to_json_local_node_id(value.base),
            "lifetimes": [
                destack._generated.mir.tree.lifetime.to_json_lifetime(item_0)
                for item_0 in value.lifetimes
            ],
        }
    elif value.kind == "reference":
        return {
            "kind": "reference",
            "kind": to_json_reference_kind(value.kind_value),
            "lifetime": destack._generated.mir.tree.lifetime.to_json_lifetime(
                value.lifetime
            ),
            "space": to_json_space(value.space),
            "access": to_json_access(value.access),
            "pointee": destack._generated.mir.tree.node.to_json_local_node_id(
                value.pointee
            ),
            "nullability": to_json_nullability(value.nullability),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
            "kind": to_json_reference_kind(value.kind_value),
            "lifetime": destack._generated.mir.tree.lifetime.to_json_lifetime(
                value.lifetime
            ),
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "space": to_json_space(value.space),
            "access": to_json_access(value.access),
            "nullability": to_json_nullability(value.nullability),
        }
    elif value.kind == "uninit":
        return {
            "kind": "uninit",
            "value": destack._generated.mir.tree.node.to_json_local_node_id(
                value.value
            ),
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "length": value.length,
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "elements": [
                destack._generated.mir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.elements
            ],
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "struct":
        return {
            "kind": "struct",
            "fields": [
                destack._generated.mir.tree.node.to_json_local_node_id(item_0)
                for item_0 in value.fields
            ],
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "newtype":
        return {
            "kind": "newtype",
            "inner": destack._generated.mir.tree.node.to_json_local_node_id(
                value.inner
            ),
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "variant":
        return {
            "kind": "variant",
            "tag": destack._generated.mir.tree.node.to_json_local_node_id(value.tag),
            "storage": destack._generated.mir.tree.node.to_json_local_node_id(
                value.storage
            ),
            "cases": [to_json_variant_case(item_0) for item_0 in value.cases],
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "vector":
        return {
            "kind": "vector",
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "lanes": value.lanes,
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "tensor":
        return {
            "kind": "tensor",
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "shape": [to_json_tensor_dimension(item_0) for item_0 in value.shape],
            "format": to_json_tensor_format(value.format),
            "sharding": to_json_tensor_sharding(value.sharding),
            "copy": to_json_copy(value.copy),
        }
    elif value.kind == "tensorView":
        return {
            "kind": "tensorView",
            "kind": to_json_reference_kind(value.kind_value),
            "lifetime": destack._generated.mir.tree.lifetime.to_json_lifetime(
                value.lifetime
            ),
            "space": to_json_space(value.space),
            "access": to_json_access(value.access),
            "element": destack._generated.mir.tree.node.to_json_local_node_id(
                value.element
            ),
            "shape": [to_json_tensor_dimension(item_0) for item_0 in value.shape],
            "format": to_json_tensor_view_format(value.format),
            "sharding": to_json_tensor_sharding(value.sharding),
            "nullability": to_json_nullability(value.nullability),
        }
    elif value.kind == "functionSignature":
        return {
            "kind": "functionSignature",
            "lifetimes": [
                destack._generated.mir.tree.lifetime.to_json_lifetime_parameter(item_0)
                for item_0 in value.lifetimes
            ],
            "parameters": [
                destack._generated.mir.tree.parameter.to_json_signature_parameter(
                    item_0
                )
                for item_0 in value.parameters
            ],
            "result": destack._generated.mir.tree.node.to_json_local_node_id(
                value.result
            ),
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
            "environment": destack._generated.mir.tree.node.to_json_local_node_id(
                value.environment
            ),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "signature": destack._generated.mir.tree.node.to_json_local_node_id(
                value.signature
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type(value: Json) -> Type:
    """Return one Type from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "error":
        return TypeError()
    elif kind == "void":
        return TypeVoid()
    elif kind == "boolean":
        return TypeBoolean()
    elif kind == "int":
        return TypeInt(
            width=json_int(json_field(object_, "width")),
            is_signed=json_bool(json_field(object_, "isSigned")),
        )
    elif kind == "isize":
        return TypeIsize()
    elif kind == "usize":
        return TypeUsize()
    elif kind == "float":
        return TypeFloat(float=from_json_float_type(json_field(object_, "float")))
    elif kind == "typeDescriptor":
        return TypeTypeDescriptor()
    elif kind == "typeId":
        return TypeTypeId()
    elif kind == "atomic":
        return TypeAtomic(
            value=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "dynamic":
        return TypeDynamic(
            constraint=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "constraint")
            ),
        )
    elif kind == "withLifetimes":
        return TypeWithLifetimes(
            base=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "base")
            ),
            lifetimes=[
                destack._generated.mir.tree.lifetime.from_json_lifetime(item_0)
                for item_0 in json_array(json_field(object_, "lifetimes"))
            ],
        )
    elif kind == "reference":
        return TypeReference(
            kind_value=from_json_reference_kind(json_field(object_, "kind")),
            lifetime=destack._generated.mir.tree.lifetime.from_json_lifetime(
                json_field(object_, "lifetime")
            ),
            space=from_json_space(json_field(object_, "space")),
            access=from_json_access(json_field(object_, "access")),
            pointee=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "pointee")
            ),
            nullability=from_json_nullability(json_field(object_, "nullability")),
        )
    elif kind == "slice":
        return TypeSlice(
            kind_value=from_json_reference_kind(json_field(object_, "kind")),
            lifetime=destack._generated.mir.tree.lifetime.from_json_lifetime(
                json_field(object_, "lifetime")
            ),
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            space=from_json_space(json_field(object_, "space")),
            access=from_json_access(json_field(object_, "access")),
            nullability=from_json_nullability(json_field(object_, "nullability")),
        )
    elif kind == "uninit":
        return TypeUninit(
            value=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "value")
            ),
        )
    elif kind == "fixedArray":
        return TypeFixedArray(
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            length=json_int(json_field(object_, "length")),
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "tuple":
        return TypeTuple(
            elements=[
                destack._generated.mir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "elements"))
            ],
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "struct":
        return TypeStruct(
            fields=[
                destack._generated.mir.tree.node.from_json_local_node_id(item_0)
                for item_0 in json_array(json_field(object_, "fields"))
            ],
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "newtype":
        return TypeNewtype(
            inner=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "inner")
            ),
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "variant":
        return TypeVariant(
            tag=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "tag")
            ),
            storage=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "storage")
            ),
            cases=[
                from_json_variant_case(item_0)
                for item_0 in json_array(json_field(object_, "cases"))
            ],
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "vector":
        return TypeVector(
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            lanes=json_int(json_field(object_, "lanes")),
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "tensor":
        return TypeTensor(
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            shape=[
                from_json_tensor_dimension(item_0)
                for item_0 in json_array(json_field(object_, "shape"))
            ],
            format=from_json_tensor_format(json_field(object_, "format")),
            sharding=from_json_tensor_sharding(json_field(object_, "sharding")),
            copy=from_json_copy(json_field(object_, "copy")),
        )
    elif kind == "tensorView":
        return TypeTensorView(
            kind_value=from_json_reference_kind(json_field(object_, "kind")),
            lifetime=destack._generated.mir.tree.lifetime.from_json_lifetime(
                json_field(object_, "lifetime")
            ),
            space=from_json_space(json_field(object_, "space")),
            access=from_json_access(json_field(object_, "access")),
            element=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "element")
            ),
            shape=[
                from_json_tensor_dimension(item_0)
                for item_0 in json_array(json_field(object_, "shape"))
            ],
            format=from_json_tensor_view_format(json_field(object_, "format")),
            sharding=from_json_tensor_sharding(json_field(object_, "sharding")),
            nullability=from_json_nullability(json_field(object_, "nullability")),
        )
    elif kind == "functionSignature":
        return TypeFunctionSignature(
            lifetimes=[
                destack._generated.mir.tree.lifetime.from_json_lifetime_parameter(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "lifetimes"))
            ],
            parameters=[
                destack._generated.mir.tree.parameter.from_json_signature_parameter(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "parameters"))
            ],
            result=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "result")
            ),
        )
    elif kind == "function":
        return TypeFunction(
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
            environment=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "environment")
            ),
        )
    elif kind == "functionPointer":
        return TypeFunctionPointer(
            signature=destack._generated.mir.tree.node.from_json_local_node_id(
                json_field(object_, "signature")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "FloatType",
    "encode_float_type",
    "decode_float_type",
    "to_json_float_type",
    "from_json_float_type",
    "Mutability",
    "encode_mutability",
    "decode_mutability",
    "to_json_mutability",
    "from_json_mutability",
    "ReferenceKind",
    "encode_reference_kind",
    "decode_reference_kind",
    "to_json_reference_kind",
    "from_json_reference_kind",
    "Space",
    "encode_space",
    "decode_space",
    "to_json_space",
    "from_json_space",
    "Access",
    "encode_access",
    "decode_access",
    "to_json_access",
    "from_json_access",
    "Nullability",
    "encode_nullability",
    "decode_nullability",
    "to_json_nullability",
    "from_json_nullability",
    "Copy",
    "encode_copy",
    "decode_copy",
    "to_json_copy",
    "from_json_copy",
    "VariantCase",
    "encode_variant_case",
    "decode_variant_case",
    "to_json_variant_case",
    "from_json_variant_case",
    "TensorDimension",
    "encode_tensor_dimension",
    "decode_tensor_dimension",
    "to_json_tensor_dimension",
    "from_json_tensor_dimension",
    "TensorDimensionStatic",
    "TensorDimensionSymbol",
    "TensorDimensionDynamic",
    "TensorFormat",
    "encode_tensor_format",
    "decode_tensor_format",
    "to_json_tensor_format",
    "from_json_tensor_format",
    "TensorFormatDense",
    "TensorDimensionOrder",
    "encode_tensor_dimension_order",
    "decode_tensor_dimension_order",
    "to_json_tensor_dimension_order",
    "from_json_tensor_dimension_order",
    "TensorSharding",
    "encode_tensor_sharding",
    "decode_tensor_sharding",
    "to_json_tensor_sharding",
    "from_json_tensor_sharding",
    "TensorShardingUnsharded",
    "TensorShardingSharding",
    "TensorShardingAxis",
    "encode_tensor_sharding_axis",
    "decode_tensor_sharding_axis",
    "to_json_tensor_sharding_axis",
    "from_json_tensor_sharding_axis",
    "TensorShardingAxisShard",
    "TensorShardingAxisReplicate",
    "TensorShardingAxisPartial",
    "TensorReduction",
    "encode_tensor_reduction",
    "decode_tensor_reduction",
    "to_json_tensor_reduction",
    "from_json_tensor_reduction",
    "TensorViewFormat",
    "encode_tensor_view_format",
    "decode_tensor_view_format",
    "to_json_tensor_view_format",
    "from_json_tensor_view_format",
    "TensorViewFormatDense",
    "TensorViewFormatStrided",
    "TypeAlias",
    "encode_type_alias",
    "decode_type_alias",
    "to_json_type_alias",
    "from_json_type_alias",
    "Field",
    "encode_field",
    "decode_field",
    "to_json_field",
    "from_json_field",
    "Type",
    "encode_type",
    "decode_type",
    "to_json_type",
    "from_json_type",
    "TypeError",
    "TypeVoid",
    "TypeBoolean",
    "TypeInt",
    "TypeIsize",
    "TypeUsize",
    "TypeFloat",
    "TypeTypeDescriptor",
    "TypeTypeId",
    "TypeAtomic",
    "TypeDynamic",
    "TypeWithLifetimes",
    "TypeReference",
    "TypeSlice",
    "TypeUninit",
    "TypeFixedArray",
    "TypeTuple",
    "TypeStruct",
    "TypeNewtype",
    "TypeVariant",
    "TypeVector",
    "TypeTensor",
    "TypeTensorView",
    "TypeFunctionSignature",
    "TypeFunction",
    "TypeFunctionPointer",
]
