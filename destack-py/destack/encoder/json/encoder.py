import base64
import json
from collections.abc import Mapping
from datetime import UTC, date, datetime, time
from enum import Enum
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
from destack.language.registry import ENUM_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.utils.string import Casing, to_casing
from destack.utils.time import timedelta_from_isoformat, timedelta_to_isoformat
from destack.utils.uuid import UUID

from .core import JsonObjectEncoder

type_ = type

METAKIND_PROPERTY = Object.__properties__["metakind"]
METATYPE_PROPERTY = Object.__properties__["metatype"]

VALUE_TYPE_PROPERTY = Value.__properties__["type"]
VALUE_VALUE_PROPERTY = Value.__properties__["value"]


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
        encoders[ObjectKind.STRUCT, StructType.VALUE] = cast(JsonObjectEncoder, JsonValueEncoder())
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def pack_object(
        self,
        object: Object,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> dict[str, Any]:
        # object
        type = (object.metakind, object.metatype)
        encoder = self.encoders.get(type)
        assert encoder is not None, f"no JsonObjectEncoder for {type!r}"
        packed_object = encoder.pack_object(self, object, options)
        # metatype
        if not options & EncoderOptions.OMIT_METATYPE:
            metakind_key = self.get_target_property_key(METAKIND_PROPERTY)
            metatype_key = self.get_target_property_key(METATYPE_PROPERTY)
            packed_object[metakind_key] = self.pack_scalar_enum(ObjectKind, object.metakind)
            packed_object[metatype_key] = self.pack_scalar_enum(
                type_(object.metatype), object.metatype
            )
        return packed_object

    @override
    def unpack_object(
        self,
        kind: ObjectKind | None,
        type: int | None,
        value: Json,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        # key
        key: tuple[ObjectKind, int]
        if kind is None or type is None:
            metakind_key = self.get_target_property_key(METAKIND_PROPERTY)
            metakind = self.unpack_scalar_enum(ObjectKind, value[metakind_key])
            if metakind == ObjectKind.NODE:
                metatype_key = self.get_target_property_key(METATYPE_PROPERTY)
                metatype = self.unpack_scalar_enum(NodeType, value[metatype_key])
                key = (metakind, metatype)
            elif metakind == ObjectKind.STRUCT:
                metatype_key = self.get_target_property_key(METATYPE_PROPERTY)
                metatype = self.unpack_scalar_enum(StructType, value[metatype_key])
                key = (metakind, metatype)
            elif metakind == ObjectKind.HANDLE:
                raise NotImplementedError(f"cannot unpack Handle: {metakind!r}")
            else:
                assert_never(metakind)
        else:
            key = (kind, type)
        # object
        encoder = self.encoders.get(key)
        assert encoder is not None, f"no JsonObjectEncoder for {key!r}"
        return encoder.unpack_object(self, value, session, options)

    @override
    def pack_object_binary(
        self,
        object: Object,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        object_packed = self.pack_object(object, options)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: int | None,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return self.unpack_object(kind, type, value_decoded, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Json:
        return self.pack_object(type, options)

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
        self.pack_object_binary(type, writer, options)

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
        return to_casing(prop.name, Casing.LOWER_CAMEL)

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
            return tuple(unpacked_tuple)
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
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return self.pack_scalar_enum(enum_cls, value)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return self.pack_object(value, options)
            else:
                return self.pack_object(value, options & ~EncoderOptions.OMIT_METATYPE)
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return self.pack_object(value, options)
        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return self.pack_object(value, options & ~EncoderOptions.OMIT_METATYPE)
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot pack Handle: {type!r}")
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
            return self.unpack_scalar_enum(enum_cls, value)
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return self.unpack_object(
                ObjectKind.STRUCT, StructType.NODE_REFERENCE, value, session, options
            )
        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return self.unpack_object(
                None, None, value, session, options & ~EncoderOptions.OMIT_METATYPE
            )
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return self.unpack_object(
                    ObjectKind.STRUCT, type.struct_type, value, session, options
                )
            else:
                return self.unpack_object(
                    None, None, value, session, options & ~EncoderOptions.OMIT_METATYPE
                )
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot unpack Handle: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    def pack_scalar_enum(self, enum_cls: type[Enum], value: Any) -> Any:
        """Pack an enum value to JSON."""
        return value.name

    def unpack_scalar_enum(self, enum_cls: type[Enum], value: Any) -> Any:
        """Unpack an enum value from JSON."""
        return enum_cls[value]

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


class JsonValueEncoder(JsonObjectEncoder[Value]):
    """Short-circuit Value encoding to the JsonEncoder's own methods."""

    @override
    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: Value,
        _options: EncoderOptions,
    ) -> dict[str, Any]:
        type_key = _encoder.get_target_property_key(VALUE_TYPE_PROPERTY)
        value_key = _encoder.get_target_property_key(VALUE_VALUE_PROPERTY)
        return {
            type_key: _encoder.pack_object(_object.type, _options),
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
        type_key = _encoder.get_target_property_key(VALUE_TYPE_PROPERTY)
        value_key = _encoder.get_target_property_key(VALUE_VALUE_PROPERTY)
        unpacked_type = _encoder.unpack_object(
            ObjectKind.STRUCT, StructType.TYPE, _object_json[type_key], _session, _options
        )
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        unpacked_value = _encoder.unpack_value(
            unpacked_type, _object_json[value_key], _session, _options
        )
        value = Value(type=unpacked_type, value=unpacked_value)
        return value
