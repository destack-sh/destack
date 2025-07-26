import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    Encoding,
    Jsonc,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .generate import JSONC_OBJECT_ENCODERS
from .wiring import pack_jsonc, unpack_jsonc


class JsoncEncoder(Encoder[Jsonc]):
    """Encoder for our custom constant folded JSON format."""

    encoding: ClassVar[Encoding] = Encoding.JSONC

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> Jsonc:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.pack_object(object)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: BinaryWriter,
    ) -> None:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        object_packed = encoder.pack_object(object)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Jsonc,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(value, session)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = JSONC_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsoncObjectEncoder for {kind.name}:{metatype.name}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(value_decoded, session)

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
    ) -> Jsonc:
        return pack_jsonc(value, type)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
    ) -> None:
        writer.write_bytes(pack_jsonc(value, type).encode("utf-8"))

    @override
    def unpack_value(
        self,
        type: Type,
        value: Jsonc,
        session: Session | None,
    ) -> Any:
        return unpack_jsonc(value, type, session)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return unpack_jsonc(value_decoded, type, session)
