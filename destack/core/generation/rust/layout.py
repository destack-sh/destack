from typing import TYPE_CHECKING, assert_never, override

from destack.registry import STRUCT_DEFINITION_BY_TYPE

from ...builtin import PrimitiveType, ScalarType, TypeCardinality
from ...definition import NodeDefinition, StructDefinition
from .._core import ObjectSize, ObjectSizer

if TYPE_CHECKING:
    from destack import Type


class RustObjectSizer(ObjectSizer):
    """
    Estimate the size of values in a Rust runtime (64-bit) in bytes.

    Assumptions:
    - struct fields store values inline; we do not add pointer overhead for fields
    - integers/floats/bool/char use their native fixed sizes
    - String and Vec headers are inline (ptr, len, cap) ≈ 24 bytes; payload is unbounded
    - HashMap header ≈ 48 bytes; entries are unbounded
    - UUID (uuid::Uuid) is 16 bytes
    - Chrono naive types (NaiveDateTime, NaiveTime, Duration) are approximated as 12 bytes
    - NaiveDate is approximated as 4 bytes
    - tuples are laid out inline: sum of element sizes (alignment ignored)
    - optional values (is_required=False) have min_size=0
    """

    # collection headers
    VEC_HEADER_SIZE = 24
    STRING_HEADER_SIZE = 24
    HASHMAP_HEADER_SIZE = 48

    # primitive sizes
    BOOL_SIZE = 1
    CHAR_SIZE = 4  # Rust char is a Unicode scalar value (u32)
    INT8_SIZE = 1
    INT16_SIZE = 2
    INT32_SIZE = 4
    INT64_SIZE = 8
    INT128_SIZE = 16
    UINT8_SIZE = 1
    UINT16_SIZE = 2
    UINT32_SIZE = 4
    UINT64_SIZE = 8
    UINT128_SIZE = 16
    FLOAT16_SIZE = 2
    FLOAT32_SIZE = 4
    FLOAT64_SIZE = 8

    # datetime sizes
    DATETIME_SIZE = 8
    DATE_SIZE = 8
    TIME_SIZE = 8
    DURATION_SIZE = 8

    # uuid size
    UUID_SIZE = 16

    # interned sizes
    INTERNED_KEY_SIZE = 8

    @override
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated size of an object instance in bytes."""
        total_max_size = 0
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            if prop.is_interned:
                prop_size = ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
            else:
                prop_size = self.size_type(prop.type, include_reference=False)
            total_max_size += prop_size.max_size or prop_size.min_size
        # max size = min size
        return ObjectSize(total_max_size, total_max_size)

    @override
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """Get the estimated size of a Type's value in bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type)
        # list (Vec)
        elif type.cardinality == TypeCardinality.LIST:
            size = ObjectSize(self.VEC_HEADER_SIZE, None)
        # tuple (inline)
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type(t) for t in type.element_types]
            min_size = sum(s.min_size for s in elem_sizes)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        # map (HashMap)
        elif type.cardinality == TypeCardinality.MAP:
            size = ObjectSize(self.HASHMAP_HEADER_SIZE, None)
        else:
            assert_never(type.cardinality)

        # optional values don't change anything because the space needs to exist anyway

        # include_reference is a no-op for Rust (values are inline in structs)

        return size

    @override
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated size of a scalar type (no reference overhead)."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return ObjectSize(0, 0)
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return ObjectSize(self.BOOL_SIZE, self.BOOL_SIZE)
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return ObjectSize(self.CHAR_SIZE, self.CHAR_SIZE)
            elif type.primitive_type == PrimitiveType.INT8:
                return ObjectSize(self.INT8_SIZE, self.INT8_SIZE)
            elif type.primitive_type == PrimitiveType.INT16:
                return ObjectSize(self.INT16_SIZE, self.INT16_SIZE)
            elif type.primitive_type == PrimitiveType.INT32:
                return ObjectSize(self.INT32_SIZE, self.INT32_SIZE)
            elif type.primitive_type == PrimitiveType.INT64:
                return ObjectSize(self.INT64_SIZE, self.INT64_SIZE)
            elif type.primitive_type == PrimitiveType.INT128:
                return ObjectSize(self.INT128_SIZE, self.INT128_SIZE)
            elif type.primitive_type == PrimitiveType.UINT8:
                return ObjectSize(self.UINT8_SIZE, self.UINT8_SIZE)
            elif type.primitive_type == PrimitiveType.UINT16:
                return ObjectSize(self.UINT16_SIZE, self.UINT16_SIZE)
            elif type.primitive_type == PrimitiveType.UINT32:
                return ObjectSize(self.UINT32_SIZE, self.UINT32_SIZE)
            elif type.primitive_type == PrimitiveType.UINT64:
                return ObjectSize(self.UINT64_SIZE, self.UINT64_SIZE)
            elif type.primitive_type == PrimitiveType.UINT128:
                return ObjectSize(self.UINT128_SIZE, self.UINT128_SIZE)
            elif type.primitive_type == PrimitiveType.FLOAT16:
                return ObjectSize(self.FLOAT16_SIZE, self.FLOAT16_SIZE)
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return ObjectSize(self.FLOAT32_SIZE, self.FLOAT32_SIZE)
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return ObjectSize(self.FLOAT64_SIZE, self.FLOAT64_SIZE)
            elif type.primitive_type == PrimitiveType.DATETIME:
                return ObjectSize(self.DATETIME_SIZE, self.DATETIME_SIZE)
            elif type.primitive_type == PrimitiveType.DATE:
                return ObjectSize(self.DATE_SIZE, self.DATE_SIZE)
            elif type.primitive_type == PrimitiveType.TIME:
                return ObjectSize(self.TIME_SIZE, self.TIME_SIZE)
            elif type.primitive_type == PrimitiveType.DURATION:
                return ObjectSize(self.DURATION_SIZE, self.DURATION_SIZE)
            elif type.primitive_type == PrimitiveType.UUID:
                return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
            elif type.primitive_type == PrimitiveType.BYTES:
                # Vec<u8> header
                return ObjectSize(self.VEC_HEADER_SIZE, None)
            elif type.primitive_type == PrimitiveType.STRING:
                # String header
                return ObjectSize(self.STRING_HEADER_SIZE, None)
            elif type.primitive_type == PrimitiveType.JSON:
                # opaque / serde_json::Value varies; treat as unknown
                return ObjectSize(8, 8)
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return ObjectSize(4, 4)
        # node
        elif type.scalar_type == ScalarType.NODE:
            return ObjectSize(8, 8)  # reference?
        # node reference as struct
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        # node id (uuid)
        elif type.scalar_type == ScalarType.NODE_ID:
            return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[type.struct_type])
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot size HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type_scalar(t) for t in type.element_types]
            min_size = min((s.min_size for s in elem_sizes), default=0)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = max((s.max_size or 0 for s in elem_sizes), default=0)
            return ObjectSize(min_size, max_size)
        else:
            assert_never(type.scalar_type)


class KompaktObjectSizer(ObjectSizer):
    """
    Estimate the encoded size in bytes for Kompakt binary format.
    NOTE: this isn't technically Rust-specific but it's related and I have to put this somewhere.

    Uses the same sizing as BinaryWriter in `destack/core/encoding/binary.py`:
    - varint/zigzag sizes for integers
    - fixed sizes for floats, bool, char, uuid
    - time types sized as their encoded integer forms (varint ranges)
    - strings/bytes with varint length prefix + payload (unbounded)
    - lists/tuples/maps compound following encoder behavior
    - enums encoded as varint u32
    - node reference encoded as type (varint u32) + 4 uuids
    """

    # fixed sizes
    BOOL_SIZE = 1
    INT8_SIZE = 1
    UINT8_SIZE = 1
    FLOAT16_SIZE = 2
    FLOAT32_SIZE = 4
    FLOAT64_SIZE = 8
    CHARACTER_SIZE = 4
    UUID_SIZE = 16

    # varint ranges (min, max)
    INT16_RANGE = (1, 3)
    INT32_RANGE = (1, 5)
    INT64_RANGE = (1, 10)
    INT128_RANGE = (1, 19)
    UINT16_RANGE = (1, 3)
    UINT32_RANGE = (1, 5)
    UINT64_RANGE = (1, 10)
    UINT128_RANGE = (1, 19)

    DATETIME_RANGE = (1, 10)  # micros zigzag varint
    DATE_RANGE = (1, 5)  # days varint
    TIME_RANGE = (1, 6)  # nanos since midnight
    DURATION_RANGE = (1, 10)  # nanos zigzag varint

    @override
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated encoded size of an object in bytes (approximate)."""
        total_min_size = 0
        total_max_size = 0
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            min_size, max_size = self.size_type(prop.type, include_reference=False)
            total_min_size += min_size
            total_max_size += max_size or min_size
        return ObjectSize(total_min_size, total_max_size)

    @override
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """Get the estimated encoded size of a Type's value in Kompakt bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type)
        # list: length varint + elements
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            # unknown length; empty list has length prefix (1 byte)
            size = ObjectSize(1, None)
        # tuple: length varint (known) + element sizes
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            length = len(type.element_types)
            base = self._varint_size(length)
            elem_sizes = [self.size_type(t) for t in type.element_types]
            min_size = base + sum(s.min_size for s in elem_sizes)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = base + sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        # map: length varint + entries
        elif type.cardinality == TypeCardinality.MAP:
            # unknown length; empty map has length prefix (1 byte)
            size = ObjectSize(1, None)
        else:
            assert_never(type.cardinality)

        # optional min size is 0
        if not type.is_required:
            size = ObjectSize(0, size.max_size or size.min_size)

        # include_reference is irrelevant for encoded bytes
        return size

    @override
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated encoded size of a scalar Type in Kompakt bytes."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            primitive = type.primitive_type
            if primitive == PrimitiveType.NONE:
                return ObjectSize(0, 0)
            elif primitive == PrimitiveType.BOOLEAN:
                return ObjectSize(self.BOOL_SIZE, self.BOOL_SIZE)
            elif primitive == PrimitiveType.INT8:
                return ObjectSize(self.INT8_SIZE, self.INT8_SIZE)
            elif primitive == PrimitiveType.INT16:
                return ObjectSize(*self.INT16_RANGE)
            elif primitive == PrimitiveType.INT32:
                return ObjectSize(*self.INT32_RANGE)
            elif primitive == PrimitiveType.INT64:
                return ObjectSize(*self.INT64_RANGE)
            elif primitive == PrimitiveType.INT128:
                return ObjectSize(*self.INT128_RANGE)
            elif primitive == PrimitiveType.UINT8:
                return ObjectSize(self.UINT8_SIZE, self.UINT8_SIZE)
            elif primitive == PrimitiveType.UINT16:
                return ObjectSize(*self.UINT16_RANGE)
            elif primitive == PrimitiveType.UINT32:
                return ObjectSize(*self.UINT32_RANGE)
            elif primitive == PrimitiveType.UINT64:
                return ObjectSize(*self.UINT64_RANGE)
            elif primitive == PrimitiveType.UINT128:
                return ObjectSize(*self.UINT128_RANGE)
            elif primitive == PrimitiveType.FLOAT16:
                return ObjectSize(self.FLOAT16_SIZE, self.FLOAT16_SIZE)
            elif primitive == PrimitiveType.FLOAT32:
                return ObjectSize(self.FLOAT32_SIZE, self.FLOAT32_SIZE)
            elif primitive == PrimitiveType.FLOAT64:
                return ObjectSize(self.FLOAT64_SIZE, self.FLOAT64_SIZE)
            elif primitive == PrimitiveType.DATETIME:
                return ObjectSize(*self.DATETIME_RANGE)
            elif primitive == PrimitiveType.DATE:
                return ObjectSize(*self.DATE_RANGE)
            elif primitive == PrimitiveType.TIME:
                return ObjectSize(*self.TIME_RANGE)
            elif primitive == PrimitiveType.DURATION:
                return ObjectSize(*self.DURATION_RANGE)
            elif primitive == PrimitiveType.UUID:
                return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
            elif primitive == PrimitiveType.BYTES:
                # length varint + payload
                return ObjectSize(1, None)
            elif primitive == PrimitiveType.STRING:
                # length varint + payload
                return ObjectSize(1, None)
            elif primitive == PrimitiveType.CHARACTER:
                return ObjectSize(self.CHARACTER_SIZE, self.CHARACTER_SIZE)
            elif primitive == PrimitiveType.JSON:
                # at least a 1-byte tag, contents vary
                return ObjectSize(1, None)
            else:
                assert_never(primitive)
        # enum: u32 varint
        elif type.scalar_type == ScalarType.ENUM:
            return ObjectSize(*self.UINT32_RANGE)
        # node: encoded as object; unknown size
        elif type.scalar_type == ScalarType.NODE:
            return ObjectSize(0, None)
        # node reference: type (u32 varint) + 4 uuids
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            min_size = self.UINT32_RANGE[0] + 4 * self.UUID_SIZE
            max_size = self.UINT32_RANGE[1] + 4 * self.UUID_SIZE
            return ObjectSize(min_size, max_size)
        # node id: uuid
        elif type.scalar_type == ScalarType.NODE_ID:
            return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
        # struct: approximate by summing field sizes from definition
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[type.struct_type])
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot size HANDLE: {type!r}")
        # union: min/min, max/max (unknown if any unbounded)
        elif type.scalar_type == ScalarType.UNION:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type_scalar(t) for t in type.element_types]
            min_size = min((s.min_size for s in elem_sizes), default=0)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = max((s.max_size or 0 for s in elem_sizes), default=0)
            return ObjectSize(min_size, max_size)
        else:
            assert_never(type.scalar_type)

    def _varint_size(self, value: int) -> int:
        """Get the number of bytes to encode an unsigned integer as varint."""
        assert value >= 0, f"varint value must be non-negative: {value!r}"
        if value < 0x80:
            return 1
        elif value < 0x4000:
            return 2
        elif value < 0x20_0000:
            return 3
        elif value < 0x1000_0000:
            return 4
        else:
            return 5
