import base64
import json
from collections.abc import Mapping
from datetime import UTC, date, datetime, time
from typing import Any, ClassVar, assert_never, cast, override

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
    Value,
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

    def __init__(self, encoders: Mapping[tuple[ObjectKind, int], JsonObjectEncoder]):
        self.encoders = encoders

    @classmethod
    def generate(cls) -> "JsonEncoder":
        from .generate import JsonEncoderGenerator

        generator = JsonEncoderGenerator()
        encoders: dict[tuple[ObjectKind, int], JsonObjectEncoder] = {}
        encoders[ObjectKind.STRUCT, StructType.TYPE] = cast(JsonObjectEncoder, JsonTypeEncoder())
        encoders[ObjectKind.STRUCT, StructType.VALUE] = cast(JsonObjectEncoder, JsonValueEncoder())
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        type: int,
        object: Object,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> dict[str, Any]:
        encoder = self.encoders.get((kind, type))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{type}"
        return encoder.pack_object(self, object, options)

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        type: int,
        value: Json,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        encoder = self.encoders.get((kind, type))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{type}"
        return encoder.unpack_object(self, value, session, options)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        type: NodeType | StructType,
        object: Object,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        encoder = self.encoders.get((kind, type))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{type}"
        object_packed = encoder.pack_object(self, object, options)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        type: int,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        encoder = self.encoders.get((kind, type))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{type}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(self, value_decoded, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Json:
        return self.pack_object(ObjectKind.STRUCT, type.metatype, type, options)

    @override
    def unpack_type(
        self,
        value: Json,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Type:
        unpacked_type = self.unpack_object(ObjectKind.STRUCT, StructType.TYPE, value, None, options)
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        self.pack_object_binary(ObjectKind.STRUCT, StructType.TYPE, type, writer, options)

    @override
    def unpack_type_binary(
        self,
        reader: BinaryReader,
        options: EncoderOptions = EncoderOptions.DEFAULT,
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
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Json:
        if value is None:
            return None
        inner_options = options & EncoderOptions.OMIT_NONE
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            return self.pack_scalar_value(type, value, options)
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_list: list[Any] = []
            for item in value:
                packed_list.append(self.pack_scalar_value(type.value_type, item, inner_options))
            return packed_list
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            packed_tuple: list[Any] = []
            for item, element_type in zip(value, type.element_types):
                packed_tuple.append(self.pack_scalar_value(element_type, item, inner_options))
            return packed_tuple
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            packed_map: dict[str, Any] = {}
            for key, val in value.items():
                # always use string keys in JSON
                packed_key = str(self.pack_scalar_value(type.key_type, key, inner_options))
                packed_val = self.pack_scalar_value(type.value_type, val, inner_options)
                packed_map[packed_key] = packed_val
            return packed_map
        #
        else:
            assert_never(type.cardinality)

    @override
    def unpack_value(
        self,
        type: Type,
        value: Json,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        if value is None:
            return None
        inner_options = options & EncoderOptions.OMIT_NONE
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            return self.unpack_scalar_value(type, value, session, inner_options)
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_list: list[Any] = []
            for item in value:
                unpacked_list.append(
                    self.unpack_scalar_value(type.value_type, item, session, inner_options)
                )
            return unpacked_list
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            unpacked_tuple: list[Any] = []
            for item, element_type in zip(value, type.element_types):
                unpacked_tuple.append(
                    self.unpack_scalar_value(element_type, item, session, inner_options)
                )
            return unpacked_tuple
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            unpacked_map = {}
            assert isinstance(value, dict), f"expected dict for map type, got {type_(value)}"
            for key, val in value.items():
                unpacked_key = (
                    self.unpack_scalar_value(type.key_type, key, session, inner_options)
                    if type.key_type
                    else key
                )
                unpacked_val = self.unpack_scalar_value(
                    type.value_type, val, session, inner_options
                )
                unpacked_map[unpacked_key] = unpacked_val
            return unpacked_map
        #
        else:
            assert_never(type.cardinality)

    def pack_scalar_value(
        self,
        type: Type,
        value: Any,
        options: EncoderOptions,
    ) -> Any:
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

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        writer.write_bytes(json.dumps(self.pack_value(type, value, options)).encode("utf-8"))

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return self.unpack_value(type, value_decoded, session, options)


class JsonTypeEncoder(JsonObjectEncoder[Type]):
    """Short-circuit Type encoding to the JsonEncoder's own methods."""

    @override
    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: Type,
        _options: EncoderOptions,
    ) -> dict[str, Any]:
        return _encoder.pack_object(ObjectKind.STRUCT, StructType.TYPE, _object, _options)

    @override
    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: dict[str, Any],
        _session: Session | None,
        _options: EncoderOptions,
    ) -> Type:
        unpacked_type = _encoder.unpack_object(
            ObjectKind.STRUCT, StructType.TYPE, _object_json, _session, _options
        )
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type


class JsonValueEncoder(JsonObjectEncoder[Value]):
    """Short-circuit Value encoding to the JsonEncoder's own methods."""

    @override
    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: Value,
        _options: EncoderOptions,
    ) -> dict[str, Any]:
        type_key = _encoder.get_target_property_key(Value.__properties__["type"])
        value_key = _encoder.get_target_property_key(Value.__properties__["value"])
        return {
            type_key: _encoder.pack_object(
                ObjectKind.STRUCT, StructType.TYPE, _object.type, _options
            ),
            value_key: _encoder.pack_value(_object.type, _object.value, _options),
        }

    @override
    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: dict[str, Any],
        _session: Session | None,
        _options: EncoderOptions,
    ) -> Value:
        type_key = _encoder.get_target_property_key(Value.__properties__["type"])
        value_key = _encoder.get_target_property_key(Value.__properties__["value"])
        unpacked_type = _encoder.unpack_object(
            ObjectKind.STRUCT, StructType.TYPE, _object_json[type_key], _session, _options
        )
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        unpacked_value = _encoder.unpack_value(
            unpacked_type, _object_json[value_key], _session, _options
        )
        assert isinstance(unpacked_value, Value), f"expected Value, got {unpacked_value!r}"
        return unpacked_value
