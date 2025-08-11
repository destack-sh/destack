from typing import TYPE_CHECKING, assert_never, override

from destack.registry import STRUCT_DEFINITION_BY_TYPE

from ...builtin import PrimitiveType, ScalarType, StructType, TypeCardinality
from ...definition import NodeDefinition, StructDefinition
from .._core import ObjectSize, ObjectSizer

if TYPE_CHECKING:
    from destack import Type


class TypeScriptObjectSizer(ObjectSizer):
    """
    Estimate the size of values in a TypeScript/JS runtime in bytes.

    Assumptions:
    - primitives (number, boolean, string, null) do not incur pointer overhead
    - UUIDs are represented as strings (36 UTF-16 code units)
    - Date/Datetime/Time/Duration are represented using Temporal objects
    - collections (arrays, tuples, maps) and objects are references
    """

    # js reference/pointer size (engine dependent)
    POINTER_SIZE = 8

    # object/collection base overheads (engine dependent)
    OBJECT_BASE_SIZE = 16
    ARRAY_BASE_SIZE = 24
    TUPLE_BASE_SIZE = 24
    MAP_BASE_SIZE = 32

    # primitive sizes
    NUMBER_SIZE = 8
    BOOLEAN_SIZE = 1
    STRING_EMPTY_SIZE = 0  # UTF-16, dynamic payload; empty string is 0
    BYTES_EMPTY_SIZE = 0  # ArrayBuffer length 0

    # datetime sizes
    TEMPORAL_INSTANT_SIZE = 16
    TEMPORAL_DATE_SIZE = 12
    TEMPORAL_TIME_SIZE = 16
    TEMPORAL_DURATION_SIZE = 24

    @override
    def size_object(self, object: "StructDefinition | NodeDefinition") -> ObjectSize:
        """Get the estimated size of an object instance in bytes."""
        total_min_size = self.OBJECT_BASE_SIZE
        total_max_size = self.OBJECT_BASE_SIZE
        for prop in object.properties:
            if prop.is_static or prop.is_runtime_only:
                continue
            min_size, max_size = self.size_type(
                prop.type,
                include_reference=False,
            )
            # property slot may hold value directly (primitives) or a reference (objects)
            ref_overhead = self.POINTER_SIZE if self._is_object(prop.type) else 0
            total_min_size += min_size + ref_overhead
            total_max_size += (max_size or min_size) + ref_overhead
        return ObjectSize(total_min_size, total_max_size)

    @override
    def size_type(self, type: "Type", include_reference: bool = True) -> ObjectSize:
        """Get the estimated size of a Type's value in bytes."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            size = self.size_type_scalar(type)
        # list (Array)
        elif type.cardinality == TypeCardinality.LIST:
            size = ObjectSize(self.ARRAY_BASE_SIZE, None)
        # tuple (fixed-length Array)
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            base = self.TUPLE_BASE_SIZE
            elem_sizes = [self.size_type(t) for t in type.element_types]
            min_size = base + sum(s.min_size for s in elem_sizes)
            if any(s.max_size is None for s in elem_sizes):
                max_size: int | None = None
            else:
                max_size = base + sum(s.max_size or 0 for s in elem_sizes)
            size = ObjectSize(min_size, max_size)
        # map (Map or plain object)
        elif type.cardinality == TypeCardinality.MAP:
            size = ObjectSize(self.MAP_BASE_SIZE, None)
        else:
            assert_never(type.cardinality)

        # if optional, min size can be 0
        if not type.is_required:
            size = ObjectSize(0, size.max_size or size.min_size)

        # add reference overhead when requested and when value is a reference in JS
        if include_reference and self._is_object(type):
            size = ObjectSize(
                size.min_size + self.POINTER_SIZE,
                (size.max_size or size.min_size) + self.POINTER_SIZE,
            )

        return size

    @override
    def size_type_scalar(self, type: "Type") -> ObjectSize:
        """Get the estimated size of a scalar type EXCLUDING any reference overhead."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return ObjectSize(0, 0)
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return ObjectSize(self.BOOLEAN_SIZE, self.BOOLEAN_SIZE)
            elif type.primitive_type in (
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
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                # JS numbers are IEEE-754 doubles
                return ObjectSize(self.NUMBER_SIZE, self.NUMBER_SIZE)
            elif type.primitive_type == PrimitiveType.DATETIME:
                return ObjectSize(self.TEMPORAL_INSTANT_SIZE, self.TEMPORAL_INSTANT_SIZE)
            elif type.primitive_type == PrimitiveType.DATE:
                return ObjectSize(self.TEMPORAL_DATE_SIZE, self.TEMPORAL_DATE_SIZE)
            elif type.primitive_type == PrimitiveType.TIME:
                return ObjectSize(self.TEMPORAL_TIME_SIZE, self.TEMPORAL_TIME_SIZE)
            elif type.primitive_type == PrimitiveType.DURATION:
                return ObjectSize(self.TEMPORAL_DURATION_SIZE, self.TEMPORAL_DURATION_SIZE)
            elif type.primitive_type == PrimitiveType.UUID:
                # 36 UTF-16 code units = 72 bytes
                uuid_bytes = 36 * 2
                return ObjectSize(uuid_bytes, uuid_bytes)
            elif type.primitive_type == PrimitiveType.BYTES:
                # ArrayBuffer with unknown payload length
                return ObjectSize(self.BYTES_EMPTY_SIZE, None)
            elif type.primitive_type == PrimitiveType.STRING:
                # dynamic payload length (UTF-16)
                return ObjectSize(self.STRING_EMPTY_SIZE, None)
            elif type.primitive_type == PrimitiveType.CHARACTER:
                # single UTF-16 code unit
                return ObjectSize(2, 2)
            elif type.primitive_type == PrimitiveType.JSON:
                # opaque
                return ObjectSize(0, 0)
            else:
                assert_never(type.primitive_type)

        # enum (assume small and shared)
        elif type.scalar_type == ScalarType.ENUM:
            return ObjectSize(0, 0)
        # node (reference)
        elif type.scalar_type == ScalarType.NODE:
            return ObjectSize(0, 0)
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            # treat as struct
            return self.size_object(STRUCT_DEFINITION_BY_TYPE[StructType.NODE_REFERENCE])
        # node id (UUID string)
        elif type.scalar_type == ScalarType.NODE_ID:
            uuid_bytes = 36 * 2
            return ObjectSize(uuid_bytes, uuid_bytes)
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

    def _is_object(self, type: "Type") -> bool:
        """Whether values of this Type are references in JS (and thus add pointer overhead)."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            assert type.scalar_type is not None, f"no scalar type for {type!r}"
            # primitives that are value-like in JS
            if type.scalar_type == ScalarType.PRIMITIVE:
                assert type.primitive_type is not None
                primitive = type.primitive_type
                if primitive == PrimitiveType.NONE:
                    return False
                elif primitive == PrimitiveType.BOOLEAN:
                    return False
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
                    PrimitiveType.FLOAT16,
                    PrimitiveType.FLOAT32,
                    PrimitiveType.FLOAT64,
                ):
                    return False
                elif primitive == PrimitiveType.STRING:
                    return False
                elif primitive == PrimitiveType.CHARACTER:
                    return False
                elif primitive == PrimitiveType.JSON:
                    return False
                elif primitive == PrimitiveType.UUID:
                    return False
                elif primitive == PrimitiveType.BYTES:
                    return True
                elif primitive == PrimitiveType.DATETIME:
                    return True
                elif primitive == PrimitiveType.DATE:
                    return True
                elif primitive == PrimitiveType.TIME:
                    return True
                elif primitive == PrimitiveType.DURATION:
                    return True
                else:
                    assert_never(primitive)
            # enums as shared singletons
            elif type.scalar_type == ScalarType.ENUM:
                return False
            # node values and structs are objects
            elif type.scalar_type in (
                ScalarType.NODE,
                ScalarType.STRUCT,
                ScalarType.NODE_REFERENCE,
                ScalarType.NODE_ID,
            ):
                return True
            # node id
            elif type.scalar_type == ScalarType.NODE_ID:
                return False
            # handle
            elif type.scalar_type == ScalarType.HANDLE:
                return True
            # union
            elif type.scalar_type == ScalarType.UNION:
                # treat union as value-like only if all elements are value-like
                assert type.element_types is not None
                return any(self._is_object(t) for t in type.element_types)
            else:
                assert_never(type.scalar_type)
        # collections are references
        elif type.cardinality in (TypeCardinality.LIST, TypeCardinality.TUPLE, TypeCardinality.MAP):
            return True
        else:
            assert_never(type.cardinality)
