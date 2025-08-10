from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, NamedTuple, assert_never, override

from destack.registry import STRUCT_DEFINITION_BY_TYPE

from ..builtin import PrimitiveType, ScalarType, StructType, TypeCardinality
from ..definition import NodeDefinition, StructDefinition

if TYPE_CHECKING:
    from destack import Type


class ObjectSize(NamedTuple):
    """The estimated size of an Object definition in bytes."""

    min_size: int
    max_size: int | None  # unset means unbounded


class ObjectSizer(ABC):
    """Estimate the actual size of an Object definition in bytes."""

    @abstractmethod
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated size of an object in bytes."""
        raise NotImplementedError

    @abstractmethod
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """
        Get the estimated size of a Type's value in bytes.
        If include_reference we add the overhead for a reference to this value (in managed languages).
        """
        raise NotImplementedError

    @abstractmethod
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated size of a scalar type in bytes."""
        raise NotImplementedError


class PythonObjectSizer(ObjectSizer):
    """Estimate the actual size of Objects in the Python runtime in bytes."""

    # python object overhead sizes
    POINTER_SIZE = 8
    OBJECT_BASE_SIZE = 16
    TUPLE_BASE_SIZE = 40  # empty tuple
    FLOAT_OBJECT_SIZE = 24
    DATETIME_OBJECT_SIZE = 48
    DATE_OBJECT_SIZE = 32
    TIME_OBJECT_SIZE = 40
    TIMEDELTA_OBJECT_SIZE = 40
    UUID_OBJECT_SIZE = 56
    BYTES_EMPTY_SIZE = 33
    STRING_EMPTY_SIZE = 49
    LIST_EMPTY_SIZE = 56
    DICT_EMPTY_SIZE = 64

    @override
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated size of an object in bytes."""
        total_min_size = self.OBJECT_BASE_SIZE
        total_max_size = self.OBJECT_BASE_SIZE
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            min_size, max_size = self.size_type(
                prop.type,
                include_reference=False,
            )
            total_min_size += min_size + self.POINTER_SIZE
            total_max_size += (max_size or min_size) + self.POINTER_SIZE
        return ObjectSize(total_min_size, total_max_size)

    @override
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """Get the estimated size of a Type's value in bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type)
        # list
        elif type.cardinality == TypeCardinality.LIST:
            # list object only, no elements yet
            size = ObjectSize(self.LIST_EMPTY_SIZE, None)
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            # tuple object + refs + element sizes
            base = self.TUPLE_BASE_SIZE + self.POINTER_SIZE * len(type.element_types)
            elem_sizes = [self.size_type(t) for t in type.element_types]
            min_size = base + sum(s.min_size for s in elem_sizes)
            # any unbounded makes tuple unbounded
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = base + sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        # map
        elif type.cardinality == TypeCardinality.MAP:
            # dict object only, entries unknown
            size = ObjectSize(self.DICT_EMPTY_SIZE, None)
        #
        else:
            assert_never(type.cardinality)

        # if optional min size is 0
        if not type.is_required:
            size = ObjectSize(0, size.max_size or size.min_size)

        # add pointer overhead if requested
        if include_reference:
            size = ObjectSize(
                size.min_size + self.POINTER_SIZE,
                (size.max_size or size.min_size) + self.POINTER_SIZE,
            )

        return size

    @override
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated size of a scalar type EXCLUDING the reference to it (no pointers)."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"
        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            primitive = type.primitive_type
            if primitive == PrimitiveType.NONE:
                return ObjectSize(0, 0)
            elif primitive == PrimitiveType.BOOLEAN:
                return ObjectSize(0, 0)
            elif primitive in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            ):
                # PyLong: ~28 bytes header + 4 bytes per 30-bit digit
                bits_by_type = {
                    PrimitiveType.INT8: 8,
                    PrimitiveType.INT16: 16,
                    PrimitiveType.INT32: 32,
                    PrimitiveType.INT64: 64,
                    PrimitiveType.INT128: 128,
                    PrimitiveType.UINT8: 8,
                    PrimitiveType.UINT16: 16,
                    PrimitiveType.UINT32: 32,
                    PrimitiveType.UINT64: 64,
                    PrimitiveType.UINT128: 128,
                }
                bits = bits_by_type[primitive]
                digits = max(1, (bits + 29) // 30)
                size = 28 + 4 * digits
                return ObjectSize(size, size)
            elif primitive in (PrimitiveType.FLOAT16, PrimitiveType.FLOAT32, PrimitiveType.FLOAT64):
                return ObjectSize(self.FLOAT_OBJECT_SIZE, self.FLOAT_OBJECT_SIZE)
            elif primitive == PrimitiveType.DATETIME:
                return ObjectSize(self.DATETIME_OBJECT_SIZE, self.DATETIME_OBJECT_SIZE)
            elif primitive == PrimitiveType.DATE:
                return ObjectSize(self.DATE_OBJECT_SIZE, self.DATE_OBJECT_SIZE)
            elif primitive == PrimitiveType.TIME:
                return ObjectSize(self.TIME_OBJECT_SIZE, self.TIME_OBJECT_SIZE)
            elif primitive == PrimitiveType.DURATION:
                return ObjectSize(self.TIMEDELTA_OBJECT_SIZE, self.TIMEDELTA_OBJECT_SIZE)
            elif primitive == PrimitiveType.UUID:
                return ObjectSize(self.UUID_OBJECT_SIZE, self.UUID_OBJECT_SIZE)
            elif primitive == PrimitiveType.BYTES:
                return ObjectSize(self.BYTES_EMPTY_SIZE, None)
            elif primitive == PrimitiveType.STRING:
                return ObjectSize(self.STRING_EMPTY_SIZE, None)
            elif primitive == PrimitiveType.CHARACTER:
                return ObjectSize(self.STRING_EMPTY_SIZE + 1, self.STRING_EMPTY_SIZE + 1)
            elif primitive == PrimitiveType.JSON:
                return ObjectSize(0, 0)
            else:
                assert_never(primitive)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            # enums are singletons, so just a pointer
            return ObjectSize(0, 0)
        # node
        elif type.scalar_type == ScalarType.NODE:
            # references in memory, also just pointers
            return ObjectSize(0, 0)
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            # treat as struct
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_REFERENCE])
        # node id
        elif type.scalar_type == ScalarType.NODE_ID:
            # UUID object
            return ObjectSize(self.UUID_OBJECT_SIZE, self.UUID_OBJECT_SIZE)
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
            min_size = min(s.min_size for s in elem_sizes) if elem_sizes else 0
            # max is unknown if any is unbounded
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = max(s.max_size or 0 for s in elem_sizes) if elem_sizes else 0
            return ObjectSize(min_size, max_size)
        #
        else:
            assert_never(type.scalar_type)
