import math
from typing import TYPE_CHECKING, assert_never, override

from destack.registry import ENUM_CLASS_BY_TYPE, STRUCT_DEFINITION_BY_TYPE

from ...builtin import PrimitiveType, ScalarType, StructType, TypeCardinality
from ...definition import NodeDefinition, PropertyDefinition, StructDefinition
from .._core import ObjectSize, ObjectSizer

if TYPE_CHECKING:
    from destack import Type


class RustObjectSizer(ObjectSizer):
    """
    Estimate the size of values in our Rust runtime (64-bit) in bytes.
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
    TIMESTAMP_SIZE = 8
    DURATION_SIZE = 8

    # uuid size
    UUID_SIZE = 16

    # interned sizes
    INTERNED_KEY_SIZE = 8

    @override
    def size_object(
        self,
        object: "StructDefinition | NodeDefinition",
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated size of an object instance in bytes."""
        total_min_size = 0
        total_max_size = 0
        path = (*_path, object.type) if isinstance(object, StructDefinition) else _path
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            prop_size = self.size_property(prop, include_reference=False, _path=path)
            total_min_size += prop_size.min_size
            if total_max_size is None or prop_size.max_size is None:
                total_max_size = None
            else:
                total_max_size += prop_size.max_size
        return ObjectSize(total_min_size, total_max_size)

    @override
    def size_property(
        self,
        property: "PropertyDefinition",
        include_reference: bool = True,
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated size of a property in bytes."""
        if property.is_interned:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        else:
            return self.size_type(property.type, include_reference=include_reference, _path=_path)

    @override
    def size_type(
        self,
        type: "Type",
        include_reference: bool = True,
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated size of a Type's value in bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type, _path=_path)
        # list (Vec)
        elif type.cardinality == TypeCardinality.LIST:
            size = ObjectSize(self.VEC_HEADER_SIZE, None)
        # tuple (inline)
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type(t, _path=_path) for t in type.element_types]
            min_size = sum(s.min_size for s in elem_sizes)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        # map (HashMap)
        elif type.cardinality == TypeCardinality.MAP:
            size = ObjectSize(self.HASHMAP_HEADER_SIZE, None)
        #
        else:
            assert_never(type.cardinality)

        # optional values don't change anything because the space needs to exist anyway

        # include_reference is a no-op for Rust (values are inline in structs)

        return size

    @override
    def size_type_scalar(
        self,
        type: "Type",
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
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
            elif type.primitive_type == PrimitiveType.TIMESTAMP:
                return ObjectSize(self.TIMESTAMP_SIZE, self.TIMESTAMP_SIZE)
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
        # enum: smallest int that fits
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            max_bytes = math.ceil(math.log2(max(enum_cls)) / 8)
            return ObjectSize(max_bytes, max_bytes)
        # node
        elif type.scalar_type == ScalarType.NODE:
            return ObjectSize(8, 8)  # reference?
        # node reference as struct
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        # node id (uuid)
        elif type.scalar_type == ScalarType.NODE_RAW:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return ObjectSize(self.INTERNED_KEY_SIZE, self.INTERNED_KEY_SIZE)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            if type.struct_type in _path:
                return ObjectSize(0, 0)  # circular self reference
            else:
                return self.size_object(STRUCT_DEFINITION_BY_TYPE[type.struct_type], _path=_path)
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot size HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type_scalar(t, _path=_path) for t in type.element_types]
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
    TIMESTAMP_RANGE = (1, 10)  # nanos zigzag varint
    DURATION_RANGE = (1, 10)  # nanos zigzag varint

    @override
    def size_object(
        self,
        object: "StructDefinition | NodeDefinition",
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated encoded size of an object in bytes (approximate)."""
        total_min_size = 0
        total_max_size = 0
        path = (*_path, object.type) if isinstance(object, StructDefinition) else _path
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            min_size, max_size = self.size_property(prop, include_reference=False, _path=path)
            total_min_size += min_size
            if total_max_size is None or max_size is None:
                total_max_size = None
            else:
                total_max_size += max_size
        return ObjectSize(total_min_size, total_max_size)

    @override
    def size_property(
        self,
        property: "PropertyDefinition",
        include_reference: bool = True,
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated encoded size of a property in bytes."""
        return self.size_type(property.type, include_reference=include_reference, _path=_path)

    @override
    def size_type(
        self,
        type: "Type",
        include_reference: bool = True,
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated encoded size of a Type's value in Kompakt bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type, _path=_path)
        # list: length varint + elements
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            # unknown length; empty list has length prefix (1 byte)
            size = ObjectSize(1, None)
        # tuple: length varint (known) + element sizes
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            length = len(type.element_types)
            base = self._estimate_varint_size(length)
            elem_sizes = [self.size_type(t, _path=_path) for t in type.element_types]
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
    def size_type_scalar(
        self,
        type: "Type",
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
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
            elif primitive == PrimitiveType.TIMESTAMP:
                return ObjectSize(*self.TIMESTAMP_RANGE)
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
        # enum: smallest varint up to 5 bytes
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            max_bytes = math.ceil(math.log2(max(enum_cls)) / 8)
            return ObjectSize(1, max_bytes)
        # node: encoded as object; unknown size
        elif type.scalar_type == ScalarType.NODE:
            return ObjectSize(0, None)
        # node reference: type (u32 varint) + 4 uuids
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_TEMPORAL_REFERENCE])
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_IDENTITY_REFERENCE])
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_SPATIAL_REFERENCE])
        # struct: approximate by summing field sizes from definition
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            if type.struct_type in _path:
                return ObjectSize(0, 0)  # circular self reference
            else:
                return self.size_object(STRUCT_DEFINITION_BY_TYPE[type.struct_type])
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot size HANDLE: {type!r}")
        # union: min/min, max/max (unknown if any unbounded)
        elif type.scalar_type == ScalarType.UNION:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type_scalar(t, _path=_path) for t in type.element_types]
            min_size = min((s.min_size for s in elem_sizes), default=0)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = max((s.max_size or 0 for s in elem_sizes), default=0)
            return ObjectSize(min_size, max_size)
        else:
            assert_never(type.scalar_type)

    def _estimate_varint_size(self, value: int) -> int:
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


class FlottObjectSizer(KompaktObjectSizer):
    """
    Estimate the encoded size in bytes for Flott binary format.
    Flott uses fixed-width integers and fixed 4-byte length prefixes (no varints).
    """

    # override integer sizes to be fixed-width
    INT16_RANGE = (2, 2)
    INT32_RANGE = (4, 4)
    INT64_RANGE = (8, 8)
    INT128_RANGE = (16, 16)
    UINT16_RANGE = (2, 2)
    UINT32_RANGE = (4, 4)
    UINT64_RANGE = (8, 8)
    UINT128_RANGE = (16, 16)

    # time-like primitives are fixed 8 bytes in Flott
    DATETIME_RANGE = (8, 8)
    DATE_RANGE = (8, 8)
    TIME_RANGE = (8, 8)
    TIMESTAMP_RANGE = (8, 8)
    DURATION_RANGE = (8, 8)

    @override
    def size_type(
        self,
        type: "Type",
        include_reference: bool = True,
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated encoded size of a Type's value in Flott bytes."""

        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type, _path=_path)
        elif type.cardinality == TypeCardinality.LIST:
            # fixed 4-byte length prefix + elements
            size = ObjectSize(4, None)
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            base = 4  # fixed u32 length prefix in Flott
            elem_sizes = [self.size_type(t, _path=_path) for t in type.element_types]
            min_size = base + sum(s.min_size for s in elem_sizes)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = base + sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        elif type.cardinality == TypeCardinality.MAP:
            # fixed 4-byte length prefix + entries
            size = ObjectSize(4, None)
        else:
            assert_never(type.cardinality)

        if not type.is_required:
            size = ObjectSize(0, size.max_size or size.min_size)

        return size

    @override
    def size_type_scalar(
        self,
        type: "Type",
        _path: tuple["StructType", ...] = (),
    ) -> ObjectSize:
        """Get the estimated encoded size of a scalar Type in Flott bytes."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

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
            elif primitive == PrimitiveType.TIMESTAMP:
                return ObjectSize(*self.TIMESTAMP_RANGE)
            elif primitive == PrimitiveType.DURATION:
                return ObjectSize(*self.DURATION_RANGE)
            elif primitive == PrimitiveType.UUID:
                return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
            elif primitive == PrimitiveType.BYTES:
                # fixed 4-byte length prefix + payload
                return ObjectSize(4, None)
            elif primitive == PrimitiveType.STRING:
                # fixed 4-byte length prefix + payload
                return ObjectSize(4, None)
            elif primitive == PrimitiveType.CHARACTER:
                return ObjectSize(self.CHARACTER_SIZE, self.CHARACTER_SIZE)
            elif primitive == PrimitiveType.JSON:
                # tag still 1 byte; nested values vary
                return ObjectSize(1, None)
            else:
                assert_never(primitive)
        elif type.scalar_type == ScalarType.ENUM:
            # enums encoded as fixed u32 in Flott
            return ObjectSize(4, 4)
        elif type.scalar_type == ScalarType.NODE:
            # encoded as object; unknown size
            return ObjectSize(0, None)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_TEMPORAL_REFERENCE])
        elif type.scalar_type == ScalarType.NODE_RAW:
            return ObjectSize(self.UUID_SIZE, self.UUID_SIZE)
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_IDENTITY_REFERENCE])
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_SPATIAL_REFERENCE])
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            if type.struct_type in _path:
                return ObjectSize(0, 0)
            else:
                return self.size_object(STRUCT_DEFINITION_BY_TYPE[type.struct_type])
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot size HANDLE: {type!r}")
        elif type.scalar_type == ScalarType.UNION:
            assert type.element_types is not None, f"no element types for {type!r}"
            elem_sizes = [self.size_type_scalar(t, _path=_path) for t in type.element_types]
            min_size = min((s.min_size for s in elem_sizes), default=0)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = max((s.max_size or 0 for s in elem_sizes), default=0)
            return ObjectSize(min_size, max_size)
        else:
            assert_never(type.scalar_type)
