from typing import Any, ClassVar, cast, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    Encoder,
    EncoderOptions,
    Encoding,
    NodeType,
    Object,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .core import KompaktObjectEncoder


class KompaktEncoder(Encoder[bytes]):
    """Encoder for our Kompakt format."""

    encoding: ClassVar[Encoding] = Encoding.KOMPAKT

    def __init__(
        self, encoders: dict[tuple[ObjectKind, NodeType | StructType], KompaktObjectEncoder]
    ):
        self.encoders = encoders

    @classmethod
    def generate(cls) -> "KompaktEncoder":
        from .generate import KompaktEncoderGenerator

        generator = KompaktEncoderGenerator()
        encoders: dict[tuple[ObjectKind, NodeType | StructType], KompaktObjectEncoder] = {
            **generator.generate()
        }
        encoders[ObjectKind.STRUCT, StructType.TYPE] = cast(
            KompaktObjectEncoder, KompaktTypeEncoder()
        )
        return cls(encoders)

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: Object,
        options: EncoderOptions,
    ) -> bytes:
        writer = BinaryWriter()
        self.pack_object_binary(kind, metatype, object, writer, options)
        object_bytes = writer.to_bytes()
        return object_bytes

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
        assert encoder is not None, f"no KompaktObjectEncoder for {kind.name}:{metatype.name}"
        encoder.pack_object(self, object, writer, options)

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: bytes,
        session: Session | None,
        options: EncoderOptions,
    ) -> Object:
        reader = BinaryReader(value)
        object = self.unpack_object_binary(kind, metatype, reader, session, options)
        return object

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
        assert encoder is not None, f"no KompaktObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(self, reader, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions,
    ) -> bytes:
        writer = BinaryWriter()
        self.pack_type_binary(type, writer, options)
        type_bytes = writer.to_bytes()
        return type_bytes

    @override
    def pack_type_binary(
        self,
        type: Type,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        raise NotImplementedError

    @override
    def unpack_type(
        self,
        value: bytes,
        options: EncoderOptions,
    ) -> Type:
        reader = BinaryReader(value)
        type = self.unpack_type_binary(reader, options)
        return type

    @override
    def unpack_type_binary(
        self,
        reader: BinaryReader,
        options: EncoderOptions,
    ) -> Type:
        raise NotImplementedError

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
        options: EncoderOptions,
    ) -> Any:
        raise NotImplementedError

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        raise NotImplementedError

    @override
    def unpack_value(
        self,
        type: Type,
        value: Any,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        raise NotImplementedError

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        raise NotImplementedError


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
        return _encoder.pack_type_binary(_object, _writer, _options)

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: Session | None,
        _options: EncoderOptions,
    ) -> Type:
        return _encoder.unpack_type_binary(_reader, _options)
