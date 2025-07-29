import base64
import json
from collections.abc import Mapping
from datetime import UTC, date, datetime, time
from typing import Any, ClassVar, assert_never, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    Encoder,
    EncoderOptions,
    Encoding,
    Json,
    NodeType,
    Object,
    ObjectKind,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Session,
    StructType,
    Type,
    TypeCardinality,
)
from destack.language.registry import ENUM_CLASS_BY_TYPE
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

from .core import JsonObjectEncoder

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class JsonEncoder(Encoder[Json]):
    """Encoder for standard JSON format with proper names."""

    encoding: ClassVar[Encoding] = Encoding.JSON

    def __init__(
        self, encoders: Mapping[tuple[ObjectKind, NodeType | StructType], JsonObjectEncoder]
    ):
        self.encoders = encoders

    @classmethod
    def generate(cls) -> "JsonEncoder":
        from .generate import JsonEncoderGenerator

        generator = JsonEncoderGenerator()
        encoders = generator.generate()
        return cls(encoders)

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: Object,
        options: EncoderOptions,
    ) -> dict[str, Any]:
        encoder = self.encoders.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.pack_object(self, object, options)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: Object,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        encoder = self.encoders.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        object_packed = encoder.pack_object(self, object, options)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Json,
        session: Session | None,
        options: EncoderOptions,
    ) -> Object:
        encoder = self.encoders.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(self, value, session, options)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
    ) -> Object:
        encoder = self.encoders.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(self, value_decoded, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions,
    ) -> Json:
        return self.pack_object(ObjectKind.STRUCT, type.metatype, type, options)

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        self.pack_object_binary(ObjectKind.STRUCT, StructType.TYPE, type, writer, options)

    @override
    def unpack_type(
        self,
        value: Json,
        options: EncoderOptions,
    ) -> Type:
        unpacked_type = self.unpack_object(ObjectKind.STRUCT, StructType.TYPE, value, None, options)
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type

    @override
    def unpack_type_binary(
        self,
        reader: BinaryReader,
        options: EncoderOptions,
    ) -> Type:
        unpacked_type = self.unpack_object_binary(
            ObjectKind.STRUCT, StructType.TYPE, reader, None, options
        )
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type

    def get_target_property_key(self, prop: PropertyDeclaration) -> str:
        return str(prop.id)

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
        options: EncoderOptions,
    ) -> Json:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if value is None:
                return None
            else:
                return self.pack_scalar_value(type, value, options)

        # list
        elif type.cardinality == TypeCardinality.LIST:
            if value is None:
                return None
            else:
                assert type.value_type is not None, f"no value type for {type!r}"
                packed_list: list[Any] = []
                for item in value:
                    packed_list.append(self.pack_scalar_value(type.value_type, item, options))
                return packed_list

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            if value is None:
                return None
            else:
                assert type.element_types is not None, f"no element types for {type!r}"
                packed_tuple: list[Any] = []
                for item, element_type in zip(value, type.element_types):
                    packed_tuple.append(self.pack_scalar_value(element_type, item, options))
                return packed_tuple

        # map
        elif type.cardinality == TypeCardinality.MAP:
            if value is None:
                return None
            else:
                assert type.key_type is not None, f"no key type for {type!r}"
                assert type.value_type is not None, f"no value type for {type!r}"
                packed_map: dict[str, Any] = {}
                for key, val in value.items():
                    # always use string keys in JSON
                    packed_key = str(self.pack_scalar_value(type.key_type, key, options))
                    packed_val = self.pack_scalar_value(type.value_type, val, options)
                    packed_map[packed_key] = packed_val
                return packed_map

        else:
            assert_never(type.cardinality)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        writer.write_bytes(json.dumps(self.pack_value(type, value, options)).encode("utf-8"))

    @override
    def unpack_value(
        self,
        type: Type,
        value: Json,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if value is None:
                return None
            else:
                return self.unpack_scalar_value(type, value, session, options)

        # list
        elif type.cardinality == TypeCardinality.LIST:
            if value is None:
                return None
            else:
                assert type.value_type is not None, f"no value type for {type!r}"
                unpacked_list: list[Any] = []
                for item in value:
                    unpacked_list.append(
                        self.unpack_scalar_value(type.value_type, item, session, options)
                    )
                return unpacked_list

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            if value is None:
                return None
            else:
                assert type.element_types is not None, f"no element types for {type!r}"
                unpacked_tuple: list[Any] = []
                for item, element_type in zip(value, type.element_types):
                    unpacked_tuple.append(
                        self.unpack_scalar_value(element_type, item, session, options)
                    )
                return unpacked_tuple

        # map
        elif type.cardinality == TypeCardinality.MAP:
            if value is None:
                return None
            else:
                assert type.key_type is not None, f"no key type for {type!r}"
                assert type.value_type is not None, f"no value type for {type!r}"
                unpacked_map = {}
                assert isinstance(value, dict), f"expected dict for map type, got {type_(value)}"
                for key, val in value.items():
                    unpacked_key = (
                        self.unpack_scalar_value(type.key_type, key, session, options)
                        if type.key_type
                        else key
                    )
                    unpacked_val = self.unpack_scalar_value(type.value_type, val, session, options)
                    unpacked_map[unpacked_key] = unpacked_val
                return unpacked_map

        # literal
        else:
            assert_never(type.cardinality)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return self.unpack_value(type, value_decoded, session, options)

    def pack_scalar_value(self, type: Type, value: Any, options: EncoderOptions) -> Any:
        """Pack a scalar value to JSON."""
        assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return None
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return value
            elif (
                type.primitive_type
                in (
                    PrimitiveType.INT8,
                    PrimitiveType.INT16,
                    PrimitiveType.INT32,
                    PrimitiveType.INT64,
                    PrimitiveType.INT128,
                )
                or type.primitive_type
                in (
                    PrimitiveType.UINT8,
                    PrimitiveType.UINT16,
                    PrimitiveType.UINT32,
                    PrimitiveType.UINT64,
                    PrimitiveType.UINT128,
                )
                or type.primitive_type
                in (
                    PrimitiveType.FLOAT16,
                    PrimitiveType.FLOAT32,
                    PrimitiveType.FLOAT64,
                )
            ):
                return float(value)
            elif type.primitive_type == PrimitiveType.DATETIME:
                return value.astimezone(UTC).isoformat()
            elif type.primitive_type == PrimitiveType.DATE:
                return value.isoformat()
            elif type.primitive_type == PrimitiveType.TIME:
                return value.astimezone(UTC).replace(tzinfo=None).isoformat()
            elif type.primitive_type == PrimitiveType.DURATION:
                return timedelta_to_isoformat(value)
            elif type.primitive_type == PrimitiveType.STRING:
                return value
            elif type.primitive_type == PrimitiveType.UUID:
                return str(value)
            elif type.primitive_type == PrimitiveType.BYTES:
                return base64.b64encode(value).decode()
            elif type.primitive_type == PrimitiveType.JSON:
                return value
            else:
                assert_never(type.primitive_type)

        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return value.name

        # node reference, node value, struct
        elif type.scalar_type in (
            ScalarType.NODE_REFERENCE,
            ScalarType.NODE_VALUE,
            ScalarType.STRUCT,
        ):
            assert type.struct_type is not None, f"no struct type for {type!r}"
            assert isinstance(value, Object), f"expected BuiltinObject for {type!r}, got {value!r}"
            return self.pack_object(ObjectKind.STRUCT, type.struct_type, value, options)

        #
        else:
            assert_never(type.scalar_type)

    def unpack_scalar_value(
        self,
        type: Type,
        value: Any,
        session: "Session | None",
        options: EncoderOptions,
    ) -> Any:
        """Unpack a scalar value from JSON."""
        assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return None
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return value
            elif type.primitive_type in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
            ) or type.primitive_type in (
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            ):
                return int(value)
            elif type.primitive_type in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                return float(value)
            elif type.primitive_type == PrimitiveType.DATETIME:
                return datetime.fromisoformat(value).astimezone(UTC)
            elif type.primitive_type == PrimitiveType.DATE:
                return date.fromisoformat(value)
            elif type.primitive_type == PrimitiveType.TIME:
                return time.fromisoformat(value).replace(tzinfo=None)
            elif type.primitive_type == PrimitiveType.DURATION:
                return timedelta_from_isoformat(value)
            elif type.primitive_type == PrimitiveType.STRING:
                return value
            elif type.primitive_type == PrimitiveType.UUID:
                return UUID(value)
            elif type.primitive_type == PrimitiveType.BYTES:
                return base64.b64decode(value)
            elif type.primitive_type == PrimitiveType.JSON:
                return value
            else:
                assert_never(type.primitive_type)

        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return enum_cls[value]

        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return self.unpack_object(
                ObjectKind.STRUCT, StructType.NODE_REFERENCE, value, session, options
            )

        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            node_type = NodeType[value["metatype"]]
            return self.unpack_object(ObjectKind.NODE, node_type, value, session, options)

        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            return self.unpack_object(ObjectKind.STRUCT, type.struct_type, value, session, options)

        #
        else:
            assert_never(type.scalar_type)
