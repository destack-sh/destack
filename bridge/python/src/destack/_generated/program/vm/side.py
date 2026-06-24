# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_array_length,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.intrinsic
import destack._generated.mir.tree.node
import destack._generated.mir.tree.tensor
import destack._generated.mir.tree.value
import destack._generated.mir.tree.vector
import destack._generated.program.vm.atomic
import destack._generated.program.vm.function
import destack._generated.program.vm.projection
import destack._generated.program.vm.range
import destack._generated.program.vm.table
import destack._generated.program.vm.tensor
import destack._generated.program.vm.value


@dataclass(frozen=True, slots=True)
class AggregateSelect:
    """Aggregate select operation."""

    # the destination frame offset
    destination_offset: int
    # the condition cell offset
    condition_offset: int
    # the true source frame offset
    then_offset: int
    # the false source frame offset
    else_offset: int
    # the selected byte length
    byte_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_aggregate_select(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AggregateSelect:
        """Decode one AggregateSelect."""
        return decode_aggregate_select(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_aggregate_select(self)

    @classmethod
    def from_json(cls, value: Json) -> AggregateSelect:
        """Return one AggregateSelect from one JSON value."""
        return from_json_aggregate_select(value)


def encode_aggregate_select(writer: BinaryWriter, value: AggregateSelect) -> None:
    """Encode one AggregateSelect."""
    writer.write_unsigned(value.destination_offset)
    writer.write_unsigned(value.condition_offset)
    writer.write_unsigned(value.then_offset)
    writer.write_unsigned(value.else_offset)
    writer.write_unsigned(value.byte_len)


def decode_aggregate_select(reader: BinaryReader) -> AggregateSelect:
    """Decode one AggregateSelect."""
    destination_offset = reader.read_number()
    condition_offset = reader.read_number()
    then_offset = reader.read_number()
    else_offset = reader.read_number()
    byte_len = reader.read_number()

    return AggregateSelect(
        destination_offset=destination_offset,
        condition_offset=condition_offset,
        then_offset=then_offset,
        else_offset=else_offset,
        byte_len=byte_len,
    )


def to_json_aggregate_select(value: AggregateSelect) -> Json:
    """Return one JSON value for one AggregateSelect."""
    return {
        "destinationOffset": value.destination_offset,
        "conditionOffset": value.condition_offset,
        "thenOffset": value.then_offset,
        "elseOffset": value.else_offset,
        "byteLen": value.byte_len,
    }


def from_json_aggregate_select(value: Json) -> AggregateSelect:
    """Return one AggregateSelect from one JSON value."""
    object_ = json_object(value)

    return AggregateSelect(
        destination_offset=json_int(json_field(object_, "destinationOffset")),
        condition_offset=json_int(json_field(object_, "conditionOffset")),
        then_offset=json_int(json_field(object_, "thenOffset")),
        else_offset=json_int(json_field(object_, "elseOffset")),
        byte_len=json_int(json_field(object_, "byteLen")),
    )


@dataclass(frozen=True, slots=True)
class AtomicCompareExchange:
    """Atomic compare exchange over one scalar value."""

    # the aggregate destination value
    destination: destack._generated.mir.tree.value.Value
    # the pointer cell offset
    pointer_offset: int
    # the expected value cell offset
    expected_offset: int
    # the replacement value cell offset
    new_value_offset: int
    # the atomic memory shape
    shape: destack._generated.program.vm.atomic.AtomicShape
    # the failure ordering
    failure_order: destack._generated.program.vm.atomic.AtomicOrder
    # whether the compare exchange is weak
    is_weak: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_atomic_compare_exchange(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AtomicCompareExchange:
        """Decode one AtomicCompareExchange."""
        return decode_atomic_compare_exchange(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_atomic_compare_exchange(self)

    @classmethod
    def from_json(cls, value: Json) -> AtomicCompareExchange:
        """Return one AtomicCompareExchange from one JSON value."""
        return from_json_atomic_compare_exchange(value)


def encode_atomic_compare_exchange(
    writer: BinaryWriter, value: AtomicCompareExchange
) -> None:
    """Encode one AtomicCompareExchange."""
    destack._generated.mir.tree.value.encode_value(writer, value.destination)
    writer.write_unsigned(value.pointer_offset)
    writer.write_unsigned(value.expected_offset)
    writer.write_unsigned(value.new_value_offset)
    destack._generated.program.vm.atomic.encode_atomic_shape(writer, value.shape)
    destack._generated.program.vm.atomic.encode_atomic_order(
        writer, value.failure_order
    )
    writer.write_bool(value.is_weak)


def decode_atomic_compare_exchange(reader: BinaryReader) -> AtomicCompareExchange:
    """Decode one AtomicCompareExchange."""
    destination = destack._generated.mir.tree.value.decode_value(reader)
    pointer_offset = reader.read_number()
    expected_offset = reader.read_number()
    new_value_offset = reader.read_number()
    shape = destack._generated.program.vm.atomic.decode_atomic_shape(reader)
    failure_order = destack._generated.program.vm.atomic.decode_atomic_order(reader)
    is_weak = reader.read_bool()

    return AtomicCompareExchange(
        destination=destination,
        pointer_offset=pointer_offset,
        expected_offset=expected_offset,
        new_value_offset=new_value_offset,
        shape=shape,
        failure_order=failure_order,
        is_weak=is_weak,
    )


def to_json_atomic_compare_exchange(value: AtomicCompareExchange) -> Json:
    """Return one JSON value for one AtomicCompareExchange."""
    return {
        "destination": destack._generated.mir.tree.value.to_json_value(
            value.destination
        ),
        "pointerOffset": value.pointer_offset,
        "expectedOffset": value.expected_offset,
        "newValueOffset": value.new_value_offset,
        "shape": destack._generated.program.vm.atomic.to_json_atomic_shape(value.shape),
        "failureOrder": destack._generated.program.vm.atomic.to_json_atomic_order(
            value.failure_order
        ),
        "isWeak": value.is_weak,
    }


def from_json_atomic_compare_exchange(value: Json) -> AtomicCompareExchange:
    """Return one AtomicCompareExchange from one JSON value."""
    object_ = json_object(value)

    return AtomicCompareExchange(
        destination=destack._generated.mir.tree.value.from_json_value(
            json_field(object_, "destination")
        ),
        pointer_offset=json_int(json_field(object_, "pointerOffset")),
        expected_offset=json_int(json_field(object_, "expectedOffset")),
        new_value_offset=json_int(json_field(object_, "newValueOffset")),
        shape=destack._generated.program.vm.atomic.from_json_atomic_shape(
            json_field(object_, "shape")
        ),
        failure_order=destack._generated.program.vm.atomic.from_json_atomic_order(
            json_field(object_, "failureOrder")
        ),
        is_weak=json_bool(json_field(object_, "isWeak")),
    )


@dataclass(frozen=True, slots=True)
class VectorSplat:
    """Vector splat operation."""

    # the destination frame offset
    dest_offset: int
    # the splatted value cell offset
    value_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_splat(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorSplat:
        """Decode one VectorSplat."""
        return decode_vector_splat(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_splat(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorSplat:
        """Return one VectorSplat from one JSON value."""
        return from_json_vector_splat(value)


def encode_vector_splat(writer: BinaryWriter, value: VectorSplat) -> None:
    """Encode one VectorSplat."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.value_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    writer.write_unsigned(value.element_count)


def decode_vector_splat(reader: BinaryReader) -> VectorSplat:
    """Decode one VectorSplat."""
    dest_offset = reader.read_number()
    value_offset = reader.read_number()
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    element_count = reader.read_number()

    return VectorSplat(
        dest_offset=dest_offset,
        value_offset=value_offset,
        dest_element=dest_element,
        element_count=element_count,
    )


def to_json_vector_splat(value: VectorSplat) -> Json:
    """Return one JSON value for one VectorSplat."""
    return {
        "destOffset": value.dest_offset,
        "valueOffset": value.value_offset,
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_splat(value: Json) -> VectorSplat:
    """Return one VectorSplat from one JSON value."""
    object_ = json_object(value)

    return VectorSplat(
        dest_offset=json_int(json_field(object_, "destOffset")),
        value_offset=json_int(json_field(object_, "valueOffset")),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorExtract:
    """Vector extract operation."""

    # the destination cell offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the index cell offset
    index_offset: int
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the source element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_extract(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorExtract:
        """Decode one VectorExtract."""
        return decode_vector_extract(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_extract(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorExtract:
        """Return one VectorExtract from one JSON value."""
        return from_json_vector_extract(value)


def encode_vector_extract(writer: BinaryWriter, value: VectorExtract) -> None:
    """Encode one VectorExtract."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.vector_offset)
    writer.write_unsigned(value.index_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.vector_element
    )
    writer.write_unsigned(value.element_count)


def decode_vector_extract(reader: BinaryReader) -> VectorExtract:
    """Decode one VectorExtract."""
    dest_offset = reader.read_number()
    vector_offset = reader.read_number()
    index_offset = reader.read_number()
    vector_element = destack._generated.program.vm.projection.decode_projection(reader)
    element_count = reader.read_number()

    return VectorExtract(
        dest_offset=dest_offset,
        vector_offset=vector_offset,
        index_offset=index_offset,
        vector_element=vector_element,
        element_count=element_count,
    )


def to_json_vector_extract(value: VectorExtract) -> Json:
    """Return one JSON value for one VectorExtract."""
    return {
        "destOffset": value.dest_offset,
        "vectorOffset": value.vector_offset,
        "indexOffset": value.index_offset,
        "vectorElement": destack._generated.program.vm.projection.to_json_projection(
            value.vector_element
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_extract(value: Json) -> VectorExtract:
    """Return one VectorExtract from one JSON value."""
    object_ = json_object(value)

    return VectorExtract(
        dest_offset=json_int(json_field(object_, "destOffset")),
        vector_offset=json_int(json_field(object_, "vectorOffset")),
        index_offset=json_int(json_field(object_, "indexOffset")),
        vector_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "vectorElement")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorBinary:
    """Elementwise vector binary operation."""

    # the destination frame offset
    dest_offset: int
    # the left vector frame offset
    left_offset: int
    # the right vector frame offset
    right_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the left source element projection
    left_element: destack._generated.program.vm.projection.Projection
    # the right source element projection
    right_element: destack._generated.program.vm.projection.Projection
    # the element binary kernel
    kernel: ElementBinaryKernel
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_binary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorBinary:
        """Decode one VectorBinary."""
        return decode_vector_binary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_binary(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorBinary:
        """Return one VectorBinary from one JSON value."""
        return from_json_vector_binary(value)


def encode_vector_binary(writer: BinaryWriter, value: VectorBinary) -> None:
    """Encode one VectorBinary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.left_offset)
    writer.write_unsigned(value.right_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.left_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.right_element
    )
    encode_element_binary_kernel(writer, value.kernel)
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    writer.write_unsigned(value.element_count)


def decode_vector_binary(reader: BinaryReader) -> VectorBinary:
    """Decode one VectorBinary."""
    dest_offset = reader.read_number()
    left_offset = reader.read_number()
    right_offset = reader.read_number()
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    left_element = destack._generated.program.vm.projection.decode_projection(reader)
    right_element = destack._generated.program.vm.projection.decode_projection(reader)
    kernel = decode_element_binary_kernel(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    element_count = reader.read_number()

    return VectorBinary(
        dest_offset=dest_offset,
        left_offset=left_offset,
        right_offset=right_offset,
        dest_element=dest_element,
        left_element=left_element,
        right_element=right_element,
        kernel=kernel,
        element_layout=element_layout,
        element_count=element_count,
    )


def to_json_vector_binary(value: VectorBinary) -> Json:
    """Return one JSON value for one VectorBinary."""
    return {
        "destOffset": value.dest_offset,
        "leftOffset": value.left_offset,
        "rightOffset": value.right_offset,
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "leftElement": destack._generated.program.vm.projection.to_json_projection(
            value.left_element
        ),
        "rightElement": destack._generated.program.vm.projection.to_json_projection(
            value.right_element
        ),
        "kernel": to_json_element_binary_kernel(value.kernel),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_binary(value: Json) -> VectorBinary:
    """Return one VectorBinary from one JSON value."""
    object_ = json_object(value)

    return VectorBinary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        left_offset=json_int(json_field(object_, "leftOffset")),
        right_offset=json_int(json_field(object_, "rightOffset")),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        left_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "leftElement")
        ),
        right_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "rightElement")
        ),
        kernel=from_json_element_binary_kernel(json_field(object_, "kernel")),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


"""Elementwise scalar binary kernel."""
ElementBinaryKernel: typing.TypeAlias = (
    typing.Literal["andBool"]
    | typing.Literal["orBool"]
    | typing.Literal["xorBool"]
    | typing.Literal["eqBool"]
    | typing.Literal["neBool"]
    | typing.Literal["addInt"]
    | typing.Literal["subInt"]
    | typing.Literal["mulInt"]
    | typing.Literal["divInt"]
    | typing.Literal["divUint"]
    | typing.Literal["remInt"]
    | typing.Literal["remUint"]
    | typing.Literal["andInt"]
    | typing.Literal["orInt"]
    | typing.Literal["xorInt"]
    | typing.Literal["shlInt"]
    | typing.Literal["shrInt"]
    | typing.Literal["shrUint"]
    | typing.Literal["eqInt"]
    | typing.Literal["neInt"]
    | typing.Literal["ltInt"]
    | typing.Literal["ltUint"]
    | typing.Literal["leInt"]
    | typing.Literal["leUint"]
    | typing.Literal["gtInt"]
    | typing.Literal["gtUint"]
    | typing.Literal["geInt"]
    | typing.Literal["geUint"]
    | typing.Literal["addF32"]
    | typing.Literal["subF32"]
    | typing.Literal["mulF32"]
    | typing.Literal["divF32"]
    | typing.Literal["addF64"]
    | typing.Literal["subF64"]
    | typing.Literal["mulF64"]
    | typing.Literal["divF64"]
    | typing.Literal["eqF32"]
    | typing.Literal["eqF64"]
    | typing.Literal["neF32"]
    | typing.Literal["neF64"]
    | typing.Literal["ltF32"]
    | typing.Literal["ltF64"]
    | typing.Literal["leF32"]
    | typing.Literal["leF64"]
    | typing.Literal["gtF32"]
    | typing.Literal["gtF64"]
    | typing.Literal["geF32"]
    | typing.Literal["geF64"]
    | typing.Literal["addFloat"]
    | typing.Literal["subFloat"]
    | typing.Literal["mulFloat"]
    | typing.Literal["divFloat"]
    | typing.Literal["eqFloat"]
    | typing.Literal["neFloat"]
    | typing.Literal["ltFloat"]
    | typing.Literal["leFloat"]
    | typing.Literal["gtFloat"]
    | typing.Literal["geFloat"]
)


def encode_element_binary_kernel(
    writer: BinaryWriter, value: ElementBinaryKernel
) -> None:
    """Encode one ElementBinaryKernel."""
    if value == "andBool":
        writer.write_unsigned(0)
    elif value == "orBool":
        writer.write_unsigned(1)
    elif value == "xorBool":
        writer.write_unsigned(2)
    elif value == "eqBool":
        writer.write_unsigned(3)
    elif value == "neBool":
        writer.write_unsigned(4)
    elif value == "addInt":
        writer.write_unsigned(5)
    elif value == "subInt":
        writer.write_unsigned(6)
    elif value == "mulInt":
        writer.write_unsigned(7)
    elif value == "divInt":
        writer.write_unsigned(8)
    elif value == "divUint":
        writer.write_unsigned(9)
    elif value == "remInt":
        writer.write_unsigned(10)
    elif value == "remUint":
        writer.write_unsigned(11)
    elif value == "andInt":
        writer.write_unsigned(12)
    elif value == "orInt":
        writer.write_unsigned(13)
    elif value == "xorInt":
        writer.write_unsigned(14)
    elif value == "shlInt":
        writer.write_unsigned(15)
    elif value == "shrInt":
        writer.write_unsigned(16)
    elif value == "shrUint":
        writer.write_unsigned(17)
    elif value == "eqInt":
        writer.write_unsigned(18)
    elif value == "neInt":
        writer.write_unsigned(19)
    elif value == "ltInt":
        writer.write_unsigned(20)
    elif value == "ltUint":
        writer.write_unsigned(21)
    elif value == "leInt":
        writer.write_unsigned(22)
    elif value == "leUint":
        writer.write_unsigned(23)
    elif value == "gtInt":
        writer.write_unsigned(24)
    elif value == "gtUint":
        writer.write_unsigned(25)
    elif value == "geInt":
        writer.write_unsigned(26)
    elif value == "geUint":
        writer.write_unsigned(27)
    elif value == "addF32":
        writer.write_unsigned(28)
    elif value == "subF32":
        writer.write_unsigned(29)
    elif value == "mulF32":
        writer.write_unsigned(30)
    elif value == "divF32":
        writer.write_unsigned(31)
    elif value == "addF64":
        writer.write_unsigned(32)
    elif value == "subF64":
        writer.write_unsigned(33)
    elif value == "mulF64":
        writer.write_unsigned(34)
    elif value == "divF64":
        writer.write_unsigned(35)
    elif value == "eqF32":
        writer.write_unsigned(36)
    elif value == "eqF64":
        writer.write_unsigned(37)
    elif value == "neF32":
        writer.write_unsigned(38)
    elif value == "neF64":
        writer.write_unsigned(39)
    elif value == "ltF32":
        writer.write_unsigned(40)
    elif value == "ltF64":
        writer.write_unsigned(41)
    elif value == "leF32":
        writer.write_unsigned(42)
    elif value == "leF64":
        writer.write_unsigned(43)
    elif value == "gtF32":
        writer.write_unsigned(44)
    elif value == "gtF64":
        writer.write_unsigned(45)
    elif value == "geF32":
        writer.write_unsigned(46)
    elif value == "geF64":
        writer.write_unsigned(47)
    elif value == "addFloat":
        writer.write_unsigned(48)
    elif value == "subFloat":
        writer.write_unsigned(49)
    elif value == "mulFloat":
        writer.write_unsigned(50)
    elif value == "divFloat":
        writer.write_unsigned(51)
    elif value == "eqFloat":
        writer.write_unsigned(52)
    elif value == "neFloat":
        writer.write_unsigned(53)
    elif value == "ltFloat":
        writer.write_unsigned(54)
    elif value == "leFloat":
        writer.write_unsigned(55)
    elif value == "gtFloat":
        writer.write_unsigned(56)
    elif value == "geFloat":
        writer.write_unsigned(57)
    else:
        raise SerdeError("unknown enum variant")


def decode_element_binary_kernel(reader: BinaryReader) -> ElementBinaryKernel:
    """Decode one ElementBinaryKernel."""
    variant = reader.read_number()

    if variant == 0:
        return "andBool"
    elif variant == 1:
        return "orBool"
    elif variant == 2:
        return "xorBool"
    elif variant == 3:
        return "eqBool"
    elif variant == 4:
        return "neBool"
    elif variant == 5:
        return "addInt"
    elif variant == 6:
        return "subInt"
    elif variant == 7:
        return "mulInt"
    elif variant == 8:
        return "divInt"
    elif variant == 9:
        return "divUint"
    elif variant == 10:
        return "remInt"
    elif variant == 11:
        return "remUint"
    elif variant == 12:
        return "andInt"
    elif variant == 13:
        return "orInt"
    elif variant == 14:
        return "xorInt"
    elif variant == 15:
        return "shlInt"
    elif variant == 16:
        return "shrInt"
    elif variant == 17:
        return "shrUint"
    elif variant == 18:
        return "eqInt"
    elif variant == 19:
        return "neInt"
    elif variant == 20:
        return "ltInt"
    elif variant == 21:
        return "ltUint"
    elif variant == 22:
        return "leInt"
    elif variant == 23:
        return "leUint"
    elif variant == 24:
        return "gtInt"
    elif variant == 25:
        return "gtUint"
    elif variant == 26:
        return "geInt"
    elif variant == 27:
        return "geUint"
    elif variant == 28:
        return "addF32"
    elif variant == 29:
        return "subF32"
    elif variant == 30:
        return "mulF32"
    elif variant == 31:
        return "divF32"
    elif variant == 32:
        return "addF64"
    elif variant == 33:
        return "subF64"
    elif variant == 34:
        return "mulF64"
    elif variant == 35:
        return "divF64"
    elif variant == 36:
        return "eqF32"
    elif variant == 37:
        return "eqF64"
    elif variant == 38:
        return "neF32"
    elif variant == 39:
        return "neF64"
    elif variant == 40:
        return "ltF32"
    elif variant == 41:
        return "ltF64"
    elif variant == 42:
        return "leF32"
    elif variant == 43:
        return "leF64"
    elif variant == 44:
        return "gtF32"
    elif variant == 45:
        return "gtF64"
    elif variant == 46:
        return "geF32"
    elif variant == 47:
        return "geF64"
    elif variant == 48:
        return "addFloat"
    elif variant == 49:
        return "subFloat"
    elif variant == 50:
        return "mulFloat"
    elif variant == 51:
        return "divFloat"
    elif variant == 52:
        return "eqFloat"
    elif variant == 53:
        return "neFloat"
    elif variant == 54:
        return "ltFloat"
    elif variant == 55:
        return "leFloat"
    elif variant == 56:
        return "gtFloat"
    elif variant == 57:
        return "geFloat"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_element_binary_kernel(value: ElementBinaryKernel) -> Json:
    """Return one JSON value for one ElementBinaryKernel."""
    return value


def from_json_element_binary_kernel(value: Json) -> ElementBinaryKernel:
    """Return one ElementBinaryKernel from one JSON value."""
    variant = json_string(value)

    if variant == "andBool":
        return "andBool"
    elif variant == "orBool":
        return "orBool"
    elif variant == "xorBool":
        return "xorBool"
    elif variant == "eqBool":
        return "eqBool"
    elif variant == "neBool":
        return "neBool"
    elif variant == "addInt":
        return "addInt"
    elif variant == "subInt":
        return "subInt"
    elif variant == "mulInt":
        return "mulInt"
    elif variant == "divInt":
        return "divInt"
    elif variant == "divUint":
        return "divUint"
    elif variant == "remInt":
        return "remInt"
    elif variant == "remUint":
        return "remUint"
    elif variant == "andInt":
        return "andInt"
    elif variant == "orInt":
        return "orInt"
    elif variant == "xorInt":
        return "xorInt"
    elif variant == "shlInt":
        return "shlInt"
    elif variant == "shrInt":
        return "shrInt"
    elif variant == "shrUint":
        return "shrUint"
    elif variant == "eqInt":
        return "eqInt"
    elif variant == "neInt":
        return "neInt"
    elif variant == "ltInt":
        return "ltInt"
    elif variant == "ltUint":
        return "ltUint"
    elif variant == "leInt":
        return "leInt"
    elif variant == "leUint":
        return "leUint"
    elif variant == "gtInt":
        return "gtInt"
    elif variant == "gtUint":
        return "gtUint"
    elif variant == "geInt":
        return "geInt"
    elif variant == "geUint":
        return "geUint"
    elif variant == "addF32":
        return "addF32"
    elif variant == "subF32":
        return "subF32"
    elif variant == "mulF32":
        return "mulF32"
    elif variant == "divF32":
        return "divF32"
    elif variant == "addF64":
        return "addF64"
    elif variant == "subF64":
        return "subF64"
    elif variant == "mulF64":
        return "mulF64"
    elif variant == "divF64":
        return "divF64"
    elif variant == "eqF32":
        return "eqF32"
    elif variant == "eqF64":
        return "eqF64"
    elif variant == "neF32":
        return "neF32"
    elif variant == "neF64":
        return "neF64"
    elif variant == "ltF32":
        return "ltF32"
    elif variant == "ltF64":
        return "ltF64"
    elif variant == "leF32":
        return "leF32"
    elif variant == "leF64":
        return "leF64"
    elif variant == "gtF32":
        return "gtF32"
    elif variant == "gtF64":
        return "gtF64"
    elif variant == "geF32":
        return "geF32"
    elif variant == "geF64":
        return "geF64"
    elif variant == "addFloat":
        return "addFloat"
    elif variant == "subFloat":
        return "subFloat"
    elif variant == "mulFloat":
        return "mulFloat"
    elif variant == "divFloat":
        return "divFloat"
    elif variant == "eqFloat":
        return "eqFloat"
    elif variant == "neFloat":
        return "neFloat"
    elif variant == "ltFloat":
        return "ltFloat"
    elif variant == "leFloat":
        return "leFloat"
    elif variant == "gtFloat":
        return "gtFloat"
    elif variant == "geFloat":
        return "geFloat"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class VectorUnary:
    """Elementwise vector unary operation."""

    # the destination frame offset
    dest_offset: int
    # the argument vector frame offset
    argument_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the argument element projection
    argument_element: destack._generated.program.vm.projection.Projection
    # the element unary kernel
    kernel: ElementUnaryKernel
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_unary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorUnary:
        """Decode one VectorUnary."""
        return decode_vector_unary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_unary(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorUnary:
        """Return one VectorUnary from one JSON value."""
        return from_json_vector_unary(value)


def encode_vector_unary(writer: BinaryWriter, value: VectorUnary) -> None:
    """Encode one VectorUnary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.argument_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.argument_element
    )
    encode_element_unary_kernel(writer, value.kernel)
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    writer.write_unsigned(value.element_count)


def decode_vector_unary(reader: BinaryReader) -> VectorUnary:
    """Decode one VectorUnary."""
    dest_offset = reader.read_number()
    argument_offset = reader.read_number()
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    argument_element = destack._generated.program.vm.projection.decode_projection(
        reader
    )
    kernel = decode_element_unary_kernel(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    element_count = reader.read_number()

    return VectorUnary(
        dest_offset=dest_offset,
        argument_offset=argument_offset,
        dest_element=dest_element,
        argument_element=argument_element,
        kernel=kernel,
        element_layout=element_layout,
        element_count=element_count,
    )


def to_json_vector_unary(value: VectorUnary) -> Json:
    """Return one JSON value for one VectorUnary."""
    return {
        "destOffset": value.dest_offset,
        "argumentOffset": value.argument_offset,
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "argumentElement": destack._generated.program.vm.projection.to_json_projection(
            value.argument_element
        ),
        "kernel": to_json_element_unary_kernel(value.kernel),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_unary(value: Json) -> VectorUnary:
    """Return one VectorUnary from one JSON value."""
    object_ = json_object(value)

    return VectorUnary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        argument_offset=json_int(json_field(object_, "argumentOffset")),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        argument_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "argumentElement")
        ),
        kernel=from_json_element_unary_kernel(json_field(object_, "kernel")),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


"""Elementwise scalar unary kernel."""
ElementUnaryKernel: typing.TypeAlias = (
    typing.Literal["notBool"]
    | typing.Literal["negInt"]
    | typing.Literal["notInt"]
    | typing.Literal["negF32"]
    | typing.Literal["negF64"]
    | typing.Literal["negFloat"]
)


def encode_element_unary_kernel(
    writer: BinaryWriter, value: ElementUnaryKernel
) -> None:
    """Encode one ElementUnaryKernel."""
    if value == "notBool":
        writer.write_unsigned(0)
    elif value == "negInt":
        writer.write_unsigned(1)
    elif value == "notInt":
        writer.write_unsigned(2)
    elif value == "negF32":
        writer.write_unsigned(3)
    elif value == "negF64":
        writer.write_unsigned(4)
    elif value == "negFloat":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_element_unary_kernel(reader: BinaryReader) -> ElementUnaryKernel:
    """Decode one ElementUnaryKernel."""
    variant = reader.read_number()

    if variant == 0:
        return "notBool"
    elif variant == 1:
        return "negInt"
    elif variant == 2:
        return "notInt"
    elif variant == 3:
        return "negF32"
    elif variant == 4:
        return "negF64"
    elif variant == 5:
        return "negFloat"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_element_unary_kernel(value: ElementUnaryKernel) -> Json:
    """Return one JSON value for one ElementUnaryKernel."""
    return value


def from_json_element_unary_kernel(value: Json) -> ElementUnaryKernel:
    """Return one ElementUnaryKernel from one JSON value."""
    variant = json_string(value)

    if variant == "notBool":
        return "notBool"
    elif variant == "negInt":
        return "negInt"
    elif variant == "notInt":
        return "notInt"
    elif variant == "negF32":
        return "negF32"
    elif variant == "negF64":
        return "negF64"
    elif variant == "negFloat":
        return "negFloat"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class VectorInsert:
    """Vector insert operation."""

    # the destination frame offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the index cell offset
    index_offset: int
    # the inserted value cell offset
    value_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the source vector element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_insert(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorInsert:
        """Decode one VectorInsert."""
        return decode_vector_insert(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_insert(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorInsert:
        """Return one VectorInsert from one JSON value."""
        return from_json_vector_insert(value)


def encode_vector_insert(writer: BinaryWriter, value: VectorInsert) -> None:
    """Encode one VectorInsert."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.vector_offset)
    writer.write_unsigned(value.index_offset)
    writer.write_unsigned(value.value_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.vector_element
    )
    writer.write_unsigned(value.element_count)


def decode_vector_insert(reader: BinaryReader) -> VectorInsert:
    """Decode one VectorInsert."""
    dest_offset = reader.read_number()
    vector_offset = reader.read_number()
    index_offset = reader.read_number()
    value_offset = reader.read_number()
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    vector_element = destack._generated.program.vm.projection.decode_projection(reader)
    element_count = reader.read_number()

    return VectorInsert(
        dest_offset=dest_offset,
        vector_offset=vector_offset,
        index_offset=index_offset,
        value_offset=value_offset,
        dest_element=dest_element,
        vector_element=vector_element,
        element_count=element_count,
    )


def to_json_vector_insert(value: VectorInsert) -> Json:
    """Return one JSON value for one VectorInsert."""
    return {
        "destOffset": value.dest_offset,
        "vectorOffset": value.vector_offset,
        "indexOffset": value.index_offset,
        "valueOffset": value.value_offset,
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "vectorElement": destack._generated.program.vm.projection.to_json_projection(
            value.vector_element
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_insert(value: Json) -> VectorInsert:
    """Return one VectorInsert from one JSON value."""
    object_ = json_object(value)

    return VectorInsert(
        dest_offset=json_int(json_field(object_, "destOffset")),
        vector_offset=json_int(json_field(object_, "vectorOffset")),
        index_offset=json_int(json_field(object_, "indexOffset")),
        value_offset=json_int(json_field(object_, "valueOffset")),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        vector_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "vectorElement")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorShuffle:
    """Vector shuffle operation."""

    # the destination frame offset
    dest_offset: int
    # the left source frame offset
    left_offset: int
    # the right source frame offset
    right_offset: int
    # the pooled shuffle mask
    mask: destack._generated.program.vm.table.U32RangeId
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the left source element projection
    left_element: destack._generated.program.vm.projection.Projection
    # the right source element projection
    right_element: destack._generated.program.vm.projection.Projection
    # the left source element count
    left_count: int
    # the right source element count
    right_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_shuffle(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorShuffle:
        """Decode one VectorShuffle."""
        return decode_vector_shuffle(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_shuffle(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorShuffle:
        """Return one VectorShuffle from one JSON value."""
        return from_json_vector_shuffle(value)


def encode_vector_shuffle(writer: BinaryWriter, value: VectorShuffle) -> None:
    """Encode one VectorShuffle."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.left_offset)
    writer.write_unsigned(value.right_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.mask)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.left_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.right_element
    )
    writer.write_unsigned(value.left_count)
    writer.write_unsigned(value.right_count)


def decode_vector_shuffle(reader: BinaryReader) -> VectorShuffle:
    """Decode one VectorShuffle."""
    dest_offset = reader.read_number()
    left_offset = reader.read_number()
    right_offset = reader.read_number()
    mask = destack._generated.program.vm.table.decode_u32_range_id(reader)
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    left_element = destack._generated.program.vm.projection.decode_projection(reader)
    right_element = destack._generated.program.vm.projection.decode_projection(reader)
    left_count = reader.read_number()
    right_count = reader.read_number()

    return VectorShuffle(
        dest_offset=dest_offset,
        left_offset=left_offset,
        right_offset=right_offset,
        mask=mask,
        dest_element=dest_element,
        left_element=left_element,
        right_element=right_element,
        left_count=left_count,
        right_count=right_count,
    )


def to_json_vector_shuffle(value: VectorShuffle) -> Json:
    """Return one JSON value for one VectorShuffle."""
    return {
        "destOffset": value.dest_offset,
        "leftOffset": value.left_offset,
        "rightOffset": value.right_offset,
        "mask": destack._generated.program.vm.table.to_json_u32_range_id(value.mask),
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "leftElement": destack._generated.program.vm.projection.to_json_projection(
            value.left_element
        ),
        "rightElement": destack._generated.program.vm.projection.to_json_projection(
            value.right_element
        ),
        "leftCount": value.left_count,
        "rightCount": value.right_count,
    }


def from_json_vector_shuffle(value: Json) -> VectorShuffle:
    """Return one VectorShuffle from one JSON value."""
    object_ = json_object(value)

    return VectorShuffle(
        dest_offset=json_int(json_field(object_, "destOffset")),
        left_offset=json_int(json_field(object_, "leftOffset")),
        right_offset=json_int(json_field(object_, "rightOffset")),
        mask=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "mask")
        ),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        left_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "leftElement")
        ),
        right_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "rightElement")
        ),
        left_count=json_int(json_field(object_, "leftCount")),
        right_count=json_int(json_field(object_, "rightCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorSelect:
    """Vector select operation."""

    # the destination frame offset
    dest_offset: int
    # the mask vector frame offset
    mask_offset: int
    # the true branch frame offset
    then_offset: int
    # the false branch frame offset
    else_offset: int
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the mask element projection
    mask_element: destack._generated.program.vm.projection.Projection
    # the true branch element projection
    then_element: destack._generated.program.vm.projection.Projection
    # the false branch element projection
    else_element: destack._generated.program.vm.projection.Projection
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_select(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorSelect:
        """Decode one VectorSelect."""
        return decode_vector_select(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_select(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorSelect:
        """Return one VectorSelect from one JSON value."""
        return from_json_vector_select(value)


def encode_vector_select(writer: BinaryWriter, value: VectorSelect) -> None:
    """Encode one VectorSelect."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.mask_offset)
    writer.write_unsigned(value.then_offset)
    writer.write_unsigned(value.else_offset)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.mask_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.then_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.else_element
    )
    writer.write_unsigned(value.element_count)


def decode_vector_select(reader: BinaryReader) -> VectorSelect:
    """Decode one VectorSelect."""
    dest_offset = reader.read_number()
    mask_offset = reader.read_number()
    then_offset = reader.read_number()
    else_offset = reader.read_number()
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    mask_element = destack._generated.program.vm.projection.decode_projection(reader)
    then_element = destack._generated.program.vm.projection.decode_projection(reader)
    else_element = destack._generated.program.vm.projection.decode_projection(reader)
    element_count = reader.read_number()

    return VectorSelect(
        dest_offset=dest_offset,
        mask_offset=mask_offset,
        then_offset=then_offset,
        else_offset=else_offset,
        dest_element=dest_element,
        mask_element=mask_element,
        then_element=then_element,
        else_element=else_element,
        element_count=element_count,
    )


def to_json_vector_select(value: VectorSelect) -> Json:
    """Return one JSON value for one VectorSelect."""
    return {
        "destOffset": value.dest_offset,
        "maskOffset": value.mask_offset,
        "thenOffset": value.then_offset,
        "elseOffset": value.else_offset,
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "maskElement": destack._generated.program.vm.projection.to_json_projection(
            value.mask_element
        ),
        "thenElement": destack._generated.program.vm.projection.to_json_projection(
            value.then_element
        ),
        "elseElement": destack._generated.program.vm.projection.to_json_projection(
            value.else_element
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_select(value: Json) -> VectorSelect:
    """Return one VectorSelect from one JSON value."""
    object_ = json_object(value)

    return VectorSelect(
        dest_offset=json_int(json_field(object_, "destOffset")),
        mask_offset=json_int(json_field(object_, "maskOffset")),
        then_offset=json_int(json_field(object_, "thenOffset")),
        else_offset=json_int(json_field(object_, "elseOffset")),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        mask_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "maskElement")
        ),
        then_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "thenElement")
        ),
        else_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "elseElement")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorReduce:
    """Vector reduction operation."""

    # the destination cell offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the reduction kernel
    kernel: destack._generated.mir.tree.vector.VectorReduceOperator
    # the source element projection
    vector_element: destack._generated.program.vm.projection.Projection
    # the vector element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the source vector element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_reduce(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorReduce:
        """Decode one VectorReduce."""
        return decode_vector_reduce(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_reduce(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorReduce:
        """Return one VectorReduce from one JSON value."""
        return from_json_vector_reduce(value)


def encode_vector_reduce(writer: BinaryWriter, value: VectorReduce) -> None:
    """Encode one VectorReduce."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.vector_offset)
    destack._generated.mir.tree.vector.encode_vector_reduce_operator(
        writer, value.kernel
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.vector_element
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    writer.write_unsigned(value.element_count)


def decode_vector_reduce(reader: BinaryReader) -> VectorReduce:
    """Decode one VectorReduce."""
    dest_offset = reader.read_number()
    vector_offset = reader.read_number()
    kernel = destack._generated.mir.tree.vector.decode_vector_reduce_operator(reader)
    vector_element = destack._generated.program.vm.projection.decode_projection(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    element_count = reader.read_number()

    return VectorReduce(
        dest_offset=dest_offset,
        vector_offset=vector_offset,
        kernel=kernel,
        vector_element=vector_element,
        element_layout=element_layout,
        element_count=element_count,
    )


def to_json_vector_reduce(value: VectorReduce) -> Json:
    """Return one JSON value for one VectorReduce."""
    return {
        "destOffset": value.dest_offset,
        "vectorOffset": value.vector_offset,
        "kernel": destack._generated.mir.tree.vector.to_json_vector_reduce_operator(
            value.kernel
        ),
        "vectorElement": destack._generated.program.vm.projection.to_json_projection(
            value.vector_element
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_reduce(value: Json) -> VectorReduce:
    """Return one VectorReduce from one JSON value."""
    object_ = json_object(value)

    return VectorReduce(
        dest_offset=json_int(json_field(object_, "destOffset")),
        vector_offset=json_int(json_field(object_, "vectorOffset")),
        kernel=destack._generated.mir.tree.vector.from_json_vector_reduce_operator(
            json_field(object_, "kernel")
        ),
        vector_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "vectorElement")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class VectorConvert:
    """Vector conversion operation."""

    # the destination frame offset
    dest_offset: int
    # the source vector frame offset
    vector_offset: int
    # the conversion kernel
    mode: destack._generated.mir.tree.vector.VectorConvertMode
    # the destination element projection
    dest_element: destack._generated.program.vm.projection.Projection
    # the source element projection
    source_element: destack._generated.program.vm.projection.Projection
    # the destination element scalar layout
    dest_layout: destack._generated.program.vm.value.ScalarLayout
    # the source element scalar layout
    source_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination element count
    element_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vector_convert(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> VectorConvert:
        """Decode one VectorConvert."""
        return decode_vector_convert(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vector_convert(self)

    @classmethod
    def from_json(cls, value: Json) -> VectorConvert:
        """Return one VectorConvert from one JSON value."""
        return from_json_vector_convert(value)


def encode_vector_convert(writer: BinaryWriter, value: VectorConvert) -> None:
    """Encode one VectorConvert."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.vector_offset)
    destack._generated.mir.tree.vector.encode_vector_convert_mode(writer, value.mode)
    destack._generated.program.vm.projection.encode_projection(
        writer, value.dest_element
    )
    destack._generated.program.vm.projection.encode_projection(
        writer, value.source_element
    )
    destack._generated.program.vm.value.encode_scalar_layout(writer, value.dest_layout)
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.source_layout
    )
    writer.write_unsigned(value.element_count)


def decode_vector_convert(reader: BinaryReader) -> VectorConvert:
    """Decode one VectorConvert."""
    dest_offset = reader.read_number()
    vector_offset = reader.read_number()
    mode = destack._generated.mir.tree.vector.decode_vector_convert_mode(reader)
    dest_element = destack._generated.program.vm.projection.decode_projection(reader)
    source_element = destack._generated.program.vm.projection.decode_projection(reader)
    dest_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    source_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    element_count = reader.read_number()

    return VectorConvert(
        dest_offset=dest_offset,
        vector_offset=vector_offset,
        mode=mode,
        dest_element=dest_element,
        source_element=source_element,
        dest_layout=dest_layout,
        source_layout=source_layout,
        element_count=element_count,
    )


def to_json_vector_convert(value: VectorConvert) -> Json:
    """Return one JSON value for one VectorConvert."""
    return {
        "destOffset": value.dest_offset,
        "vectorOffset": value.vector_offset,
        "mode": destack._generated.mir.tree.vector.to_json_vector_convert_mode(
            value.mode
        ),
        "destElement": destack._generated.program.vm.projection.to_json_projection(
            value.dest_element
        ),
        "sourceElement": destack._generated.program.vm.projection.to_json_projection(
            value.source_element
        ),
        "destLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.dest_layout
        ),
        "sourceLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.source_layout
        ),
        "elementCount": value.element_count,
    }


def from_json_vector_convert(value: Json) -> VectorConvert:
    """Return one VectorConvert from one JSON value."""
    object_ = json_object(value)

    return VectorConvert(
        dest_offset=json_int(json_field(object_, "destOffset")),
        vector_offset=json_int(json_field(object_, "vectorOffset")),
        mode=destack._generated.mir.tree.vector.from_json_vector_convert_mode(
            json_field(object_, "mode")
        ),
        dest_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "destElement")
        ),
        source_element=destack._generated.program.vm.projection.from_json_projection(
            json_field(object_, "sourceElement")
        ),
        dest_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "destLayout")
        ),
        source_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "sourceLayout")
        ),
        element_count=json_int(json_field(object_, "elementCount")),
    )


@dataclass(frozen=True, slots=True)
class FunctionBind:
    """Function bind operation."""

    # the environment cell layout
    environment: destack._generated.program.vm.value.CellLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_bind(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBind:
        """Decode one FunctionBind."""
        return decode_function_bind(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_bind(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionBind:
        """Return one FunctionBind from one JSON value."""
        return from_json_function_bind(value)


def encode_function_bind(writer: BinaryWriter, value: FunctionBind) -> None:
    """Encode one FunctionBind."""
    destack._generated.program.vm.value.encode_cell_layout(writer, value.environment)


def decode_function_bind(reader: BinaryReader) -> FunctionBind:
    """Decode one FunctionBind."""
    environment = destack._generated.program.vm.value.decode_cell_layout(reader)

    return FunctionBind(
        environment=environment,
    )


def to_json_function_bind(value: FunctionBind) -> Json:
    """Return one JSON value for one FunctionBind."""
    return {
        "environment": destack._generated.program.vm.value.to_json_cell_layout(
            value.environment
        ),
    }


def from_json_function_bind(value: Json) -> FunctionBind:
    """Return one FunctionBind from one JSON value."""
    object_ = json_object(value)

    return FunctionBind(
        environment=destack._generated.program.vm.value.from_json_cell_layout(
            json_field(object_, "environment")
        ),
    )


@dataclass(frozen=True, slots=True)
class Call:
    """Direct function call."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the argument frame moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Call:
        """Decode one Call."""
        return decode_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call(self)

    @classmethod
    def from_json(cls, value: Json) -> Call:
        """Return one Call from one JSON value."""
        return from_json_call(value)


def encode_call(writer: BinaryWriter, value: Call) -> None:
    """Encode one Call."""
    writer.write_unsigned(value.function)
    destack._generated.program.vm.function.encode_call_target(writer, value.target)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    destack._generated.program.vm.range.encode_move_range(writer, value.moves)


def decode_call(reader: BinaryReader) -> Call:
    """Decode one Call."""
    function = reader.read_number()
    target = destack._generated.program.vm.function.decode_call_target(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    moves = destack._generated.program.vm.range.decode_move_range(reader)

    return Call(
        function=function,
        target=target,
        arguments=arguments,
        moves=moves,
    )


def to_json_call(value: Call) -> Json:
    """Return one JSON value for one Call."""
    return {
        "function": value.function,
        "target": destack._generated.program.vm.function.to_json_call_target(
            value.target
        ),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "moves": destack._generated.program.vm.range.to_json_move_range(value.moves),
    }


def from_json_call(value: Json) -> Call:
    """Return one Call from one JSON value."""
    object_ = json_object(value)

    return Call(
        function=json_int(json_field(object_, "function")),
        target=destack._generated.program.vm.function.from_json_call_target(
            json_field(object_, "target")
        ),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        moves=destack._generated.program.vm.range.from_json_move_range(
            json_field(object_, "moves")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallBranch:
    """Function call terminator with an explicit continuation."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallBranch:
        """Decode one CallBranch."""
        return decode_call_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> CallBranch:
        """Return one CallBranch from one JSON value."""
        return from_json_call_branch(value)


def encode_call_branch(writer: BinaryWriter, value: CallBranch) -> None:
    """Encode one CallBranch."""
    writer.write_unsigned(value.function)
    destack._generated.program.vm.function.encode_call_target(writer, value.target)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.target_state
    )


def decode_call_branch(reader: BinaryReader) -> CallBranch:
    """Decode one CallBranch."""
    function = reader.read_number()
    target = destack._generated.program.vm.function.decode_call_target(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    target_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)

    return CallBranch(
        function=function,
        target=target,
        arguments=arguments,
        target_state=target_state,
    )


def to_json_call_branch(value: CallBranch) -> Json:
    """Return one JSON value for one CallBranch."""
    return {
        "function": value.function,
        "target": destack._generated.program.vm.function.to_json_call_target(
            value.target
        ),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "targetState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.target_state
        ),
    }


def from_json_call_branch(value: Json) -> CallBranch:
    """Return one CallBranch from one JSON value."""
    object_ = json_object(value)

    return CallBranch(
        function=json_int(json_field(object_, "function")),
        target=destack._generated.program.vm.function.from_json_call_target(
            json_field(object_, "target")
        ),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        target_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "targetState")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallVirtual:
    """Class method call."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_virtual(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallVirtual:
        """Decode one CallVirtual."""
        return decode_call_virtual(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_virtual(self)

    @classmethod
    def from_json(cls, value: Json) -> CallVirtual:
        """Return one CallVirtual from one JSON value."""
        return from_json_call_virtual(value)


def encode_call_virtual(writer: BinaryWriter, value: CallVirtual) -> None:
    """Encode one CallVirtual."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_call_virtual(reader: BinaryReader) -> CallVirtual:
    """Decode one CallVirtual."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return CallVirtual(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
    )


def to_json_call_virtual(value: CallVirtual) -> Json:
    """Return one JSON value for one CallVirtual."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_call_virtual(value: Json) -> CallVirtual:
    """Return one CallVirtual from one JSON value."""
    object_ = json_object(value)

    return CallVirtual(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallVirtualBranch:
    """Class method call terminator with an explicit continuation."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_virtual_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallVirtualBranch:
        """Decode one CallVirtualBranch."""
        return decode_call_virtual_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_virtual_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> CallVirtualBranch:
        """Return one CallVirtualBranch from one JSON value."""
        return from_json_call_virtual_branch(value)


def encode_call_virtual_branch(writer: BinaryWriter, value: CallVirtualBranch) -> None:
    """Encode one CallVirtualBranch."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.target_state
    )


def decode_call_virtual_branch(reader: BinaryReader) -> CallVirtualBranch:
    """Decode one CallVirtualBranch."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    target_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)

    return CallVirtualBranch(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
        target_state=target_state,
    )


def to_json_call_virtual_branch(value: CallVirtualBranch) -> Json:
    """Return one JSON value for one CallVirtualBranch."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "targetState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.target_state
        ),
    }


def from_json_call_virtual_branch(value: Json) -> CallVirtualBranch:
    """Return one CallVirtualBranch from one JSON value."""
    object_ = json_object(value)

    return CallVirtualBranch(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        target_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "targetState")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallDynamic:
    """Dynamic method call."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_dynamic(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallDynamic:
        """Decode one CallDynamic."""
        return decode_call_dynamic(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_dynamic(self)

    @classmethod
    def from_json(cls, value: Json) -> CallDynamic:
        """Return one CallDynamic from one JSON value."""
        return from_json_call_dynamic(value)


def encode_call_dynamic(writer: BinaryWriter, value: CallDynamic) -> None:
    """Encode one CallDynamic."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_call_dynamic(reader: BinaryReader) -> CallDynamic:
    """Decode one CallDynamic."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return CallDynamic(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
    )


def to_json_call_dynamic(value: CallDynamic) -> Json:
    """Return one JSON value for one CallDynamic."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_call_dynamic(value: Json) -> CallDynamic:
    """Return one CallDynamic from one JSON value."""
    object_ = json_object(value)

    return CallDynamic(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


@dataclass(frozen=True, slots=True)
class CallDynamicBranch:
    """Dynamic method call terminator with an explicit continuation."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_dynamic_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallDynamicBranch:
        """Decode one CallDynamicBranch."""
        return decode_call_dynamic_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_dynamic_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> CallDynamicBranch:
        """Return one CallDynamicBranch from one JSON value."""
        return from_json_call_dynamic_branch(value)


def encode_call_dynamic_branch(writer: BinaryWriter, value: CallDynamicBranch) -> None:
    """Encode one CallDynamicBranch."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.target_state
    )


def decode_call_dynamic_branch(reader: BinaryReader) -> CallDynamicBranch:
    """Decode one CallDynamicBranch."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    target_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)

    return CallDynamicBranch(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
        target_state=target_state,
    )


def to_json_call_dynamic_branch(value: CallDynamicBranch) -> Json:
    """Return one JSON value for one CallDynamicBranch."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "targetState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.target_state
        ),
    }


def from_json_call_dynamic_branch(value: Json) -> CallDynamicBranch:
    """Return one CallDynamicBranch from one JSON value."""
    object_ = json_object(value)

    return CallDynamicBranch(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        target_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "targetState")
        ),
    )


@dataclass(frozen=True, slots=True)
class IndirectCall:
    """Indirect function call."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_indirect_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectCall:
        """Decode one IndirectCall."""
        return decode_indirect_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_indirect_call(self)

    @classmethod
    def from_json(cls, value: Json) -> IndirectCall:
        """Return one IndirectCall from one JSON value."""
        return from_json_indirect_call(value)


def encode_indirect_call(writer: BinaryWriter, value: IndirectCall) -> None:
    """Encode one IndirectCall."""
    writer.write_unsigned(value.callee_offset)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_indirect_call(reader: BinaryReader) -> IndirectCall:
    """Decode one IndirectCall."""
    callee_offset = reader.read_number()
    signature = destack._generated.mir.tree.node.decode_local_node_id(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return IndirectCall(
        callee_offset=callee_offset,
        signature=signature,
        arguments=arguments,
    )


def to_json_indirect_call(value: IndirectCall) -> Json:
    """Return one JSON value for one IndirectCall."""
    return {
        "calleeOffset": value.callee_offset,
        "signature": destack._generated.mir.tree.node.to_json_local_node_id(
            value.signature
        ),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_indirect_call(value: Json) -> IndirectCall:
    """Return one IndirectCall from one JSON value."""
    object_ = json_object(value)

    return IndirectCall(
        callee_offset=json_int(json_field(object_, "calleeOffset")),
        signature=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "signature")
        ),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


@dataclass(frozen=True, slots=True)
class IndirectCallBranch:
    """Indirect call terminator with an explicit continuation."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the continuation frame state
    target_state: destack._generated.mir.metadata.frame.FrameStateId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_indirect_call_branch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectCallBranch:
        """Decode one IndirectCallBranch."""
        return decode_indirect_call_branch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_indirect_call_branch(self)

    @classmethod
    def from_json(cls, value: Json) -> IndirectCallBranch:
        """Return one IndirectCallBranch from one JSON value."""
        return from_json_indirect_call_branch(value)


def encode_indirect_call_branch(
    writer: BinaryWriter, value: IndirectCallBranch
) -> None:
    """Encode one IndirectCallBranch."""
    writer.write_unsigned(value.callee_offset)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.target_state
    )


def decode_indirect_call_branch(reader: BinaryReader) -> IndirectCallBranch:
    """Decode one IndirectCallBranch."""
    callee_offset = reader.read_number()
    signature = destack._generated.mir.tree.node.decode_local_node_id(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    target_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)

    return IndirectCallBranch(
        callee_offset=callee_offset,
        signature=signature,
        arguments=arguments,
        target_state=target_state,
    )


def to_json_indirect_call_branch(value: IndirectCallBranch) -> Json:
    """Return one JSON value for one IndirectCallBranch."""
    return {
        "calleeOffset": value.callee_offset,
        "signature": destack._generated.mir.tree.node.to_json_local_node_id(
            value.signature
        ),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "targetState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.target_state
        ),
    }


def from_json_indirect_call_branch(value: Json) -> IndirectCallBranch:
    """Return one IndirectCallBranch from one JSON value."""
    object_ = json_object(value)

    return IndirectCallBranch(
        callee_offset=json_int(json_field(object_, "calleeOffset")),
        signature=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "signature")
        ),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        target_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "targetState")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorLoad:
    """Load a tensor element from a view."""

    # the destination scalar
    dest_offset: int
    # the source tensor view
    view_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_load(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorLoad:
        """Decode one TensorLoad."""
        return decode_tensor_load(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_load(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorLoad:
        """Return one TensorLoad from one JSON value."""
        return from_json_tensor_load(value)


def encode_tensor_load(writer: BinaryWriter, value: TensorLoad) -> None:
    """Encode one TensorLoad."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.view_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.indices)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.view_layout
    )
    destack._generated.program.vm.table.encode_projection_id(writer, value.element)
    destack._generated.program.vm.tensor.encode_tensor_address(writer, value.address)


def decode_tensor_load(reader: BinaryReader) -> TensorLoad:
    """Decode one TensorLoad."""
    dest_offset = reader.read_number()
    view_offset = reader.read_number()
    indices = destack._generated.program.vm.table.decode_u32_range_id(reader)
    view_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element = destack._generated.program.vm.table.decode_projection_id(reader)
    address = destack._generated.program.vm.tensor.decode_tensor_address(reader)

    return TensorLoad(
        dest_offset=dest_offset,
        view_offset=view_offset,
        indices=indices,
        view_layout=view_layout,
        element=element,
        address=address,
    )


def to_json_tensor_load(value: TensorLoad) -> Json:
    """Return one JSON value for one TensorLoad."""
    return {
        "destOffset": value.dest_offset,
        "viewOffset": value.view_offset,
        "indices": destack._generated.program.vm.table.to_json_u32_range_id(
            value.indices
        ),
        "viewLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.view_layout
        ),
        "element": destack._generated.program.vm.table.to_json_projection_id(
            value.element
        ),
        "address": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.address
        ),
    }


def from_json_tensor_load(value: Json) -> TensorLoad:
    """Return one TensorLoad from one JSON value."""
    object_ = json_object(value)

    return TensorLoad(
        dest_offset=json_int(json_field(object_, "destOffset")),
        view_offset=json_int(json_field(object_, "viewOffset")),
        indices=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "indices")
        ),
        view_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "viewLayout")
        ),
        element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "element")
        ),
        address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "address")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorExtract:
    """Extract a tensor element from a tensor value."""

    # the destination scalar frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    tensor_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_extract(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorExtract:
        """Decode one TensorExtract."""
        return decode_tensor_extract(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_extract(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorExtract:
        """Return one TensorExtract from one JSON value."""
        return from_json_tensor_extract(value)


def encode_tensor_extract(writer: BinaryWriter, value: TensorExtract) -> None:
    """Encode one TensorExtract."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.indices)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.tensor_layout
    )


def decode_tensor_extract(reader: BinaryReader) -> TensorExtract:
    """Decode one TensorExtract."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    indices = destack._generated.program.vm.table.decode_u32_range_id(reader)
    tensor_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorExtract(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        indices=indices,
        tensor_layout=tensor_layout,
    )


def to_json_tensor_extract(value: TensorExtract) -> Json:
    """Return one JSON value for one TensorExtract."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "indices": destack._generated.program.vm.table.to_json_u32_range_id(
            value.indices
        ),
        "tensorLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.tensor_layout
        ),
    }


def from_json_tensor_extract(value: Json) -> TensorExtract:
    """Return one TensorExtract from one JSON value."""
    object_ = json_object(value)

    return TensorExtract(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        indices=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "indices")
        ),
        tensor_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "tensorLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorBinary:
    """Elementwise tensor binary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the left tensor layout
    left_layout: destack._generated.program.vm.table.TensorLayoutId
    # the right tensor layout
    right_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the element binary kernel
    kernel: ElementBinaryKernel
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_binary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorBinary:
        """Decode one TensorBinary."""
        return decode_tensor_binary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_binary(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorBinary:
        """Return one TensorBinary from one JSON value."""
        return from_json_tensor_binary(value)


def encode_tensor_binary(writer: BinaryWriter, value: TensorBinary) -> None:
    """Encode one TensorBinary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.left_offset)
    writer.write_unsigned(value.right_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.left_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.right_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    encode_element_binary_kernel(writer, value.kernel)
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )


def decode_tensor_binary(reader: BinaryReader) -> TensorBinary:
    """Decode one TensorBinary."""
    dest_offset = reader.read_number()
    left_offset = reader.read_number()
    right_offset = reader.read_number()
    left_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    right_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    kernel = decode_element_binary_kernel(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)

    return TensorBinary(
        dest_offset=dest_offset,
        left_offset=left_offset,
        right_offset=right_offset,
        left_layout=left_layout,
        right_layout=right_layout,
        dest_layout=dest_layout,
        kernel=kernel,
        element_layout=element_layout,
    )


def to_json_tensor_binary(value: TensorBinary) -> Json:
    """Return one JSON value for one TensorBinary."""
    return {
        "destOffset": value.dest_offset,
        "leftOffset": value.left_offset,
        "rightOffset": value.right_offset,
        "leftLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.left_layout
        ),
        "rightLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.right_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "kernel": to_json_element_binary_kernel(value.kernel),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
    }


def from_json_tensor_binary(value: Json) -> TensorBinary:
    """Return one TensorBinary from one JSON value."""
    object_ = json_object(value)

    return TensorBinary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        left_offset=json_int(json_field(object_, "leftOffset")),
        right_offset=json_int(json_field(object_, "rightOffset")),
        left_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "leftLayout")
        ),
        right_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "rightLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        kernel=from_json_element_binary_kernel(json_field(object_, "kernel")),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorContiguousBinary:
    """Contiguous elementwise tensor binary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the contiguous binary kernel selected during lowering
    kernel: ElementBinaryKernel

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_contiguous_binary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorContiguousBinary:
        """Decode one TensorContiguousBinary."""
        return decode_tensor_contiguous_binary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_contiguous_binary(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorContiguousBinary:
        """Return one TensorContiguousBinary from one JSON value."""
        return from_json_tensor_contiguous_binary(value)


def encode_tensor_contiguous_binary(
    writer: BinaryWriter, value: TensorContiguousBinary
) -> None:
    """Encode one TensorContiguousBinary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.left_offset)
    writer.write_unsigned(value.right_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    encode_element_binary_kernel(writer, value.kernel)


def decode_tensor_contiguous_binary(reader: BinaryReader) -> TensorContiguousBinary:
    """Decode one TensorContiguousBinary."""
    dest_offset = reader.read_number()
    left_offset = reader.read_number()
    right_offset = reader.read_number()
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    kernel = decode_element_binary_kernel(reader)

    return TensorContiguousBinary(
        dest_offset=dest_offset,
        left_offset=left_offset,
        right_offset=right_offset,
        dest_layout=dest_layout,
        element_layout=element_layout,
        kernel=kernel,
    )


def to_json_tensor_contiguous_binary(value: TensorContiguousBinary) -> Json:
    """Return one JSON value for one TensorContiguousBinary."""
    return {
        "destOffset": value.dest_offset,
        "leftOffset": value.left_offset,
        "rightOffset": value.right_offset,
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "kernel": to_json_element_binary_kernel(value.kernel),
    }


def from_json_tensor_contiguous_binary(value: Json) -> TensorContiguousBinary:
    """Return one TensorContiguousBinary from one JSON value."""
    object_ = json_object(value)

    return TensorContiguousBinary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        left_offset=json_int(json_field(object_, "leftOffset")),
        right_offset=json_int(json_field(object_, "rightOffset")),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        kernel=from_json_element_binary_kernel(json_field(object_, "kernel")),
    )


@dataclass(frozen=True, slots=True)
class TensorUnary:
    """Elementwise tensor unary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    argument_offset: int
    # the argument tensor layout
    argument_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the element unary kernel
    kernel: ElementUnaryKernel

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_unary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorUnary:
        """Decode one TensorUnary."""
        return decode_tensor_unary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_unary(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorUnary:
        """Return one TensorUnary from one JSON value."""
        return from_json_tensor_unary(value)


def encode_tensor_unary(writer: BinaryWriter, value: TensorUnary) -> None:
    """Encode one TensorUnary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.argument_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.argument_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    encode_element_unary_kernel(writer, value.kernel)


def decode_tensor_unary(reader: BinaryReader) -> TensorUnary:
    """Decode one TensorUnary."""
    dest_offset = reader.read_number()
    argument_offset = reader.read_number()
    argument_layout = destack._generated.program.vm.table.decode_tensor_layout_id(
        reader
    )
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    kernel = decode_element_unary_kernel(reader)

    return TensorUnary(
        dest_offset=dest_offset,
        argument_offset=argument_offset,
        argument_layout=argument_layout,
        element_layout=element_layout,
        dest_layout=dest_layout,
        kernel=kernel,
    )


def to_json_tensor_unary(value: TensorUnary) -> Json:
    """Return one JSON value for one TensorUnary."""
    return {
        "destOffset": value.dest_offset,
        "argumentOffset": value.argument_offset,
        "argumentLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.argument_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "kernel": to_json_element_unary_kernel(value.kernel),
    }


def from_json_tensor_unary(value: Json) -> TensorUnary:
    """Return one TensorUnary from one JSON value."""
    object_ = json_object(value)

    return TensorUnary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        argument_offset=json_int(json_field(object_, "argumentOffset")),
        argument_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "argumentLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        kernel=from_json_element_unary_kernel(json_field(object_, "kernel")),
    )


@dataclass(frozen=True, slots=True)
class TensorContiguousUnary:
    """Contiguous elementwise tensor unary operation."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    argument_offset: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the contiguous unary kernel selected during lowering
    kernel: ElementUnaryKernel

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_contiguous_unary(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorContiguousUnary:
        """Decode one TensorContiguousUnary."""
        return decode_tensor_contiguous_unary(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_contiguous_unary(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorContiguousUnary:
        """Return one TensorContiguousUnary from one JSON value."""
        return from_json_tensor_contiguous_unary(value)


def encode_tensor_contiguous_unary(
    writer: BinaryWriter, value: TensorContiguousUnary
) -> None:
    """Encode one TensorContiguousUnary."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.argument_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    encode_element_unary_kernel(writer, value.kernel)


def decode_tensor_contiguous_unary(reader: BinaryReader) -> TensorContiguousUnary:
    """Decode one TensorContiguousUnary."""
    dest_offset = reader.read_number()
    argument_offset = reader.read_number()
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    kernel = decode_element_unary_kernel(reader)

    return TensorContiguousUnary(
        dest_offset=dest_offset,
        argument_offset=argument_offset,
        dest_layout=dest_layout,
        element_layout=element_layout,
        kernel=kernel,
    )


def to_json_tensor_contiguous_unary(value: TensorContiguousUnary) -> Json:
    """Return one JSON value for one TensorContiguousUnary."""
    return {
        "destOffset": value.dest_offset,
        "argumentOffset": value.argument_offset,
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "kernel": to_json_element_unary_kernel(value.kernel),
    }


def from_json_tensor_contiguous_unary(value: Json) -> TensorContiguousUnary:
    """Return one TensorContiguousUnary from one JSON value."""
    object_ = json_object(value)

    return TensorContiguousUnary(
        dest_offset=json_int(json_field(object_, "destOffset")),
        argument_offset=json_int(json_field(object_, "argumentOffset")),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        kernel=from_json_element_unary_kernel(json_field(object_, "kernel")),
    )


@dataclass(frozen=True, slots=True)
class TensorStore:
    """Store a tensor element into a view."""

    # the destination tensor view
    view_offset: int
    # the index frame offsets
    indices: destack._generated.program.vm.table.U32RangeId
    # the stored scalar
    value_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_store(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorStore:
        """Decode one TensorStore."""
        return decode_tensor_store(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_store(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorStore:
        """Return one TensorStore from one JSON value."""
        return from_json_tensor_store(value)


def encode_tensor_store(writer: BinaryWriter, value: TensorStore) -> None:
    """Encode one TensorStore."""
    writer.write_unsigned(value.view_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.indices)
    writer.write_unsigned(value.value_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.view_layout
    )
    destack._generated.program.vm.table.encode_projection_id(writer, value.element)
    destack._generated.program.vm.tensor.encode_tensor_address(writer, value.address)


def decode_tensor_store(reader: BinaryReader) -> TensorStore:
    """Decode one TensorStore."""
    view_offset = reader.read_number()
    indices = destack._generated.program.vm.table.decode_u32_range_id(reader)
    value_offset = reader.read_number()
    view_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element = destack._generated.program.vm.table.decode_projection_id(reader)
    address = destack._generated.program.vm.tensor.decode_tensor_address(reader)

    return TensorStore(
        view_offset=view_offset,
        indices=indices,
        value_offset=value_offset,
        view_layout=view_layout,
        element=element,
        address=address,
    )


def to_json_tensor_store(value: TensorStore) -> Json:
    """Return one JSON value for one TensorStore."""
    return {
        "viewOffset": value.view_offset,
        "indices": destack._generated.program.vm.table.to_json_u32_range_id(
            value.indices
        ),
        "valueOffset": value.value_offset,
        "viewLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.view_layout
        ),
        "element": destack._generated.program.vm.table.to_json_projection_id(
            value.element
        ),
        "address": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.address
        ),
    }


def from_json_tensor_store(value: Json) -> TensorStore:
    """Return one TensorStore from one JSON value."""
    object_ = json_object(value)

    return TensorStore(
        view_offset=json_int(json_field(object_, "viewOffset")),
        indices=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "indices")
        ),
        value_offset=json_int(json_field(object_, "valueOffset")),
        view_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "viewLayout")
        ),
        element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "element")
        ),
        address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "address")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorFill:
    """Fill a tensor reference with a scalar value."""

    # the filled tensor view
    view_offset: int
    # the scalar fill value
    value_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_fill(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorFill:
        """Decode one TensorFill."""
        return decode_tensor_fill(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_fill(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorFill:
        """Return one TensorFill from one JSON value."""
        return from_json_tensor_fill(value)


def encode_tensor_fill(writer: BinaryWriter, value: TensorFill) -> None:
    """Encode one TensorFill."""
    writer.write_unsigned(value.view_offset)
    writer.write_unsigned(value.value_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.view_layout
    )
    destack._generated.program.vm.table.encode_projection_id(writer, value.element)
    destack._generated.program.vm.tensor.encode_tensor_address(writer, value.address)


def decode_tensor_fill(reader: BinaryReader) -> TensorFill:
    """Decode one TensorFill."""
    view_offset = reader.read_number()
    value_offset = reader.read_number()
    view_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element = destack._generated.program.vm.table.decode_projection_id(reader)
    address = destack._generated.program.vm.tensor.decode_tensor_address(reader)

    return TensorFill(
        view_offset=view_offset,
        value_offset=value_offset,
        view_layout=view_layout,
        element=element,
        address=address,
    )


def to_json_tensor_fill(value: TensorFill) -> Json:
    """Return one JSON value for one TensorFill."""
    return {
        "viewOffset": value.view_offset,
        "valueOffset": value.value_offset,
        "viewLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.view_layout
        ),
        "element": destack._generated.program.vm.table.to_json_projection_id(
            value.element
        ),
        "address": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.address
        ),
    }


def from_json_tensor_fill(value: Json) -> TensorFill:
    """Return one TensorFill from one JSON value."""
    object_ = json_object(value)

    return TensorFill(
        view_offset=json_int(json_field(object_, "viewOffset")),
        value_offset=json_int(json_field(object_, "valueOffset")),
        view_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "viewLayout")
        ),
        element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "element")
        ),
        address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "address")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorCopy:
    """Copy elements between tensor references."""

    # the target tensor view
    target_offset: int
    # the source tensor view
    source_offset: int
    # the target tensor view layout
    target_layout: destack._generated.program.vm.table.TensorLayoutId
    # the source tensor view layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the target element projection
    target_element: destack._generated.program.vm.table.ProjectionId
    # the source element projection
    source_element: destack._generated.program.vm.table.ProjectionId
    # the target tensor backing memory
    target_address: destack._generated.program.vm.tensor.TensorAddress
    # the source tensor backing memory
    source_address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_copy(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorCopy:
        """Decode one TensorCopy."""
        return decode_tensor_copy(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_copy(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorCopy:
        """Return one TensorCopy from one JSON value."""
        return from_json_tensor_copy(value)


def encode_tensor_copy(writer: BinaryWriter, value: TensorCopy) -> None:
    """Encode one TensorCopy."""
    writer.write_unsigned(value.target_offset)
    writer.write_unsigned(value.source_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.target_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_projection_id(
        writer, value.target_element
    )
    destack._generated.program.vm.table.encode_projection_id(
        writer, value.source_element
    )
    destack._generated.program.vm.tensor.encode_tensor_address(
        writer, value.target_address
    )
    destack._generated.program.vm.tensor.encode_tensor_address(
        writer, value.source_address
    )


def decode_tensor_copy(reader: BinaryReader) -> TensorCopy:
    """Decode one TensorCopy."""
    target_offset = reader.read_number()
    source_offset = reader.read_number()
    target_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    target_element = destack._generated.program.vm.table.decode_projection_id(reader)
    source_element = destack._generated.program.vm.table.decode_projection_id(reader)
    target_address = destack._generated.program.vm.tensor.decode_tensor_address(reader)
    source_address = destack._generated.program.vm.tensor.decode_tensor_address(reader)

    return TensorCopy(
        target_offset=target_offset,
        source_offset=source_offset,
        target_layout=target_layout,
        source_layout=source_layout,
        target_element=target_element,
        source_element=source_element,
        target_address=target_address,
        source_address=source_address,
    )


def to_json_tensor_copy(value: TensorCopy) -> Json:
    """Return one JSON value for one TensorCopy."""
    return {
        "targetOffset": value.target_offset,
        "sourceOffset": value.source_offset,
        "targetLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.target_layout
        ),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "targetElement": destack._generated.program.vm.table.to_json_projection_id(
            value.target_element
        ),
        "sourceElement": destack._generated.program.vm.table.to_json_projection_id(
            value.source_element
        ),
        "targetAddress": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.target_address
        ),
        "sourceAddress": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.source_address
        ),
    }


def from_json_tensor_copy(value: Json) -> TensorCopy:
    """Return one TensorCopy from one JSON value."""
    object_ = json_object(value)

    return TensorCopy(
        target_offset=json_int(json_field(object_, "targetOffset")),
        source_offset=json_int(json_field(object_, "sourceOffset")),
        target_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "targetLayout")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        target_element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "targetElement")
        ),
        source_element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "sourceElement")
        ),
        target_address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "targetAddress")
        ),
        source_address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "sourceAddress")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorViewCast:
    """Cast a dense pointer into a tensor view descriptor."""

    # the destination tensor view
    dest_offset: int
    # the MIR pointer cell
    pointer_offset: int
    # the tensor view layout
    view_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view_cast(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorViewCast:
        """Decode one TensorViewCast."""
        return decode_tensor_view_cast(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view_cast(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorViewCast:
        """Return one TensorViewCast from one JSON value."""
        return from_json_tensor_view_cast(value)


def encode_tensor_view_cast(writer: BinaryWriter, value: TensorViewCast) -> None:
    """Encode one TensorViewCast."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.pointer_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.view_layout
    )


def decode_tensor_view_cast(reader: BinaryReader) -> TensorViewCast:
    """Decode one TensorViewCast."""
    dest_offset = reader.read_number()
    pointer_offset = reader.read_number()
    view_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorViewCast(
        dest_offset=dest_offset,
        pointer_offset=pointer_offset,
        view_layout=view_layout,
    )


def to_json_tensor_view_cast(value: TensorViewCast) -> Json:
    """Return one JSON value for one TensorViewCast."""
    return {
        "destOffset": value.dest_offset,
        "pointerOffset": value.pointer_offset,
        "viewLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.view_layout
        ),
    }


def from_json_tensor_view_cast(value: Json) -> TensorViewCast:
    """Return one TensorViewCast from one JSON value."""
    object_ = json_object(value)

    return TensorViewCast(
        dest_offset=json_int(json_field(object_, "destOffset")),
        pointer_offset=json_int(json_field(object_, "pointerOffset")),
        view_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "viewLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorReshape:
    """Reshape a tensor into a new shape."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the destination shape values
    shape: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_reshape(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorReshape:
        """Decode one TensorReshape."""
        return decode_tensor_reshape(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_reshape(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorReshape:
        """Return one TensorReshape from one JSON value."""
        return from_json_tensor_reshape(value)


def encode_tensor_reshape(writer: BinaryWriter, value: TensorReshape) -> None:
    """Encode one TensorReshape."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.shape)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_reshape(reader: BinaryReader) -> TensorReshape:
    """Decode one TensorReshape."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    shape = destack._generated.program.vm.table.decode_u32_range_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorReshape(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        shape=shape,
        source_layout=source_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_reshape(value: TensorReshape) -> Json:
    """Return one JSON value for one TensorReshape."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "shape": destack._generated.program.vm.table.to_json_u32_range_id(value.shape),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_reshape(value: Json) -> TensorReshape:
    """Return one TensorReshape from one JSON value."""
    object_ = json_object(value)

    return TensorReshape(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        shape=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "shape")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorBroadcast:
    """Broadcast a tensor into a larger shape."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the broadcast dimension mapping
    dimensions: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_broadcast(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorBroadcast:
        """Decode one TensorBroadcast."""
        return decode_tensor_broadcast(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_broadcast(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorBroadcast:
        """Return one TensorBroadcast from one JSON value."""
        return from_json_tensor_broadcast(value)


def encode_tensor_broadcast(writer: BinaryWriter, value: TensorBroadcast) -> None:
    """Encode one TensorBroadcast."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.dimensions)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_broadcast(reader: BinaryReader) -> TensorBroadcast:
    """Decode one TensorBroadcast."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    dimensions = destack._generated.program.vm.table.decode_u32_range_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorBroadcast(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        dimensions=dimensions,
        source_layout=source_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_broadcast(value: TensorBroadcast) -> Json:
    """Return one JSON value for one TensorBroadcast."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "dimensions": destack._generated.program.vm.table.to_json_u32_range_id(
            value.dimensions
        ),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_broadcast(value: Json) -> TensorBroadcast:
    """Return one TensorBroadcast from one JSON value."""
    object_ = json_object(value)

    return TensorBroadcast(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        dimensions=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "dimensions")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorTranspose:
    """Permute tensor dimensions."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the dimension permutation
    permutation: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_transpose(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorTranspose:
        """Decode one TensorTranspose."""
        return decode_tensor_transpose(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_transpose(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorTranspose:
        """Return one TensorTranspose from one JSON value."""
        return from_json_tensor_transpose(value)


def encode_tensor_transpose(writer: BinaryWriter, value: TensorTranspose) -> None:
    """Encode one TensorTranspose."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.permutation)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_transpose(reader: BinaryReader) -> TensorTranspose:
    """Decode one TensorTranspose."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    permutation = destack._generated.program.vm.table.decode_u32_range_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorTranspose(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        permutation=permutation,
        source_layout=source_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_transpose(value: TensorTranspose) -> Json:
    """Return one JSON value for one TensorTranspose."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "permutation": destack._generated.program.vm.table.to_json_u32_range_id(
            value.permutation
        ),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_transpose(value: Json) -> TensorTranspose:
    """Return one TensorTranspose from one JSON value."""
    object_ = json_object(value)

    return TensorTranspose(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        permutation=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "permutation")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorSlice:
    """Slice a tensor by offsets, sizes, and strides."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the packed offset, size, and stride values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorSlice:
        """Decode one TensorSlice."""
        return decode_tensor_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorSlice:
        """Return one TensorSlice from one JSON value."""
        return from_json_tensor_slice(value)


def encode_tensor_slice(writer: BinaryWriter, value: TensorSlice) -> None:
    """Encode one TensorSlice."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.arguments)
    writer.write_unsigned(value.offsets_count)
    writer.write_unsigned(value.sizes_count)
    writer.write_unsigned(value.strides_count)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_slice(reader: BinaryReader) -> TensorSlice:
    """Decode one TensorSlice."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    arguments = destack._generated.program.vm.table.decode_u32_range_id(reader)
    offsets_count = reader.read_number()
    sizes_count = reader.read_number()
    strides_count = reader.read_number()
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorSlice(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        arguments=arguments,
        offsets_count=offsets_count,
        sizes_count=sizes_count,
        strides_count=strides_count,
        source_layout=source_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_slice(value: TensorSlice) -> Json:
    """Return one JSON value for one TensorSlice."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "arguments": destack._generated.program.vm.table.to_json_u32_range_id(
            value.arguments
        ),
        "offsetsCount": value.offsets_count,
        "sizesCount": value.sizes_count,
        "stridesCount": value.strides_count,
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_slice(value: Json) -> TensorSlice:
    """Return one TensorSlice from one JSON value."""
    object_ = json_object(value)

    return TensorSlice(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        arguments=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "arguments")
        ),
        offsets_count=json_int(json_field(object_, "offsetsCount")),
        sizes_count=json_int(json_field(object_, "sizesCount")),
        strides_count=json_int(json_field(object_, "stridesCount")),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorPad:
    """Pad a tensor with low, high, and interior padding."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the packed low, high, and interior padding values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of low padding values
    low_count: int
    # the number of high padding values
    high_count: int
    # the number of interior padding values
    interior_count: int
    # the padding value cell offset
    value_offset: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_pad(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorPad:
        """Decode one TensorPad."""
        return decode_tensor_pad(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_pad(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorPad:
        """Return one TensorPad from one JSON value."""
        return from_json_tensor_pad(value)


def encode_tensor_pad(writer: BinaryWriter, value: TensorPad) -> None:
    """Encode one TensorPad."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.arguments)
    writer.write_unsigned(value.low_count)
    writer.write_unsigned(value.high_count)
    writer.write_unsigned(value.interior_count)
    writer.write_unsigned(value.value_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_pad(reader: BinaryReader) -> TensorPad:
    """Decode one TensorPad."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    arguments = destack._generated.program.vm.table.decode_u32_range_id(reader)
    low_count = reader.read_number()
    high_count = reader.read_number()
    interior_count = reader.read_number()
    value_offset = reader.read_number()
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorPad(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        arguments=arguments,
        low_count=low_count,
        high_count=high_count,
        interior_count=interior_count,
        value_offset=value_offset,
        source_layout=source_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_pad(value: TensorPad) -> Json:
    """Return one JSON value for one TensorPad."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "arguments": destack._generated.program.vm.table.to_json_u32_range_id(
            value.arguments
        ),
        "lowCount": value.low_count,
        "highCount": value.high_count,
        "interiorCount": value.interior_count,
        "valueOffset": value.value_offset,
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_pad(value: Json) -> TensorPad:
    """Return one TensorPad from one JSON value."""
    object_ = json_object(value)

    return TensorPad(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        arguments=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "arguments")
        ),
        low_count=json_int(json_field(object_, "lowCount")),
        high_count=json_int(json_field(object_, "highCount")),
        interior_count=json_int(json_field(object_, "interiorCount")),
        value_offset=json_int(json_field(object_, "valueOffset")),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorConcat:
    """Concatenate tensors along a dimension."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offsets
    tensors: destack._generated.program.vm.table.U32RangeId
    # the source tensor layouts
    tensor_layouts: destack._generated.program.vm.table.U32RangeId
    # the concatenation axis
    axis: int
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_concat(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConcat:
        """Decode one TensorConcat."""
        return decode_tensor_concat(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_concat(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorConcat:
        """Return one TensorConcat from one JSON value."""
        return from_json_tensor_concat(value)


def encode_tensor_concat(writer: BinaryWriter, value: TensorConcat) -> None:
    """Encode one TensorConcat."""
    writer.write_unsigned(value.dest_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.tensors)
    destack._generated.program.vm.table.encode_u32_range_id(
        writer, value.tensor_layouts
    )
    writer.write_unsigned(value.axis)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_concat(reader: BinaryReader) -> TensorConcat:
    """Decode one TensorConcat."""
    dest_offset = reader.read_number()
    tensors = destack._generated.program.vm.table.decode_u32_range_id(reader)
    tensor_layouts = destack._generated.program.vm.table.decode_u32_range_id(reader)
    axis = reader.read_number()
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorConcat(
        dest_offset=dest_offset,
        tensors=tensors,
        tensor_layouts=tensor_layouts,
        axis=axis,
        dest_layout=dest_layout,
    )


def to_json_tensor_concat(value: TensorConcat) -> Json:
    """Return one JSON value for one TensorConcat."""
    return {
        "destOffset": value.dest_offset,
        "tensors": destack._generated.program.vm.table.to_json_u32_range_id(
            value.tensors
        ),
        "tensorLayouts": destack._generated.program.vm.table.to_json_u32_range_id(
            value.tensor_layouts
        ),
        "axis": value.axis,
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_concat(value: Json) -> TensorConcat:
    """Return one TensorConcat from one JSON value."""
    object_ = json_object(value)

    return TensorConcat(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensors=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "tensors")
        ),
        tensor_layouts=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "tensorLayouts")
        ),
        axis=json_int(json_field(object_, "axis")),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorReduce:
    """Reduce a tensor along axes."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the initial value cell offset
    initial_offset: int
    # the reduced axis range
    axes: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the reduction kernel
    kernel: destack._generated.mir.tree.tensor.TensorReduceOperator

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_reduce(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorReduce:
        """Decode one TensorReduce."""
        return decode_tensor_reduce(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_reduce(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorReduce:
        """Return one TensorReduce from one JSON value."""
        return from_json_tensor_reduce(value)


def encode_tensor_reduce(writer: BinaryWriter, value: TensorReduce) -> None:
    """Encode one TensorReduce."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    writer.write_unsigned(value.initial_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.axes)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.mir.tree.tensor.encode_tensor_reduce_operator(
        writer, value.kernel
    )


def decode_tensor_reduce(reader: BinaryReader) -> TensorReduce:
    """Decode one TensorReduce."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    initial_offset = reader.read_number()
    axes = destack._generated.program.vm.table.decode_u32_range_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    kernel = destack._generated.mir.tree.tensor.decode_tensor_reduce_operator(reader)

    return TensorReduce(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        initial_offset=initial_offset,
        axes=axes,
        source_layout=source_layout,
        dest_layout=dest_layout,
        kernel=kernel,
    )


def to_json_tensor_reduce(value: TensorReduce) -> Json:
    """Return one JSON value for one TensorReduce."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "initialOffset": value.initial_offset,
        "axes": destack._generated.program.vm.table.to_json_u32_range_id(value.axes),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "kernel": destack._generated.mir.tree.tensor.to_json_tensor_reduce_operator(
            value.kernel
        ),
    }


def from_json_tensor_reduce(value: Json) -> TensorReduce:
    """Return one TensorReduce from one JSON value."""
    object_ = json_object(value)

    return TensorReduce(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        initial_offset=json_int(json_field(object_, "initialOffset")),
        axes=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "axes")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        kernel=destack._generated.mir.tree.tensor.from_json_tensor_reduce_operator(
            json_field(object_, "kernel")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorIndexReduce:
    """Reduce a tensor along one axis and return selected source indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the reduced axis
    axis: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the index reduction kernel
    kernel: destack._generated.mir.tree.tensor.TensorIndexReduceOperator
    # the behavior for equal selected values
    tie_break: destack._generated.mir.tree.tensor.TensorIndexTieBreak

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_index_reduce(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorIndexReduce:
        """Decode one TensorIndexReduce."""
        return decode_tensor_index_reduce(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_index_reduce(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorIndexReduce:
        """Return one TensorIndexReduce from one JSON value."""
        return from_json_tensor_index_reduce(value)


def encode_tensor_index_reduce(writer: BinaryWriter, value: TensorIndexReduce) -> None:
    """Encode one TensorIndexReduce."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    writer.write_unsigned(value.axis)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.mir.tree.tensor.encode_tensor_index_reduce_operator(
        writer, value.kernel
    )
    destack._generated.mir.tree.tensor.encode_tensor_index_tie_break(
        writer, value.tie_break
    )


def decode_tensor_index_reduce(reader: BinaryReader) -> TensorIndexReduce:
    """Decode one TensorIndexReduce."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    axis = reader.read_number()
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    kernel = destack._generated.mir.tree.tensor.decode_tensor_index_reduce_operator(
        reader
    )
    tie_break = destack._generated.mir.tree.tensor.decode_tensor_index_tie_break(reader)

    return TensorIndexReduce(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        axis=axis,
        source_layout=source_layout,
        dest_layout=dest_layout,
        kernel=kernel,
        tie_break=tie_break,
    )


def to_json_tensor_index_reduce(value: TensorIndexReduce) -> Json:
    """Return one JSON value for one TensorIndexReduce."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "axis": value.axis,
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "kernel": destack._generated.mir.tree.tensor.to_json_tensor_index_reduce_operator(
            value.kernel
        ),
        "tieBreak": destack._generated.mir.tree.tensor.to_json_tensor_index_tie_break(
            value.tie_break
        ),
    }


def from_json_tensor_index_reduce(value: Json) -> TensorIndexReduce:
    """Return one TensorIndexReduce from one JSON value."""
    object_ = json_object(value)

    return TensorIndexReduce(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        axis=json_int(json_field(object_, "axis")),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        kernel=destack._generated.mir.tree.tensor.from_json_tensor_index_reduce_operator(
            json_field(object_, "kernel")
        ),
        tie_break=destack._generated.mir.tree.tensor.from_json_tensor_index_tie_break(
            json_field(object_, "tieBreak")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorDot:
    """Dot product of two tensors."""

    # the destination tensor frame offset
    dest_offset: int
    # the left tensor frame offset
    left_offset: int
    # the right tensor frame offset
    right_offset: int
    # the dot dimension numbers
    dimensions: destack._generated.program.vm.table.TensorDotId
    # the left tensor layout
    left_layout: destack._generated.program.vm.table.TensorLayoutId
    # the right tensor layout
    right_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_dot(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorDot:
        """Decode one TensorDot."""
        return decode_tensor_dot(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_dot(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorDot:
        """Return one TensorDot from one JSON value."""
        return from_json_tensor_dot(value)


def encode_tensor_dot(writer: BinaryWriter, value: TensorDot) -> None:
    """Encode one TensorDot."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.left_offset)
    writer.write_unsigned(value.right_offset)
    destack._generated.program.vm.table.encode_tensor_dot_id(writer, value.dimensions)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.left_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.right_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )


def decode_tensor_dot(reader: BinaryReader) -> TensorDot:
    """Decode one TensorDot."""
    dest_offset = reader.read_number()
    left_offset = reader.read_number()
    right_offset = reader.read_number()
    dimensions = destack._generated.program.vm.table.decode_tensor_dot_id(reader)
    left_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    right_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)

    return TensorDot(
        dest_offset=dest_offset,
        left_offset=left_offset,
        right_offset=right_offset,
        dimensions=dimensions,
        left_layout=left_layout,
        right_layout=right_layout,
        dest_layout=dest_layout,
        element_layout=element_layout,
    )


def to_json_tensor_dot(value: TensorDot) -> Json:
    """Return one JSON value for one TensorDot."""
    return {
        "destOffset": value.dest_offset,
        "leftOffset": value.left_offset,
        "rightOffset": value.right_offset,
        "dimensions": destack._generated.program.vm.table.to_json_tensor_dot_id(
            value.dimensions
        ),
        "leftLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.left_layout
        ),
        "rightLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.right_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
    }


def from_json_tensor_dot(value: Json) -> TensorDot:
    """Return one TensorDot from one JSON value."""
    object_ = json_object(value)

    return TensorDot(
        dest_offset=json_int(json_field(object_, "destOffset")),
        left_offset=json_int(json_field(object_, "leftOffset")),
        right_offset=json_int(json_field(object_, "rightOffset")),
        dimensions=destack._generated.program.vm.table.from_json_tensor_dot_id(
            json_field(object_, "dimensions")
        ),
        left_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "leftLayout")
        ),
        right_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "rightLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorConvolution:
    """Convolution between an input tensor and a kernel tensor."""

    # the destination tensor frame offset
    dest_offset: int
    # the input tensor frame offset
    input_offset: int
    # the kernel tensor frame offset
    kernel_offset: int
    # the convolution dimension numbers
    dimensions: destack._generated.program.vm.table.TensorConvolutionId
    # the convolution window descriptor
    window: destack._generated.program.vm.table.TensorWindowId
    # the feature group count
    feature_group_count: int
    # the batch group count
    batch_group_count: int
    # the input tensor layout
    input_layout: destack._generated.program.vm.table.TensorLayoutId
    # the kernel tensor layout
    kernel_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_convolution(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvolution:
        """Decode one TensorConvolution."""
        return decode_tensor_convolution(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_convolution(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorConvolution:
        """Return one TensorConvolution from one JSON value."""
        return from_json_tensor_convolution(value)


def encode_tensor_convolution(writer: BinaryWriter, value: TensorConvolution) -> None:
    """Encode one TensorConvolution."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.input_offset)
    writer.write_unsigned(value.kernel_offset)
    destack._generated.program.vm.table.encode_tensor_convolution_id(
        writer, value.dimensions
    )
    destack._generated.program.vm.table.encode_tensor_window_id(writer, value.window)
    writer.write_unsigned(value.feature_group_count)
    writer.write_unsigned(value.batch_group_count)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.input_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.kernel_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )


def decode_tensor_convolution(reader: BinaryReader) -> TensorConvolution:
    """Decode one TensorConvolution."""
    dest_offset = reader.read_number()
    input_offset = reader.read_number()
    kernel_offset = reader.read_number()
    dimensions = destack._generated.program.vm.table.decode_tensor_convolution_id(
        reader
    )
    window = destack._generated.program.vm.table.decode_tensor_window_id(reader)
    feature_group_count = reader.read_number()
    batch_group_count = reader.read_number()
    input_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    kernel_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)

    return TensorConvolution(
        dest_offset=dest_offset,
        input_offset=input_offset,
        kernel_offset=kernel_offset,
        dimensions=dimensions,
        window=window,
        feature_group_count=feature_group_count,
        batch_group_count=batch_group_count,
        input_layout=input_layout,
        kernel_layout=kernel_layout,
        dest_layout=dest_layout,
        element_layout=element_layout,
    )


def to_json_tensor_convolution(value: TensorConvolution) -> Json:
    """Return one JSON value for one TensorConvolution."""
    return {
        "destOffset": value.dest_offset,
        "inputOffset": value.input_offset,
        "kernelOffset": value.kernel_offset,
        "dimensions": destack._generated.program.vm.table.to_json_tensor_convolution_id(
            value.dimensions
        ),
        "window": destack._generated.program.vm.table.to_json_tensor_window_id(
            value.window
        ),
        "featureGroupCount": value.feature_group_count,
        "batchGroupCount": value.batch_group_count,
        "inputLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.input_layout
        ),
        "kernelLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.kernel_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
    }


def from_json_tensor_convolution(value: Json) -> TensorConvolution:
    """Return one TensorConvolution from one JSON value."""
    object_ = json_object(value)

    return TensorConvolution(
        dest_offset=json_int(json_field(object_, "destOffset")),
        input_offset=json_int(json_field(object_, "inputOffset")),
        kernel_offset=json_int(json_field(object_, "kernelOffset")),
        dimensions=destack._generated.program.vm.table.from_json_tensor_convolution_id(
            json_field(object_, "dimensions")
        ),
        window=destack._generated.program.vm.table.from_json_tensor_window_id(
            json_field(object_, "window")
        ),
        feature_group_count=json_int(json_field(object_, "featureGroupCount")),
        batch_group_count=json_int(json_field(object_, "batchGroupCount")),
        input_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "inputLayout")
        ),
        kernel_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "kernelLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorGather:
    """Gather slices from a tensor based on indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    source_offset: int
    # the indices tensor frame offset
    indices_offset: int
    # the gather dimension numbers
    dimensions: destack._generated.program.vm.table.TensorGatherId
    # the gathered slice sizes
    slice_sizes: destack._generated.program.vm.table.U32RangeId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the indices tensor layout
    indices_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_gather(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorGather:
        """Decode one TensorGather."""
        return decode_tensor_gather(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_gather(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorGather:
        """Return one TensorGather from one JSON value."""
        return from_json_tensor_gather(value)


def encode_tensor_gather(writer: BinaryWriter, value: TensorGather) -> None:
    """Encode one TensorGather."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.source_offset)
    writer.write_unsigned(value.indices_offset)
    destack._generated.program.vm.table.encode_tensor_gather_id(
        writer, value.dimensions
    )
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.slice_sizes)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.indices_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_gather(reader: BinaryReader) -> TensorGather:
    """Decode one TensorGather."""
    dest_offset = reader.read_number()
    source_offset = reader.read_number()
    indices_offset = reader.read_number()
    dimensions = destack._generated.program.vm.table.decode_tensor_gather_id(reader)
    slice_sizes = destack._generated.program.vm.table.decode_u32_range_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    indices_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorGather(
        dest_offset=dest_offset,
        source_offset=source_offset,
        indices_offset=indices_offset,
        dimensions=dimensions,
        slice_sizes=slice_sizes,
        source_layout=source_layout,
        indices_layout=indices_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_gather(value: TensorGather) -> Json:
    """Return one JSON value for one TensorGather."""
    return {
        "destOffset": value.dest_offset,
        "sourceOffset": value.source_offset,
        "indicesOffset": value.indices_offset,
        "dimensions": destack._generated.program.vm.table.to_json_tensor_gather_id(
            value.dimensions
        ),
        "sliceSizes": destack._generated.program.vm.table.to_json_u32_range_id(
            value.slice_sizes
        ),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "indicesLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.indices_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_gather(value: Json) -> TensorGather:
    """Return one TensorGather from one JSON value."""
    object_ = json_object(value)

    return TensorGather(
        dest_offset=json_int(json_field(object_, "destOffset")),
        source_offset=json_int(json_field(object_, "sourceOffset")),
        indices_offset=json_int(json_field(object_, "indicesOffset")),
        dimensions=destack._generated.program.vm.table.from_json_tensor_gather_id(
            json_field(object_, "dimensions")
        ),
        slice_sizes=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "sliceSizes")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        indices_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "indicesLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorScatter:
    """Scatter updates into a tensor based on indices."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    source_offset: int
    # the indices tensor frame offset
    indices_offset: int
    # the updates tensor frame offset
    updates_offset: int
    # the scatter dimension numbers
    dimensions: destack._generated.program.vm.table.TensorScatterId
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the indices tensor layout
    indices_layout: destack._generated.program.vm.table.TensorLayoutId
    # the updates tensor layout
    updates_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element scalar layout
    element_layout: destack._generated.program.vm.value.ScalarLayout
    # the scatter kernel
    mode: destack._generated.mir.tree.tensor.TensorScatterMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_scatter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorScatter:
        """Decode one TensorScatter."""
        return decode_tensor_scatter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_scatter(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorScatter:
        """Return one TensorScatter from one JSON value."""
        return from_json_tensor_scatter(value)


def encode_tensor_scatter(writer: BinaryWriter, value: TensorScatter) -> None:
    """Encode one TensorScatter."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.source_offset)
    writer.write_unsigned(value.indices_offset)
    writer.write_unsigned(value.updates_offset)
    destack._generated.program.vm.table.encode_tensor_scatter_id(
        writer, value.dimensions
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.indices_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.updates_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.element_layout
    )
    destack._generated.mir.tree.tensor.encode_tensor_scatter_mode(writer, value.mode)


def decode_tensor_scatter(reader: BinaryReader) -> TensorScatter:
    """Decode one TensorScatter."""
    dest_offset = reader.read_number()
    source_offset = reader.read_number()
    indices_offset = reader.read_number()
    updates_offset = reader.read_number()
    dimensions = destack._generated.program.vm.table.decode_tensor_scatter_id(reader)
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    indices_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    updates_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element_layout = destack._generated.program.vm.value.decode_scalar_layout(reader)
    mode = destack._generated.mir.tree.tensor.decode_tensor_scatter_mode(reader)

    return TensorScatter(
        dest_offset=dest_offset,
        source_offset=source_offset,
        indices_offset=indices_offset,
        updates_offset=updates_offset,
        dimensions=dimensions,
        source_layout=source_layout,
        indices_layout=indices_layout,
        updates_layout=updates_layout,
        dest_layout=dest_layout,
        element_layout=element_layout,
        mode=mode,
    )


def to_json_tensor_scatter(value: TensorScatter) -> Json:
    """Return one JSON value for one TensorScatter."""
    return {
        "destOffset": value.dest_offset,
        "sourceOffset": value.source_offset,
        "indicesOffset": value.indices_offset,
        "updatesOffset": value.updates_offset,
        "dimensions": destack._generated.program.vm.table.to_json_tensor_scatter_id(
            value.dimensions
        ),
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "indicesLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.indices_layout
        ),
        "updatesLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.updates_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "elementLayout": destack._generated.program.vm.value.to_json_scalar_layout(
            value.element_layout
        ),
        "mode": destack._generated.mir.tree.tensor.to_json_tensor_scatter_mode(
            value.mode
        ),
    }


def from_json_tensor_scatter(value: Json) -> TensorScatter:
    """Return one TensorScatter from one JSON value."""
    object_ = json_object(value)

    return TensorScatter(
        dest_offset=json_int(json_field(object_, "destOffset")),
        source_offset=json_int(json_field(object_, "sourceOffset")),
        indices_offset=json_int(json_field(object_, "indicesOffset")),
        updates_offset=json_int(json_field(object_, "updatesOffset")),
        dimensions=destack._generated.program.vm.table.from_json_tensor_scatter_id(
            json_field(object_, "dimensions")
        ),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        indices_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "indicesLayout")
        ),
        updates_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "updatesLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element_layout=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "elementLayout")
        ),
        mode=destack._generated.mir.tree.tensor.from_json_tensor_scatter_mode(
            json_field(object_, "mode")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorSelect:
    """Select tensor elements based on a boolean mask."""

    # the destination tensor frame offset
    dest_offset: int
    # the mask tensor frame offset
    mask_offset: int
    # the true branch tensor frame offset
    then_offset: int
    # the false branch tensor frame offset
    else_offset: int
    # the mask tensor layout
    mask_layout: destack._generated.program.vm.table.TensorLayoutId
    # the true branch tensor layout
    then_layout: destack._generated.program.vm.table.TensorLayoutId
    # the false branch tensor layout
    else_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_select(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorSelect:
        """Decode one TensorSelect."""
        return decode_tensor_select(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_select(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorSelect:
        """Return one TensorSelect from one JSON value."""
        return from_json_tensor_select(value)


def encode_tensor_select(writer: BinaryWriter, value: TensorSelect) -> None:
    """Encode one TensorSelect."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.mask_offset)
    writer.write_unsigned(value.then_offset)
    writer.write_unsigned(value.else_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.mask_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.then_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.else_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )


def decode_tensor_select(reader: BinaryReader) -> TensorSelect:
    """Decode one TensorSelect."""
    dest_offset = reader.read_number()
    mask_offset = reader.read_number()
    then_offset = reader.read_number()
    else_offset = reader.read_number()
    mask_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    then_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    else_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)

    return TensorSelect(
        dest_offset=dest_offset,
        mask_offset=mask_offset,
        then_offset=then_offset,
        else_offset=else_offset,
        mask_layout=mask_layout,
        then_layout=then_layout,
        else_layout=else_layout,
        dest_layout=dest_layout,
    )


def to_json_tensor_select(value: TensorSelect) -> Json:
    """Return one JSON value for one TensorSelect."""
    return {
        "destOffset": value.dest_offset,
        "maskOffset": value.mask_offset,
        "thenOffset": value.then_offset,
        "elseOffset": value.else_offset,
        "maskLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.mask_layout
        ),
        "thenLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.then_layout
        ),
        "elseLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.else_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
    }


def from_json_tensor_select(value: Json) -> TensorSelect:
    """Return one TensorSelect from one JSON value."""
    object_ = json_object(value)

    return TensorSelect(
        dest_offset=json_int(json_field(object_, "destOffset")),
        mask_offset=json_int(json_field(object_, "maskOffset")),
        then_offset=json_int(json_field(object_, "thenOffset")),
        else_offset=json_int(json_field(object_, "elseOffset")),
        mask_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "maskLayout")
        ),
        then_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "thenLayout")
        ),
        else_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "elseLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorConvert:
    """Convert a tensor element type."""

    # the destination tensor frame offset
    dest_offset: int
    # the source tensor frame offset
    tensor_offset: int
    # the source tensor layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the source tensor scalar layout
    source_scalar: destack._generated.program.vm.value.ScalarLayout
    # the destination tensor scalar layout
    dest_scalar: destack._generated.program.vm.value.ScalarLayout
    # the conversion kernel
    mode: destack._generated.mir.tree.tensor.TensorConvertMode

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_convert(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorConvert:
        """Decode one TensorConvert."""
        return decode_tensor_convert(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_convert(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorConvert:
        """Return one TensorConvert from one JSON value."""
        return from_json_tensor_convert(value)


def encode_tensor_convert(writer: BinaryWriter, value: TensorConvert) -> None:
    """Encode one TensorConvert."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.tensor_offset)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.value.encode_scalar_layout(
        writer, value.source_scalar
    )
    destack._generated.program.vm.value.encode_scalar_layout(writer, value.dest_scalar)
    destack._generated.mir.tree.tensor.encode_tensor_convert_mode(writer, value.mode)


def decode_tensor_convert(reader: BinaryReader) -> TensorConvert:
    """Decode one TensorConvert."""
    dest_offset = reader.read_number()
    tensor_offset = reader.read_number()
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    source_scalar = destack._generated.program.vm.value.decode_scalar_layout(reader)
    dest_scalar = destack._generated.program.vm.value.decode_scalar_layout(reader)
    mode = destack._generated.mir.tree.tensor.decode_tensor_convert_mode(reader)

    return TensorConvert(
        dest_offset=dest_offset,
        tensor_offset=tensor_offset,
        source_layout=source_layout,
        dest_layout=dest_layout,
        source_scalar=source_scalar,
        dest_scalar=dest_scalar,
        mode=mode,
    )


def to_json_tensor_convert(value: TensorConvert) -> Json:
    """Return one JSON value for one TensorConvert."""
    return {
        "destOffset": value.dest_offset,
        "tensorOffset": value.tensor_offset,
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "sourceScalar": destack._generated.program.vm.value.to_json_scalar_layout(
            value.source_scalar
        ),
        "destScalar": destack._generated.program.vm.value.to_json_scalar_layout(
            value.dest_scalar
        ),
        "mode": destack._generated.mir.tree.tensor.to_json_tensor_convert_mode(
            value.mode
        ),
    }


def from_json_tensor_convert(value: Json) -> TensorConvert:
    """Return one TensorConvert from one JSON value."""
    object_ = json_object(value)

    return TensorConvert(
        dest_offset=json_int(json_field(object_, "destOffset")),
        tensor_offset=json_int(json_field(object_, "tensorOffset")),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        source_scalar=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "sourceScalar")
        ),
        dest_scalar=destack._generated.program.vm.value.from_json_scalar_layout(
            json_field(object_, "destScalar")
        ),
        mode=destack._generated.mir.tree.tensor.from_json_tensor_convert_mode(
            json_field(object_, "mode")
        ),
    )


@dataclass(frozen=True, slots=True)
class TensorView:
    """Create a view into a tensor reference."""

    # the destination tensor view
    dest_offset: int
    # the source tensor view
    view_offset: int
    # the offset, size, and stride values
    arguments: destack._generated.program.vm.table.U32RangeId
    # the number of offset values
    offsets_count: int
    # the number of size values
    sizes_count: int
    # the number of stride values
    strides_count: int
    # the source tensor view layout
    source_layout: destack._generated.program.vm.table.TensorLayoutId
    # the destination tensor view layout
    dest_layout: destack._generated.program.vm.table.TensorLayoutId
    # the tensor element projection
    element: destack._generated.program.vm.table.ProjectionId
    # the tensor view backing memory
    address: destack._generated.program.vm.tensor.TensorAddress

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tensor_view(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TensorView:
        """Decode one TensorView."""
        return decode_tensor_view(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tensor_view(self)

    @classmethod
    def from_json(cls, value: Json) -> TensorView:
        """Return one TensorView from one JSON value."""
        return from_json_tensor_view(value)


def encode_tensor_view(writer: BinaryWriter, value: TensorView) -> None:
    """Encode one TensorView."""
    writer.write_unsigned(value.dest_offset)
    writer.write_unsigned(value.view_offset)
    destack._generated.program.vm.table.encode_u32_range_id(writer, value.arguments)
    writer.write_unsigned(value.offsets_count)
    writer.write_unsigned(value.sizes_count)
    writer.write_unsigned(value.strides_count)
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.source_layout
    )
    destack._generated.program.vm.table.encode_tensor_layout_id(
        writer, value.dest_layout
    )
    destack._generated.program.vm.table.encode_projection_id(writer, value.element)
    destack._generated.program.vm.tensor.encode_tensor_address(writer, value.address)


def decode_tensor_view(reader: BinaryReader) -> TensorView:
    """Decode one TensorView."""
    dest_offset = reader.read_number()
    view_offset = reader.read_number()
    arguments = destack._generated.program.vm.table.decode_u32_range_id(reader)
    offsets_count = reader.read_number()
    sizes_count = reader.read_number()
    strides_count = reader.read_number()
    source_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    dest_layout = destack._generated.program.vm.table.decode_tensor_layout_id(reader)
    element = destack._generated.program.vm.table.decode_projection_id(reader)
    address = destack._generated.program.vm.tensor.decode_tensor_address(reader)

    return TensorView(
        dest_offset=dest_offset,
        view_offset=view_offset,
        arguments=arguments,
        offsets_count=offsets_count,
        sizes_count=sizes_count,
        strides_count=strides_count,
        source_layout=source_layout,
        dest_layout=dest_layout,
        element=element,
        address=address,
    )


def to_json_tensor_view(value: TensorView) -> Json:
    """Return one JSON value for one TensorView."""
    return {
        "destOffset": value.dest_offset,
        "viewOffset": value.view_offset,
        "arguments": destack._generated.program.vm.table.to_json_u32_range_id(
            value.arguments
        ),
        "offsetsCount": value.offsets_count,
        "sizesCount": value.sizes_count,
        "stridesCount": value.strides_count,
        "sourceLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.source_layout
        ),
        "destLayout": destack._generated.program.vm.table.to_json_tensor_layout_id(
            value.dest_layout
        ),
        "element": destack._generated.program.vm.table.to_json_projection_id(
            value.element
        ),
        "address": destack._generated.program.vm.tensor.to_json_tensor_address(
            value.address
        ),
    }


def from_json_tensor_view(value: Json) -> TensorView:
    """Return one TensorView from one JSON value."""
    object_ = json_object(value)

    return TensorView(
        dest_offset=json_int(json_field(object_, "destOffset")),
        view_offset=json_int(json_field(object_, "viewOffset")),
        arguments=destack._generated.program.vm.table.from_json_u32_range_id(
            json_field(object_, "arguments")
        ),
        offsets_count=json_int(json_field(object_, "offsetsCount")),
        sizes_count=json_int(json_field(object_, "sizesCount")),
        strides_count=json_int(json_field(object_, "stridesCount")),
        source_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "sourceLayout")
        ),
        dest_layout=destack._generated.program.vm.table.from_json_tensor_layout_id(
            json_field(object_, "destLayout")
        ),
        element=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "element")
        ),
        address=destack._generated.program.vm.tensor.from_json_tensor_address(
            json_field(object_, "address")
        ),
    )


@dataclass(frozen=True, slots=True)
class Intrinsic:
    """Intrinsic call."""

    # the intrinsic kernel
    kernel: destack._generated.mir.tree.intrinsic.Intrinsic
    # the destination shape
    dest: IntrinsicDest
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange
    # the lowered shape for each argument
    layouts: tuple[
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
        destack._generated.program.vm.value.ValueShape,
    ]
    # the argument count
    layout_count: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intrinsic(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Intrinsic:
        """Decode one Intrinsic."""
        return decode_intrinsic(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intrinsic(self)

    @classmethod
    def from_json(cls, value: Json) -> Intrinsic:
        """Return one Intrinsic from one JSON value."""
        return from_json_intrinsic(value)


def encode_intrinsic(writer: BinaryWriter, value: Intrinsic) -> None:
    """Encode one Intrinsic."""
    destack._generated.mir.tree.intrinsic.encode_intrinsic(writer, value.kernel)
    encode_intrinsic_dest(writer, value.dest)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)
    writer.write_unsigned(len(value.layouts))
    for item_value_layouts_0 in value.layouts:
        destack._generated.program.vm.value.encode_value_shape(
            writer, item_value_layouts_0
        )
    writer.write_byte(value.layout_count)


def decode_intrinsic(reader: BinaryReader) -> Intrinsic:
    """Decode one Intrinsic."""
    kernel = destack._generated.mir.tree.intrinsic.decode_intrinsic(reader)
    dest = decode_intrinsic_dest(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)
    layouts = (
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
        destack._generated.program.vm.value.decode_value_shape(reader),
    )
    layout_count = reader.read_byte()

    return Intrinsic(
        kernel=kernel,
        dest=dest,
        arguments=arguments,
        layouts=layouts,
        layout_count=layout_count,
    )


def to_json_intrinsic(value: Intrinsic) -> Json:
    """Return one JSON value for one Intrinsic."""
    return {
        "kernel": destack._generated.mir.tree.intrinsic.to_json_intrinsic(value.kernel),
        "dest": to_json_intrinsic_dest(value.dest),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
        "layouts": [
            destack._generated.program.vm.value.to_json_value_shape(item_0)
            for item_0 in value.layouts
        ],
        "layoutCount": value.layout_count,
    }


def from_json_intrinsic(value: Json) -> Intrinsic:
    """Return one Intrinsic from one JSON value."""
    object_ = json_object(value)

    return Intrinsic(
        kernel=destack._generated.mir.tree.intrinsic.from_json_intrinsic(
            json_field(object_, "kernel")
        ),
        dest=from_json_intrinsic_dest(json_field(object_, "dest")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
        layouts=(
            lambda items: (
                destack._generated.program.vm.value.from_json_value_shape(items[0]),
                destack._generated.program.vm.value.from_json_value_shape(items[1]),
                destack._generated.program.vm.value.from_json_value_shape(items[2]),
                destack._generated.program.vm.value.from_json_value_shape(items[3]),
                destack._generated.program.vm.value.from_json_value_shape(items[4]),
                destack._generated.program.vm.value.from_json_value_shape(items[5]),
                destack._generated.program.vm.value.from_json_value_shape(items[6]),
                destack._generated.program.vm.value.from_json_value_shape(items[7]),
                destack._generated.program.vm.value.from_json_value_shape(items[8]),
                destack._generated.program.vm.value.from_json_value_shape(items[9]),
                destack._generated.program.vm.value.from_json_value_shape(items[10]),
                destack._generated.program.vm.value.from_json_value_shape(items[11]),
                destack._generated.program.vm.value.from_json_value_shape(items[12]),
                destack._generated.program.vm.value.from_json_value_shape(items[13]),
                destack._generated.program.vm.value.from_json_value_shape(items[14]),
                destack._generated.program.vm.value.from_json_value_shape(items[15]),
            )
        )(json_array_length(json_field(object_, "layouts"), 16)),
        layout_count=json_int(json_field(object_, "layoutCount")),
    )


@dataclass(frozen=True, slots=True)
class IntrinsicDestNone:
    """No destination."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intrinsic_dest(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intrinsic_dest(self)


@dataclass(frozen=True, slots=True)
class IntrinsicDestCell:
    """Cell destination frame offset."""

    cell: int
    kind: typing.Literal["cell"] = "cell"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intrinsic_dest(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intrinsic_dest(self)


@dataclass(frozen=True, slots=True)
class IntrinsicDestFrame:
    """Frame destination value."""

    frame: destack._generated.mir.tree.value.Value
    kind: typing.Literal["frame"] = "frame"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intrinsic_dest(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intrinsic_dest(self)


"""Intrinsic destination."""
IntrinsicDest: typing.TypeAlias = (
    IntrinsicDestNone | IntrinsicDestCell | IntrinsicDestFrame
)


def encode_intrinsic_dest(writer: BinaryWriter, value: IntrinsicDest) -> None:
    """Encode one IntrinsicDest."""
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "cell":
        writer.write_unsigned(1)
        writer.write_unsigned(value.cell)
    elif value.kind == "frame":
        writer.write_unsigned(2)
        destack._generated.mir.tree.value.encode_value(writer, value.frame)
    else:
        raise SerdeError("unknown enum variant")


def decode_intrinsic_dest(reader: BinaryReader) -> IntrinsicDest:
    """Decode one IntrinsicDest."""
    variant = reader.read_number()

    if variant == 0:
        return IntrinsicDestNone()
    elif variant == 1:
        cell = reader.read_number()

        return IntrinsicDestCell(cell=cell)
    elif variant == 2:
        frame = destack._generated.mir.tree.value.decode_value(reader)

        return IntrinsicDestFrame(frame=frame)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_intrinsic_dest(value: IntrinsicDest) -> Json:
    """Return one JSON value for one IntrinsicDest."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "cell":
        return {
            "kind": "cell",
            "cell": value.cell,
        }
    elif value.kind == "frame":
        return {
            "kind": "frame",
            "frame": destack._generated.mir.tree.value.to_json_value(value.frame),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_intrinsic_dest(value: Json) -> IntrinsicDest:
    """Return one IntrinsicDest from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return IntrinsicDestNone()
    elif kind == "cell":
        return IntrinsicDestCell(cell=json_int(json_field(object_, "cell")))
    elif kind == "frame":
        return IntrinsicDestFrame(
            frame=destack._generated.mir.tree.value.from_json_value(
                json_field(object_, "frame")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TailCall:
    """Tail call to a function."""

    # the callee function index
    function: int
    # the resolved call target
    target: destack._generated.program.vm.function.CallTarget
    # the argument frame moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tail_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCall:
        """Decode one TailCall."""
        return decode_tail_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tail_call(self)

    @classmethod
    def from_json(cls, value: Json) -> TailCall:
        """Return one TailCall from one JSON value."""
        return from_json_tail_call(value)


def encode_tail_call(writer: BinaryWriter, value: TailCall) -> None:
    """Encode one TailCall."""
    writer.write_unsigned(value.function)
    destack._generated.program.vm.function.encode_call_target(writer, value.target)
    destack._generated.program.vm.range.encode_move_range(writer, value.moves)


def decode_tail_call(reader: BinaryReader) -> TailCall:
    """Decode one TailCall."""
    function = reader.read_number()
    target = destack._generated.program.vm.function.decode_call_target(reader)
    moves = destack._generated.program.vm.range.decode_move_range(reader)

    return TailCall(
        function=function,
        target=target,
        moves=moves,
    )


def to_json_tail_call(value: TailCall) -> Json:
    """Return one JSON value for one TailCall."""
    return {
        "function": value.function,
        "target": destack._generated.program.vm.function.to_json_call_target(
            value.target
        ),
        "moves": destack._generated.program.vm.range.to_json_move_range(value.moves),
    }


def from_json_tail_call(value: Json) -> TailCall:
    """Return one TailCall from one JSON value."""
    object_ = json_object(value)

    return TailCall(
        function=json_int(json_field(object_, "function")),
        target=destack._generated.program.vm.function.from_json_call_target(
            json_field(object_, "target")
        ),
        moves=destack._generated.program.vm.range.from_json_move_range(
            json_field(object_, "moves")
        ),
    )


@dataclass(frozen=True, slots=True)
class TailCallVirtual:
    """Class tail call."""

    # the receiver cell offset
    receiver_offset: int
    # the dispatch table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dispatch table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tail_call_virtual(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCallVirtual:
        """Decode one TailCallVirtual."""
        return decode_tail_call_virtual(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tail_call_virtual(self)

    @classmethod
    def from_json(cls, value: Json) -> TailCallVirtual:
        """Return one TailCallVirtual from one JSON value."""
        return from_json_tail_call_virtual(value)


def encode_tail_call_virtual(writer: BinaryWriter, value: TailCallVirtual) -> None:
    """Encode one TailCallVirtual."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_tail_call_virtual(reader: BinaryReader) -> TailCallVirtual:
    """Decode one TailCallVirtual."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return TailCallVirtual(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
    )


def to_json_tail_call_virtual(value: TailCallVirtual) -> Json:
    """Return one JSON value for one TailCallVirtual."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_tail_call_virtual(value: Json) -> TailCallVirtual:
    """Return one TailCallVirtual from one JSON value."""
    object_ = json_object(value)

    return TailCallVirtual(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


@dataclass(frozen=True, slots=True)
class TailCallDynamic:
    """Dynamic tail call."""

    # the receiver cell offset
    receiver_offset: int
    # the dynamic table field projection
    table_field: destack._generated.program.vm.table.ProjectionId
    # the dynamic table slot
    slot: int
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tail_call_dynamic(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TailCallDynamic:
        """Decode one TailCallDynamic."""
        return decode_tail_call_dynamic(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tail_call_dynamic(self)

    @classmethod
    def from_json(cls, value: Json) -> TailCallDynamic:
        """Return one TailCallDynamic from one JSON value."""
        return from_json_tail_call_dynamic(value)


def encode_tail_call_dynamic(writer: BinaryWriter, value: TailCallDynamic) -> None:
    """Encode one TailCallDynamic."""
    writer.write_unsigned(value.receiver_offset)
    destack._generated.program.vm.table.encode_projection_id(writer, value.table_field)
    writer.write_unsigned(value.slot)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_tail_call_dynamic(reader: BinaryReader) -> TailCallDynamic:
    """Decode one TailCallDynamic."""
    receiver_offset = reader.read_number()
    table_field = destack._generated.program.vm.table.decode_projection_id(reader)
    slot = reader.read_number()
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return TailCallDynamic(
        receiver_offset=receiver_offset,
        table_field=table_field,
        slot=slot,
        arguments=arguments,
    )


def to_json_tail_call_dynamic(value: TailCallDynamic) -> Json:
    """Return one JSON value for one TailCallDynamic."""
    return {
        "receiverOffset": value.receiver_offset,
        "tableField": destack._generated.program.vm.table.to_json_projection_id(
            value.table_field
        ),
        "slot": value.slot,
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_tail_call_dynamic(value: Json) -> TailCallDynamic:
    """Return one TailCallDynamic from one JSON value."""
    object_ = json_object(value)

    return TailCallDynamic(
        receiver_offset=json_int(json_field(object_, "receiverOffset")),
        table_field=destack._generated.program.vm.table.from_json_projection_id(
            json_field(object_, "tableField")
        ),
        slot=json_int(json_field(object_, "slot")),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


@dataclass(frozen=True, slots=True)
class IndirectTailCall:
    """Indirect tail call."""

    # the callee cell offset
    callee_offset: int
    # the expected function signature
    signature: destack._generated.mir.tree.node.LocalNodeId
    # the pooled argument range
    arguments: destack._generated.program.vm.range.ArgumentRange

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_indirect_tail_call(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectTailCall:
        """Decode one IndirectTailCall."""
        return decode_indirect_tail_call(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_indirect_tail_call(self)

    @classmethod
    def from_json(cls, value: Json) -> IndirectTailCall:
        """Return one IndirectTailCall from one JSON value."""
        return from_json_indirect_tail_call(value)


def encode_indirect_tail_call(writer: BinaryWriter, value: IndirectTailCall) -> None:
    """Encode one IndirectTailCall."""
    writer.write_unsigned(value.callee_offset)
    destack._generated.mir.tree.node.encode_local_node_id(writer, value.signature)
    destack._generated.program.vm.range.encode_argument_range(writer, value.arguments)


def decode_indirect_tail_call(reader: BinaryReader) -> IndirectTailCall:
    """Decode one IndirectTailCall."""
    callee_offset = reader.read_number()
    signature = destack._generated.mir.tree.node.decode_local_node_id(reader)
    arguments = destack._generated.program.vm.range.decode_argument_range(reader)

    return IndirectTailCall(
        callee_offset=callee_offset,
        signature=signature,
        arguments=arguments,
    )


def to_json_indirect_tail_call(value: IndirectTailCall) -> Json:
    """Return one JSON value for one IndirectTailCall."""
    return {
        "calleeOffset": value.callee_offset,
        "signature": destack._generated.mir.tree.node.to_json_local_node_id(
            value.signature
        ),
        "arguments": destack._generated.program.vm.range.to_json_argument_range(
            value.arguments
        ),
    }


def from_json_indirect_tail_call(value: Json) -> IndirectTailCall:
    """Return one IndirectTailCall from one JSON value."""
    object_ = json_object(value)

    return IndirectTailCall(
        callee_offset=json_int(json_field(object_, "calleeOffset")),
        signature=destack._generated.mir.tree.node.from_json_local_node_id(
            json_field(object_, "signature")
        ),
        arguments=destack._generated.program.vm.range.from_json_argument_range(
            json_field(object_, "arguments")
        ),
    )


__all__ = [
    "AggregateSelect",
    "encode_aggregate_select",
    "decode_aggregate_select",
    "to_json_aggregate_select",
    "from_json_aggregate_select",
    "AtomicCompareExchange",
    "encode_atomic_compare_exchange",
    "decode_atomic_compare_exchange",
    "to_json_atomic_compare_exchange",
    "from_json_atomic_compare_exchange",
    "VectorSplat",
    "encode_vector_splat",
    "decode_vector_splat",
    "to_json_vector_splat",
    "from_json_vector_splat",
    "VectorExtract",
    "encode_vector_extract",
    "decode_vector_extract",
    "to_json_vector_extract",
    "from_json_vector_extract",
    "VectorBinary",
    "encode_vector_binary",
    "decode_vector_binary",
    "to_json_vector_binary",
    "from_json_vector_binary",
    "ElementBinaryKernel",
    "encode_element_binary_kernel",
    "decode_element_binary_kernel",
    "to_json_element_binary_kernel",
    "from_json_element_binary_kernel",
    "VectorUnary",
    "encode_vector_unary",
    "decode_vector_unary",
    "to_json_vector_unary",
    "from_json_vector_unary",
    "ElementUnaryKernel",
    "encode_element_unary_kernel",
    "decode_element_unary_kernel",
    "to_json_element_unary_kernel",
    "from_json_element_unary_kernel",
    "VectorInsert",
    "encode_vector_insert",
    "decode_vector_insert",
    "to_json_vector_insert",
    "from_json_vector_insert",
    "VectorShuffle",
    "encode_vector_shuffle",
    "decode_vector_shuffle",
    "to_json_vector_shuffle",
    "from_json_vector_shuffle",
    "VectorSelect",
    "encode_vector_select",
    "decode_vector_select",
    "to_json_vector_select",
    "from_json_vector_select",
    "VectorReduce",
    "encode_vector_reduce",
    "decode_vector_reduce",
    "to_json_vector_reduce",
    "from_json_vector_reduce",
    "VectorConvert",
    "encode_vector_convert",
    "decode_vector_convert",
    "to_json_vector_convert",
    "from_json_vector_convert",
    "FunctionBind",
    "encode_function_bind",
    "decode_function_bind",
    "to_json_function_bind",
    "from_json_function_bind",
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
    "CallBranch",
    "encode_call_branch",
    "decode_call_branch",
    "to_json_call_branch",
    "from_json_call_branch",
    "CallVirtual",
    "encode_call_virtual",
    "decode_call_virtual",
    "to_json_call_virtual",
    "from_json_call_virtual",
    "CallVirtualBranch",
    "encode_call_virtual_branch",
    "decode_call_virtual_branch",
    "to_json_call_virtual_branch",
    "from_json_call_virtual_branch",
    "CallDynamic",
    "encode_call_dynamic",
    "decode_call_dynamic",
    "to_json_call_dynamic",
    "from_json_call_dynamic",
    "CallDynamicBranch",
    "encode_call_dynamic_branch",
    "decode_call_dynamic_branch",
    "to_json_call_dynamic_branch",
    "from_json_call_dynamic_branch",
    "IndirectCall",
    "encode_indirect_call",
    "decode_indirect_call",
    "to_json_indirect_call",
    "from_json_indirect_call",
    "IndirectCallBranch",
    "encode_indirect_call_branch",
    "decode_indirect_call_branch",
    "to_json_indirect_call_branch",
    "from_json_indirect_call_branch",
    "TensorLoad",
    "encode_tensor_load",
    "decode_tensor_load",
    "to_json_tensor_load",
    "from_json_tensor_load",
    "TensorExtract",
    "encode_tensor_extract",
    "decode_tensor_extract",
    "to_json_tensor_extract",
    "from_json_tensor_extract",
    "TensorBinary",
    "encode_tensor_binary",
    "decode_tensor_binary",
    "to_json_tensor_binary",
    "from_json_tensor_binary",
    "TensorContiguousBinary",
    "encode_tensor_contiguous_binary",
    "decode_tensor_contiguous_binary",
    "to_json_tensor_contiguous_binary",
    "from_json_tensor_contiguous_binary",
    "TensorUnary",
    "encode_tensor_unary",
    "decode_tensor_unary",
    "to_json_tensor_unary",
    "from_json_tensor_unary",
    "TensorContiguousUnary",
    "encode_tensor_contiguous_unary",
    "decode_tensor_contiguous_unary",
    "to_json_tensor_contiguous_unary",
    "from_json_tensor_contiguous_unary",
    "TensorStore",
    "encode_tensor_store",
    "decode_tensor_store",
    "to_json_tensor_store",
    "from_json_tensor_store",
    "TensorFill",
    "encode_tensor_fill",
    "decode_tensor_fill",
    "to_json_tensor_fill",
    "from_json_tensor_fill",
    "TensorCopy",
    "encode_tensor_copy",
    "decode_tensor_copy",
    "to_json_tensor_copy",
    "from_json_tensor_copy",
    "TensorViewCast",
    "encode_tensor_view_cast",
    "decode_tensor_view_cast",
    "to_json_tensor_view_cast",
    "from_json_tensor_view_cast",
    "TensorReshape",
    "encode_tensor_reshape",
    "decode_tensor_reshape",
    "to_json_tensor_reshape",
    "from_json_tensor_reshape",
    "TensorBroadcast",
    "encode_tensor_broadcast",
    "decode_tensor_broadcast",
    "to_json_tensor_broadcast",
    "from_json_tensor_broadcast",
    "TensorTranspose",
    "encode_tensor_transpose",
    "decode_tensor_transpose",
    "to_json_tensor_transpose",
    "from_json_tensor_transpose",
    "TensorSlice",
    "encode_tensor_slice",
    "decode_tensor_slice",
    "to_json_tensor_slice",
    "from_json_tensor_slice",
    "TensorPad",
    "encode_tensor_pad",
    "decode_tensor_pad",
    "to_json_tensor_pad",
    "from_json_tensor_pad",
    "TensorConcat",
    "encode_tensor_concat",
    "decode_tensor_concat",
    "to_json_tensor_concat",
    "from_json_tensor_concat",
    "TensorReduce",
    "encode_tensor_reduce",
    "decode_tensor_reduce",
    "to_json_tensor_reduce",
    "from_json_tensor_reduce",
    "TensorIndexReduce",
    "encode_tensor_index_reduce",
    "decode_tensor_index_reduce",
    "to_json_tensor_index_reduce",
    "from_json_tensor_index_reduce",
    "TensorDot",
    "encode_tensor_dot",
    "decode_tensor_dot",
    "to_json_tensor_dot",
    "from_json_tensor_dot",
    "TensorConvolution",
    "encode_tensor_convolution",
    "decode_tensor_convolution",
    "to_json_tensor_convolution",
    "from_json_tensor_convolution",
    "TensorGather",
    "encode_tensor_gather",
    "decode_tensor_gather",
    "to_json_tensor_gather",
    "from_json_tensor_gather",
    "TensorScatter",
    "encode_tensor_scatter",
    "decode_tensor_scatter",
    "to_json_tensor_scatter",
    "from_json_tensor_scatter",
    "TensorSelect",
    "encode_tensor_select",
    "decode_tensor_select",
    "to_json_tensor_select",
    "from_json_tensor_select",
    "TensorConvert",
    "encode_tensor_convert",
    "decode_tensor_convert",
    "to_json_tensor_convert",
    "from_json_tensor_convert",
    "TensorView",
    "encode_tensor_view",
    "decode_tensor_view",
    "to_json_tensor_view",
    "from_json_tensor_view",
    "Intrinsic",
    "encode_intrinsic",
    "decode_intrinsic",
    "to_json_intrinsic",
    "from_json_intrinsic",
    "IntrinsicDest",
    "encode_intrinsic_dest",
    "decode_intrinsic_dest",
    "to_json_intrinsic_dest",
    "from_json_intrinsic_dest",
    "IntrinsicDestNone",
    "IntrinsicDestCell",
    "IntrinsicDestFrame",
    "TailCall",
    "encode_tail_call",
    "decode_tail_call",
    "to_json_tail_call",
    "from_json_tail_call",
    "TailCallVirtual",
    "encode_tail_call_virtual",
    "decode_tail_call_virtual",
    "to_json_tail_call_virtual",
    "from_json_tail_call_virtual",
    "TailCallDynamic",
    "encode_tail_call_dynamic",
    "decode_tail_call_dynamic",
    "to_json_tail_call_dynamic",
    "from_json_tail_call_dynamic",
    "IndirectTailCall",
    "encode_indirect_tail_call",
    "decode_indirect_tail_call",
    "to_json_indirect_tail_call",
    "from_json_indirect_tail_call",
]
