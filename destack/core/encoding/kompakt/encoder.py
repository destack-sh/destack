from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, assert_never, cast, override

from destack.registry import ENUM_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE

from ...builtin import (
    UUID,
    EnumType,
    NodeType,
    Object,
    ObjectKind,
    PrimitiveType,
    ScalarType,
    StructType,
    TypeCardinality,
)
from ...common import Type, Value
from ..encoder import BinaryReader, BinaryWriter, Encoder, EncoderFlag
from .core import KompaktObjectEncoder

if TYPE_CHECKING:
    from destack import Session

_UUID_NULL = UUID(int=0)


class KompaktEncoder(Encoder):
    """Encoder for our Kompakt format."""

    def __init__(self, encoders: Mapping[tuple[ObjectKind, int], KompaktObjectEncoder]):
        self.encoders = encoders

    @classmethod
    def generate(cls) -> "KompaktEncoder":
        from .generate import KompaktEncoderGenerator

        generator = KompaktEncoderGenerator()
        encoders: dict[tuple[ObjectKind, int], KompaktObjectEncoder] = {}
        encoders[ObjectKind.STRUCT, StructType.TYPE] = cast(
            KompaktObjectEncoder, KompaktTypeEncoder()
        )
        encoders[ObjectKind.STRUCT, StructType.VALUE] = cast(
            KompaktObjectEncoder, KompaktValueEncoder()
        )
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def pack_object(
        self,
        object: Object,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> bytes:
        writer = BinaryWriter()
        object_bytes = writer.to_bytes()
        return object_bytes

    @override
    def unpack_object(
        self,
        kind: ObjectKind | None,
        type: int | None,
        value: bytes,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        reader = BinaryReader(buffer=value)
        object = self.unpack_object_binary(kind, type, reader, session, options)
        return object

    @override
    def pack_object_binary(
        self,
        object: Object,
        writer: BinaryWriter,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        # metatype
        if not options & EncoderFlag.OMIT_METATYPE:
            writer.write_uint8(object.metakind)
            writer.write_uint32(object.metatype)
        # object
        key = (object.metakind, object.metatype)
        encoder = self.encoders.get(key)
        assert encoder is not None, f"no KompaktObjectEncoder for {key!r}"
        encoder.pack_object(self, object, writer, options)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: int | None,
        reader: BinaryReader,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        # metatype
        if kind is None or type is None:
            metakind = ObjectKind(reader.read_uint8())
            metatype = reader.read_uint32()
            key = (metakind, metatype)
        else:
            key = (kind, type)
        # object
        encoder = self.encoders.get(key)
        assert encoder is not None, f"no KompaktObjectEncoder for {key!r}"
        return encoder.unpack_object(self, reader, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> bytes:
        writer = BinaryWriter()
        self.pack_type_binary(type, writer, options)
        type_bytes = writer.to_bytes()
        return type_bytes

    @override
    def unpack_type(
        self,
        value: bytes,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Type:
        reader = BinaryReader(buffer=value)
        type = self.unpack_type_binary(reader, options)
        return type

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        # preamble (4 bits cardinality, 4 bits scalar type)
        writer.write_uint8(type.cardinality | ((type.scalar_type or 0) << 4))
        # scalar (type folded into preamble)
        if type.cardinality == TypeCardinality.SCALAR:
            assert type.scalar_type is not None, f"no scalar type for {type!r}"
            # primitive
            if type.scalar_type == ScalarType.PRIMITIVE:
                assert type.primitive_type is not None, f"no primitive type for {type!r}"
                writer.write_uint8(type.primitive_type)
            # enum
            elif type.scalar_type == ScalarType.ENUM:
                assert type.enum_type is not None, f"no enum type for {type!r}"
                writer.write_uint32(type.enum_type)
            # node
            elif type.scalar_type in (
                ScalarType.NODE,
                ScalarType.NODE_RAW,
                ScalarType.NODE_IDENTITY,
                ScalarType.NODE_SPATIAL,
                ScalarType.NODE_TEMPORAL,
            ):
                assert type.node_types is not None, f"no node types for {type!r}"
                writer.write_uint32(len(type.node_types))
                for node_type in type.node_types:
                    writer.write_uint32(node_type)
            # struct
            elif type.scalar_type == ScalarType.STRUCT:
                assert type.struct_type is not None, f"no struct type for {type!r}"
                writer.write_uint32(type.struct_type)
            elif type.scalar_type == ScalarType.HANDLE:
                raise NotImplementedError(f"cannot pack HANDLE: {type!r}")
            elif type.scalar_type == ScalarType.UNION:
                assert type.element_types is not None, f"no element types for {type!r}"
                writer.write_uint32(len(type.element_types))
                for element_type in type.element_types:
                    self.pack_type_binary(element_type, writer, options)
            else:
                assert_never(type.scalar_type)
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            self.pack_type_binary(type.value_type, writer, options)
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            writer.write_uint32(len(type.element_types))
            for element_type in type.element_types:
                self.pack_type_binary(element_type, writer, options)
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            self.pack_type_binary(type.key_type, writer, options)
            self.pack_type_binary(type.value_type, writer, options)
        #
        else:
            assert_never(type.cardinality)

    @override
    def unpack_type_binary(
        self,
        reader: BinaryReader,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Type:
        # preamble (4 bits cardinality, 4 bits scalar type)
        preamble = reader.read_uint8()
        cardinality = TypeCardinality(preamble & 0b1111)
        scalar_type_bits = (preamble >> 4) & 0b1111
        scalar_type = ScalarType(scalar_type_bits) if scalar_type_bits else None
        key_type: Type | None = None
        value_type: Type | None = None
        element_types: list[Type] | None = None
        node_types: list[NodeType] | None = None
        struct_type: StructType | None = None
        primitive_type: PrimitiveType | None = None
        enum_type: EnumType | None = None
        # scalar (type folded into preamble)
        if cardinality == TypeCardinality.SCALAR:
            assert scalar_type is not None, f"no scalar type for {cardinality}"
            # primitive
            if scalar_type == ScalarType.PRIMITIVE:
                primitive_type = PrimitiveType(reader.read_uint8())
            # enum
            elif scalar_type == ScalarType.ENUM:
                enum_type = EnumType(reader.read_uint32())
            # node
            elif scalar_type in (
                ScalarType.NODE,
                ScalarType.NODE_RAW,
                ScalarType.NODE_IDENTITY,
                ScalarType.NODE_SPATIAL,
                ScalarType.NODE_TEMPORAL,
            ):
                node_types = [NodeType(reader.read_uint32()) for _ in range(reader.read_uint32())]
            # struct
            elif scalar_type == ScalarType.STRUCT:
                struct_type = StructType(reader.read_uint32())
            elif scalar_type == ScalarType.HANDLE:
                raise NotImplementedError(f"cannot unpack HANDLE: {scalar_type!r}")
            elif scalar_type == ScalarType.UNION:
                element_types = [
                    self.unpack_type_binary(reader, options) for _ in range(reader.read_uint32())
                ]
            else:
                assert_never(scalar_type)
        # list
        elif cardinality == TypeCardinality.LIST:
            value_type = self.unpack_type_binary(reader, options)
        # tuple
        elif cardinality == TypeCardinality.TUPLE:
            element_types = [
                self.unpack_type_binary(reader, options) for _ in range(reader.read_uint32())
            ]
        # map
        elif cardinality == TypeCardinality.MAP:
            key_type = self.unpack_type_binary(reader, options)
            value_type = self.unpack_type_binary(reader, options)
        #
        else:
            assert_never(cardinality)
        return Type(
            # cardinality
            cardinality=cardinality,
            value_type=value_type,
            element_types=element_types,
            key_type=key_type,
            # scalar
            scalar_type=scalar_type,
            primitive_type=primitive_type,
            enum_type=enum_type,
            node_types=node_types,
            struct_type=struct_type,
        )

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        writer = BinaryWriter()
        self.pack_value_binary(type, value, writer, options)
        value_bytes = writer.to_bytes()
        return value_bytes

    @override
    def unpack_value(
        self,
        type: Type,
        value: Any,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        reader = BinaryReader(buffer=value)
        return self.unpack_value_binary(type, reader, session, options)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            self.pack_scalar_value_binary(type, value, writer, options | EncoderFlag.OMIT_METATYPE)
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            writer.write_uint32(len(value))
            for item in value:
                self.pack_value_binary(
                    type.value_type, item, writer, options | EncoderFlag.OMIT_METATYPE
                )
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            writer.write_uint32(len(value))
            for item, element_type in zip(value, type.element_types):
                self.pack_value_binary(
                    element_type, item, writer, options | EncoderFlag.OMIT_METATYPE
                )
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            writer.write_uint32(len(value))
            for key, val in value.items():
                self.pack_value_binary(
                    type.key_type, key, writer, options | EncoderFlag.OMIT_METATYPE
                )
                self.pack_value_binary(
                    type.value_type, val, writer, options | EncoderFlag.OMIT_METATYPE
                )
        #
        else:
            assert_never(type.cardinality)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: "Session | None",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            return self.unpack_scalar_value_binary(
                type, reader, session, options | EncoderFlag.OMIT_METATYPE
            )
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            length = reader.read_uint32()
            return [
                self.unpack_value_binary(
                    type.value_type, reader, session, options | EncoderFlag.OMIT_METATYPE
                )
                for _ in range(length)
            ]
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            length = reader.read_uint32()
            return tuple(
                self.unpack_value_binary(
                    element_type, reader, session, options | EncoderFlag.OMIT_METATYPE
                )
                for element_type in type.element_types
            )
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            length = reader.read_uint32()
            return {
                self.unpack_value_binary(
                    type.key_type, reader, session, options | EncoderFlag.OMIT_METATYPE
                ): self.unpack_value_binary(
                    type.value_type, reader, session, options | EncoderFlag.OMIT_METATYPE
                )
                for _ in range(length)
            }
        #
        else:
            assert_never(type.cardinality)

    def pack_scalar_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderFlag,
    ) -> None:
        assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"
        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                pass
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                writer.write_bool(value)
            elif type.primitive_type == PrimitiveType.INT8:
                writer.write_int8(value)
            elif type.primitive_type == PrimitiveType.INT16:
                writer.write_int16(value)
            elif type.primitive_type == PrimitiveType.INT32:
                writer.write_int32(value)
            elif type.primitive_type == PrimitiveType.INT64:
                writer.write_int64(value)
            elif type.primitive_type == PrimitiveType.INT128:
                writer.write_int128(value)
            elif type.primitive_type == PrimitiveType.UINT8:
                writer.write_uint8(value)
            elif type.primitive_type == PrimitiveType.UINT16:
                writer.write_uint16(value)
            elif type.primitive_type == PrimitiveType.UINT32:
                writer.write_uint32(value)
            elif type.primitive_type == PrimitiveType.UINT64:
                writer.write_uint64(value)
            elif type.primitive_type == PrimitiveType.UINT128:
                writer.write_uint128(value)
            elif type.primitive_type == PrimitiveType.FLOAT16:
                writer.write_float16(value)
            elif type.primitive_type == PrimitiveType.FLOAT32:
                writer.write_float32(value)
            elif type.primitive_type == PrimitiveType.FLOAT64:
                writer.write_float64(value)
            elif type.primitive_type == PrimitiveType.DATETIME:
                writer.write_datetime(value)
            elif type.primitive_type == PrimitiveType.DATE:
                writer.write_date(value)
            elif type.primitive_type == PrimitiveType.TIME:
                writer.write_time(value)
            elif type.primitive_type == PrimitiveType.DURATION:
                writer.write_duration(value)
            elif type.primitive_type == PrimitiveType.STRING:
                writer.write_string(value)
            elif type.primitive_type == PrimitiveType.CHARACTER:
                writer.write_character(value)
            elif type.primitive_type == PrimitiveType.UUID:
                writer.write_uuid(value)
            elif type.primitive_type == PrimitiveType.BYTES:
                writer.write_bytes(value)
            elif type.primitive_type == PrimitiveType.JSON:
                writer.write_json(value)
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            writer.write_uint32(value)
        # node
        elif type.scalar_type == ScalarType.NODE:
            self.pack_object_binary(value, writer, options & ~EncoderFlag.OMIT_METATYPE)
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            writer.write_uuid(value.id)
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            self.pack_object_binary(value, writer, options | EncoderFlag.OMIT_METATYPE)
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            self.pack_object_binary(value, writer, options | EncoderFlag.OMIT_METATYPE)
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            self.pack_object_binary(value, writer, options | EncoderFlag.OMIT_METATYPE)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                self.pack_object_binary(value, writer, options | EncoderFlag.OMIT_METATYPE)
            else:
                self.pack_object_binary(value, writer, options & ~EncoderFlag.OMIT_METATYPE)
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot pack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot pack UNION: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    def unpack_scalar_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: "Session | None",
        options: EncoderFlag,
    ) -> Any:
        """Unpack a scalar value from binary."""
        assert type.cardinality == TypeCardinality.SCALAR, f"expected scalar type, got {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"
        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return None
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return reader.read_bool()
            elif type.primitive_type == PrimitiveType.INT8:
                return reader.read_int8()
            elif type.primitive_type == PrimitiveType.INT16:
                return reader.read_int16()
            elif type.primitive_type == PrimitiveType.INT32:
                return reader.read_int32()
            elif type.primitive_type == PrimitiveType.INT64:
                return reader.read_int64()
            elif type.primitive_type == PrimitiveType.INT128:
                return reader.read_int128()
            elif type.primitive_type == PrimitiveType.UINT8:
                return reader.read_uint8()
            elif type.primitive_type == PrimitiveType.UINT16:
                return reader.read_uint16()
            elif type.primitive_type == PrimitiveType.UINT32:
                return reader.read_uint32()
            elif type.primitive_type == PrimitiveType.UINT64:
                return reader.read_uint64()
            elif type.primitive_type == PrimitiveType.UINT128:
                return reader.read_uint128()
            elif type.primitive_type == PrimitiveType.FLOAT16:
                return reader.read_float16()
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return reader.read_float32()
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return reader.read_float64()
            elif type.primitive_type == PrimitiveType.DATETIME:
                return reader.read_datetime()
            elif type.primitive_type == PrimitiveType.DATE:
                return reader.read_date()
            elif type.primitive_type == PrimitiveType.TIME:
                return reader.read_time()
            elif type.primitive_type == PrimitiveType.DURATION:
                return reader.read_duration()
            elif type.primitive_type == PrimitiveType.STRING:
                return reader.read_string()
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return reader.read_character()
            elif type.primitive_type == PrimitiveType.UUID:
                return reader.read_uuid()
            elif type.primitive_type == PrimitiveType.BYTES:
                return reader.read_bytes()
            elif type.primitive_type == PrimitiveType.JSON:
                return reader.read_json()
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            enum_value = reader.read_uint32()
            return enum_cls.__options_by_id__[enum_value]
        # node
        elif type.scalar_type == ScalarType.NODE:
            return self.unpack_object_binary(
                None,
                None,
                reader,
                session,
                options | EncoderFlag.OMIT_METATYPE,
            )
        # node reference
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return self.unpack_object_binary(
                ObjectKind.STRUCT,
                StructType.NODE_TEMPORAL_REFERENCE,
                reader,
                session,
                options | EncoderFlag.OMIT_METATYPE,
            )
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return reader.read_uuid()
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return self.unpack_object_binary(
                ObjectKind.STRUCT,
                StructType.NODE_IDENTITY_REFERENCE,
                reader,
                session,
                options | EncoderFlag.OMIT_METATYPE,
            )
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return self.unpack_object_binary(
                ObjectKind.STRUCT,
                StructType.NODE_SPATIAL_REFERENCE,
                reader,
                session,
                options | EncoderFlag.OMIT_METATYPE,
            )
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return self.unpack_object_binary(
                    ObjectKind.STRUCT,
                    type.struct_type,
                    reader,
                    session,
                    options | EncoderFlag.OMIT_METATYPE,
                )
            else:
                return self.unpack_object_binary(
                    None,
                    None,
                    reader,
                    session,
                    options & ~EncoderFlag.OMIT_METATYPE,
                )
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot unpack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot pack UNION: {type!r}")
        #
        else:
            assert_never(type.scalar_type)


class KompaktTypeEncoder(KompaktObjectEncoder[Type]):
    """Short-circuit Type encoding to the KompaktEncoder's own methods."""

    @override
    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: Type,
        _writer: BinaryWriter,
        _options: EncoderFlag,
    ) -> None:
        _encoder.pack_type_binary(_object, _writer, _options)

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: "Session | None",
        _options: EncoderFlag,
    ) -> Type:
        return _encoder.unpack_type_binary(_reader, _options)


class KompaktValueEncoder(KompaktObjectEncoder[Value]):
    """Short-circuit Value encoding to the KompaktEncoder's own methods."""

    @override
    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: Value,
        _writer: BinaryWriter,
        _options: EncoderFlag,
    ) -> None:
        _encoder.pack_type_binary(_object.type, _writer, _options)
        _encoder.pack_value_binary(
            _object.type, _object.value, _writer, _options | EncoderFlag.OMIT_METATYPE
        )

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: "Session | None",
        _options: EncoderFlag,
    ) -> Value:
        type = _encoder.unpack_type_binary(_reader, _options)
        value = _encoder.unpack_value_binary(
            type, _reader, _session, _options | EncoderFlag.OMIT_METATYPE
        )
        return Value(type=type, value=value)
