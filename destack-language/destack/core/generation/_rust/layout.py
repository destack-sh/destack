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
