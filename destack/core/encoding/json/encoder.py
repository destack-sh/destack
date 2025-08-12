import base64
import json
from collections.abc import Mapping
from datetime import UTC, date, datetime, time
from typing import TYPE_CHECKING, Any, assert_never, cast, override

from destack.registry import ENUM_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ...builtin import (
    UUID,
    Enum,
    Json,
    NodeType,
    Object,
    ObjectKind,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    StringCasing,
    StructType,
    TypeCardinality,
    to_casing,
)
from ...common import Type, Value
from ..encoder import BinaryReader, BinaryWriter, Encoder, EncoderFlag
from ..time import timedelta_from_isoformat, timedelta_to_isoformat
from .core import JsonObjectEncoder

if TYPE_CHECKING:
    from destack import Session

type_ = type

METAKIND_PROPERTY = Object.__properties__["metakind"]
METATYPE_PROPERTY = Object.__properties__["metatype"]

VALUE_TYPE_PROPERTY = Value.__properties__["type"]
VALUE_VALUE_PROPERTY = Value.__properties__["value"]


class JsonEncoder(Encoder):
    """Encoder for standard JSON format with proper names."""

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
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> dict[str, Any]:
        # object
        type = (object.metakind, object.metatype)
        encoder = self.encoders.get(type)
        assert encoder is not None, f"no JsonObjectEncoder for {type!r}"
        packed_object = encoder.pack_object(self, object, options)
        # metatype
        if not options & EncoderFlag.OMIT_METATYPE:
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
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        # key
        key: tuple[ObjectKind, int]
        if kind is None or type is None:
            metakind_key = self.get_target_property_key(METAKIND_PROPERTY)
            metakind = cast(ObjectKind, self.unpack_scalar_enum(ObjectKind, value[metakind_key]))
            if metakind == ObjectKind.NODE:
                metatype_key = self.get_target_property_key(METATYPE_PROPERTY)
                metatype = self.unpack_scalar_enum(NodeType, value[metatype_key])
                key = (metakind, metatype)
            elif metakind == ObjectKind.STRUCT:
                metatype_key = self.get_target_property_key(METATYPE_PROPERTY)
                metatype = self.unpack_scalar_enum(StructType, value[metatype_key])
                key = (metakind, metatype)
            elif metakind == ObjectKind.HANDLE or metakind == ObjectKind.MODULE:
                raise NotImplementedError(f"cannot unpack {metakind!r}")
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
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        object_packed = self.pack_object(object, options)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: int | None,
        reader: BinaryReader,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return self.unpack_object(kind, type, value_decoded, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Json:
        return self.pack_object(type, options)

    @override
    def unpack_type(
        self,
        value: Json,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Type:
        unpacked_type = self.unpack_object(ObjectKind.STRUCT, StructType.TYPE, value, None, options)
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        self.pack_object_binary(type, writer, options)

    @override
    def unpack_type_binary(
        self,
        reader: BinaryReader,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Type:
        unpacked_type = self.unpack_object_binary(
            ObjectKind.STRUCT, StructType.TYPE, reader, None, options
        )
        assert isinstance(unpacked_type, Type), f"expected Type, got {unpacked_type!r}"
        return unpacked_type

    def get_target_property_key(self, prop: PropertyDeclaration) -> str:
        return to_casing(prop.name, StringCasing.LOWER_CAMEL)

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Json:
        if value is None:
            return None
        inner_options = options & EncoderFlag.OMIT_NONE
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
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        if value is None:
            return None
        inner_options = options & EncoderFlag.OMIT_NONE
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
        options: EncoderFlag,
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
            elif type.primitive_type == PrimitiveType.TIMESTAMP:
                return value
            elif type.primitive_type == PrimitiveType.DURATION:
                return timedelta_to_isoformat(value)
            elif type.primitive_type == PrimitiveType.STRING:
                return value
            elif type.primitive_type == PrimitiveType.CHARACTER:
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
        # node
        elif type.scalar_type == ScalarType.NODE:
            return self.pack_object(value, options & ~EncoderFlag.OMIT_METATYPE)
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return str(value)
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return str(value.id)
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return self.pack_object(value, options)
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return self.pack_object(value, options)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return self.pack_object(value, options)
            else:
                return self.pack_object(value, options & ~EncoderFlag.OMIT_METATYPE)
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot pack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot pack UNION: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    def unpack_scalar_value(
        self,
        type: Type,
        value: Any,
        session: "Session | None",
        options: EncoderFlag,
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
            elif type.primitive_type == PrimitiveType.TIMESTAMP:
                return value
            elif type.primitive_type == PrimitiveType.DURATION:
                return timedelta_from_isoformat(value)
            elif type.primitive_type == PrimitiveType.STRING:
                return value
            elif type.primitive_type == PrimitiveType.CHARACTER:
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
        # node
        elif type.scalar_type == ScalarType.NODE:
            return self.unpack_object(
                None, None, value, session, options & ~EncoderFlag.OMIT_METATYPE
            )
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return UUID(value)
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return self.unpack_object(
                ObjectKind.STRUCT, StructType.NODE_IDENTITY_REFERENCE, value, session, options
            )
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return self.unpack_object(
                ObjectKind.STRUCT, StructType.NODE_SPATIAL_REFERENCE, value, session, options
            )
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return self.unpack_object(
                ObjectKind.STRUCT, StructType.NODE_TEMPORAL_REFERENCE, value, session, options
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
                    None, None, value, session, options & ~EncoderFlag.OMIT_METATYPE
                )
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot unpack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot unpack UNION: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    def pack_scalar_enum(self, enum_cls: type[Enum], value: Any) -> Any:
        """Pack an enum value to JSON."""
        return value.name

    def unpack_scalar_enum[T: Enum](self, enum_cls: type[T], value: Any) -> T:
        """Unpack an enum value from JSON."""
        return enum_cls(value)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        writer.write_bytes(json.dumps(self.pack_value(type, value, options)).encode("utf-8"))

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
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
        _options: EncoderFlag,
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
        _session: "Session | None",
        _options: EncoderFlag,
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
