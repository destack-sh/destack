import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Cson,
    Encoder,
    Encoding,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .generate import CSON_OBJECT_ENCODERS
from .wiring import pack_cson, unpack_cson


class CsonEncoder(Encoder[Cson]):
    """Encoder for our custom constant folded JSON format."""

    encoding: ClassVar[Encoding] = Encoding.CSON

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> Cson:
        encoder = CSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no CsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.pack_object(object)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: BinaryWriter,
    ) -> None:
        encoder = CSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no CsonObjectEncoder for {kind.name}:{metatype.name}"
        object_packed = encoder.pack_object(object)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Cson,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = CSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no CsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(value, session)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = CSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no CsonObjectEncoder for {kind.name}:{metatype.name}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(value_decoded, session)

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
    ) -> Cson:
        return pack_cson(value, type)

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
    ) -> None:
        writer.write_bytes(pack_cson(value, type).encode("utf-8"))

    @override
    def unpack_value(
        self,
        type: Type,
        value: Cson,
        session: Session | None,
    ) -> Any:
        return unpack_cson(value, type, session)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return unpack_cson(value_decoded, type, session)
