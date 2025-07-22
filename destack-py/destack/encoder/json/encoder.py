import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BuiltinObject,
    Encoder,
    Encoding,
    Graph,
    GraphConnection,
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
        encoder = JSON_OBJECT_ENCODERS[kind, metatype]
        return encoder.pack_object(object)

    @override
    def pack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
        encoder = JSON_OBJECT_ENCODERS[kind, metatype]
        object_packed = encoder.pack_object(object)
        return json.dumps(object_packed).encode("utf-8")

    @override
    def pack_value(
        self,
        value: Any,
        type: Type,
    ) -> Json:
        return pack_json(value, type)

    @override
    def pack_value_bytes(
        self,
        value: Any,
        type: Type,
    ) -> bytes:
        return json.dumps(pack_json(value, type)).encode("utf-8")

    @override
    def unpack_value(
        self,
        type: Type,
        value: Json,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> Any:
        return unpack_json(value, type, session, graph, connection)

    @override
    def unpack_value_bytes(
        self,
        type: Type,
        value: bytes,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> Any:
        value_decoded = json.loads(value.decode("utf-8"))
        return unpack_json(value_decoded, type, session, graph, connection)

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Json,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        encoder = JSON_OBJECT_ENCODERS[kind, metatype]
        return encoder.unpack_object(value, session, graph, connection)

    @override
    def unpack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: bytes,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        encoder = JSON_OBJECT_ENCODERS[kind, metatype]
        value_decoded = json.loads(value.decode("utf-8"))
        return encoder.unpack_object(value_decoded, session, graph, connection)
