from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    EncoderOptions,
    Encoding,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .generate import KOMPAKT_OBJECT_ENCODERS


class KompaktEncoder(Encoder[bytes]):
    """Encoder for our Kompakt format."""

    encoding: ClassVar[Encoding] = Encoding.KOMPAKT

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
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
        object: BuiltinObject,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        encoder = KOMPAKT_OBJECT_ENCODERS.get((kind, metatype))
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
    ) -> BuiltinObject:
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
    ) -> BuiltinObject:
        encoder = KOMPAKT_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no KompaktObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(self, reader, session, options)

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
