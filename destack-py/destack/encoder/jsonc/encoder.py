import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    EncoderOptions,
    Encoding,
    Jsonc,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .generate import JSONC_OBJECT_ENCODERS
from .value import pack_jsonc, unpack_jsonc


class JsoncEncoder(Encoder[Jsonc]):
    """Encoder for our custom constant folded JSON format."""

    encoding: ClassVar[Encoding] = Encoding.JSONC

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        options: EncoderOptions,
    ) -> Jsonc:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.pack_object(self, object, options)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        object_packed = encoder.pack_object(self, object, options)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Jsonc,
        session: Session | None,
        options: EncoderOptions,
    ) -> BuiltinObject:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(self, value, session, options)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
    ) -> BuiltinObject:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(self, value_decoded, session, options)

    @override
    def pack_type(
        self,
        type: Type,
        options: EncoderOptions,
    ) -> Jsonc:
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
        value: Jsonc,
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

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
        options: EncoderOptions,
    ) -> Jsonc:
        return pack_jsonc(self, type, value, options)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
        options: EncoderOptions,
    ) -> None:
        writer.write_bytes(pack_jsonc(self, type, value, options).encode("utf-8"))

    @override
    def unpack_value(
        self,
        type: Type,
        value: Jsonc,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        return unpack_jsonc(self, type, value, session, options)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return unpack_jsonc(self, value_decoded, type, session, options)
