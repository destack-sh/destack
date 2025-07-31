from collections.abc import Mapping
from typing import Any, ClassVar, assert_never, cast, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    Encoder,
    EncoderOptions,
    Encoding,
    EnumType,
    NodeReference,
    NodeType,
    Object,
    ObjectKind,
    PrimitiveType,
    ScalarType,
    Session,
    StructType,
    Type,
    TypeCardinality,
    Value,
)
from destack.language.registry import ENUM_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.utils.uuid import UUID

from .core import KompaktObjectEncoder

_UUID_NULL = UUID(int=0)


class KompaktEncoder(Encoder[bytes]):
    """Encoder for our Kompakt format."""

    encoding: ClassVar[Encoding] = Encoding.KOMPAKT

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
        encoders[ObjectKind.STRUCT, StructType.NODE_REFERENCE] = cast(
            KompaktObjectEncoder, KompaktNodeReferenceEncoder()
        )
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def pack_object(
        self,
        object: Object,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> bytes:
        writer = BinaryWriter()
        object_bytes = writer.to_bytes()
        return object_bytes

    @override
    def unpack_object(
        self,
        type: tuple[ObjectKind, int] | None,
        value: bytes,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        reader = BinaryReader(value)
        object = self.unpack_object_binary(type, reader, session, options)
        return object

    @override
    def pack_object_binary(
        self,
        object: Object,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        # metatype
        if not options & EncoderOptions.OMIT_METATYPE:
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
        type: tuple[ObjectKind, int] | None,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        # metatype
        if type is None:
            metakind = ObjectKind(reader.read_uint8())
            metatype = reader.read_uint32()
            type = (metakind, metatype)
        # object
        encoder = self.encoders.get(type)
        assert encoder is not None, f"no KompaktObjectEncoder for {type!r}"
        return encoder.unpack_object(self, reader, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> bytes:
        writer = BinaryWriter()
        self.pack_type_binary(type, writer, options)
        type_bytes = writer.to_bytes()
        return type_bytes

    @override
    def unpack_type(
        self,
        value: bytes,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Type:
        reader = BinaryReader(value)
        type = self.unpack_type_binary(reader, options)
        return type

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        # preamble (3 bits cardinality, 3 bits scalar type, 1 bit is_required)
        writer.write_uint8(
            type.cardinality | ((type.scalar_type or 0) << 3) | (type.is_required << 6)
        )
        # scalar (type folded into preamble)
        if type.cardinality == TypeCardinality.SCALAR:
            assert type.scalar_type is not None, f"no scalar type for {type!r}"
            if type.scalar_type == ScalarType.PRIMITIVE:
                assert type.primitive_type is not None, f"no primitive type for {type!r}"
                writer.write_uint8(type.primitive_type)
            elif type.scalar_type == ScalarType.ENUM:
                assert type.enum_type is not None, f"no enum type for {type!r}"
                writer.write_uint32(type.enum_type)
            elif type.scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
                assert type.node_types is not None, f"no node types for {type!r}"
                writer.write_uint32(len(type.node_types))
                for node_type in type.node_types:
                    writer.write_uint32(node_type)
            elif type.scalar_type == ScalarType.STRUCT:
                assert type.struct_type is not None, f"no struct type for {type!r}"
                writer.write_uint32(type.struct_type)
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
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Type:
        # preamble (3 bits cardinality, 3 bits scalar type, 1 bit is_required)
        preamble = reader.read_uint8()
        cardinality = TypeCardinality(preamble & 0b111)
        scalar_type = ScalarType((preamble >> 3) & 0b111) if (preamble >> 3) & 0b111 else None
        is_required = bool((preamble >> 6) & 0b1)
        key_type = None
        value_type = None
        element_types = None
        node_types = None
        struct_type = None
        primitive_type = None
        enum_type = None
        # scalar (type folded into preamble)
        if cardinality == TypeCardinality.SCALAR:
            assert scalar_type is not None, f"no scalar type for {cardinality}"
            if scalar_type == ScalarType.PRIMITIVE:
                primitive_type = PrimitiveType(reader.read_uint8())
            elif scalar_type == ScalarType.ENUM:
                enum_type = EnumType(reader.read_uint32())
            elif scalar_type in (ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
                node_types = [NodeType(reader.read_uint32()) for _ in range(reader.read_uint32())]
            elif scalar_type == ScalarType.STRUCT:
                struct_type = StructType(reader.read_uint32())
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
            is_required=is_required,
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
        options: EncoderOptions = EncoderOptions.DEFAULT,
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
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        reader = BinaryReader(value)
        return self.unpack_value_binary(type, reader, session, options)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            self.pack_scalar_value_binary(
                type, value, writer, options | EncoderOptions.OMIT_METATYPE
            )
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            writer.write_uint32(len(value))
            for item in value:
                self.pack_value_binary(
                    type.value_type, item, writer, options | EncoderOptions.OMIT_METATYPE
                )
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            writer.write_uint32(len(value))
            for item, element_type in zip(value, type.element_types):
                self.pack_value_binary(
                    element_type, item, writer, options | EncoderOptions.OMIT_METATYPE
                )
        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            writer.write_uint32(len(value))
            for key, val in value.items():
                self.pack_value_binary(
                    type.key_type, key, writer, options | EncoderOptions.OMIT_METATYPE
                )
                self.pack_value_binary(
                    type.value_type, val, writer, options | EncoderOptions.OMIT_METATYPE
                )
        #
        else:
            assert_never(type.cardinality)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            return self.unpack_scalar_value_binary(
                type, reader, session, options | EncoderOptions.OMIT_METATYPE
            )
        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            length = reader.read_uint32()
            return [
                self.unpack_value_binary(
                    type.value_type, reader, session, options | EncoderOptions.OMIT_METATYPE
                )
                for _ in range(length)
            ]
        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            length = reader.read_uint32()
            return tuple(
                self.unpack_value_binary(
                    element_type, reader, session, options | EncoderOptions.OMIT_METATYPE
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
                    type.key_type, reader, session, options | EncoderOptions.OMIT_METATYPE
                ): self.unpack_value_binary(
                    type.value_type, reader, session, options | EncoderOptions.OMIT_METATYPE
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
        options: EncoderOptions,
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
            elif type.primitive_type == PrimitiveType.UUID:
                writer.write_uuid(value)
            elif type.primitive_type == PrimitiveType.BYTES:
                writer.write_bytes(value)
            elif type.primitive_type == PrimitiveType.STRING:
                writer.write_string(value)
            elif type.primitive_type == PrimitiveType.JSON:
                writer.write_json(value)
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            writer.write_uint32(value)
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            self.pack_object_binary(value, writer, options | EncoderOptions.OMIT_METATYPE)
        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            self.pack_object_binary(value, writer, options & ~EncoderOptions.OMIT_METATYPE)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                self.pack_object_binary(value, writer, options | EncoderOptions.OMIT_METATYPE)
            else:
                self.pack_object_binary(value, writer, options & ~EncoderOptions.OMIT_METATYPE)
        #
        else:
            assert_never(type.scalar_type)

    def unpack_scalar_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
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
            elif type.primitive_type == PrimitiveType.UUID:
                return reader.read_uuid()
            elif type.primitive_type == PrimitiveType.BYTES:
                return reader.read_bytes()
            elif type.primitive_type == PrimitiveType.STRING:
                return reader.read_string()
            elif type.primitive_type == PrimitiveType.JSON:
                return reader.read_json()
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            enum_value = reader.read_uint32()
            return enum_cls(enum_value)
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return self.unpack_object_binary(
                    (ObjectKind.STRUCT, type.struct_type),
                    reader,
                    session,
                    options | EncoderOptions.OMIT_METATYPE,
                )
            else:
                return self.unpack_object_binary(
                    None,
                    reader,
                    session,
                    options & ~EncoderOptions.OMIT_METATYPE,
                )
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return self.unpack_object_binary(
                (ObjectKind.STRUCT, StructType.NODE_REFERENCE),
                reader,
                session,
                options | EncoderOptions.OMIT_METATYPE,
            )
        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return self.unpack_object_binary(
                None,
                reader,
                session,
                options | EncoderOptions.OMIT_METATYPE,
            )
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
        _options: EncoderOptions,
    ) -> None:
        _encoder.pack_type_binary(_object, _writer, _options)

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: Session | None,
        _options: EncoderOptions,
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
        _options: EncoderOptions,
    ) -> None:
        _encoder.pack_type_binary(_object.type, _writer, _options)
        _encoder.pack_value_binary(
            _object.type, _object.value, _writer, _options | EncoderOptions.OMIT_METATYPE
        )

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: Session | None,
        _options: EncoderOptions,
    ) -> Value:
        type = _encoder.unpack_type_binary(_reader, _options)
        value = _encoder.unpack_value_binary(
            type, _reader, _session, _options | EncoderOptions.OMIT_METATYPE
        )
        return Value(type=type, value=value)


class KompaktNodeReferenceEncoder(KompaktObjectEncoder[NodeReference]):
    """More compact NodeReference Kompakt encoding (because it's so heavily used)."""

    @override
    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: NodeReference,
        _writer: BinaryWriter,
        _options: EncoderOptions,
    ) -> None:
        _writer.write_uint32(_object.type)
        _writer.write_uuid(_object.id)
        _writer.write_uuid(_object.space_id)
        _writer.write_uuid(
            _object.definition_id if _object.definition_id is not None else _UUID_NULL
        )
        _writer.write_uuid(_object.branch_id)
        _writer.write_uuid(_object.snapshot_id)

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: Session | None,
        _options: EncoderOptions,
    ) -> NodeReference:
        type = NodeType(_reader.read_uint32())
        id = _reader.read_uuid()
        space_id = _reader.read_uuid()
        definition_id = _reader.read_uuid()
        if definition_id == _UUID_NULL:
            definition_id = None
        branch_id = _reader.read_uuid()
        snapshot_id = _reader.read_uuid()
        return NodeReference(
            type=type,
            id=id,
            space_id=space_id,
            definition_id=definition_id,
            branch_id=branch_id,
            snapshot_id=snapshot_id,
        )
