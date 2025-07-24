import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    Encoding,
    Json,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)

from .generate import JSON_OBJECT_ENCODERS
from .wiring import pack_json, unpack_json


class JsonEncoder(Encoder[Json]):
    """Encoder for standard JSON format with proper names."""

    encoding: ClassVar[Encoding] = Encoding.JSON

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> dict[str, Any]:
        encoder = JSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.pack_object(object)

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: BinaryWriter,
    ) -> None:
        encoder = JSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        object_packed = encoder.pack_object(object)
        writer.write_bytes(json.dumps(object_packed).encode("utf-8"))

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Json,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = JSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        return encoder.unpack_object(value, session)

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
    ) -> BuiltinObject:
        encoder = JSON_OBJECT_ENCODERS.get((kind, metatype))
        assert encoder is not None, f"no JsonObjectEncoder for {kind.name}:{metatype.name}"
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return encoder.unpack_object(value_decoded, session)

    @override
    def pack_value(
        self,
        value: Any,
        type: Type,
    ) -> Json:
        return pack_json(value, type)

    @override
    def pack_value_binary(
        self,
        value: Any,
        type: Type,
        writer: BinaryWriter,
    ) -> None:
        writer.write_bytes(json.dumps(pack_json(value, type)).encode("utf-8"))

    @override
    def unpack_value(
        self,
        type: Type,
        value: Json,
        session: Session | None,
    ) -> Any:
        return unpack_json(value, type, session)

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
    ) -> Any:
        value_decoded = json.loads(reader.read_bytes().decode("utf-8"))
        return unpack_json(value_decoded, type, session)
