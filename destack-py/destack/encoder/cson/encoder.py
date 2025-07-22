import json
from typing import Any, ClassVar, override

from destack.language.core import (
    BuiltinObject,
    Cson,
    Encoder,
    Encoding,
    Graph,
    GraphConnection,
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
        encoder = CSON_OBJECT_ENCODERS[kind, metatype]
        return encoder.pack_object(object)

    @override
    def pack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
        encoder = CSON_OBJECT_ENCODERS[kind, metatype]
        object_packed = encoder.pack_object(object)
        return json.dumps(object_packed).encode("utf-8")

    @override
    def pack_value(
        self,
        value: Any,
        type: Type,
    ) -> Cson:
        return pack_cson(value, type)

    @override
    def pack_value_bytes(
        self,
        value: Any,
        type: Type,
    ) -> bytes:
        return pack_cson(value, type).encode("utf-8")

    @override
    def unpack_value(
        self,
        type: Type,
        value: Cson,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> Any:
        return unpack_cson(value, type, session, graph, connection)

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
        return unpack_cson(value_decoded, type, session, graph, connection)

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: Cson,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        encoder = CSON_OBJECT_ENCODERS[kind, metatype]
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
        encoder = CSON_OBJECT_ENCODERS[kind, metatype]
        value_decoded = json.loads(value.decode("utf-8"))
        return encoder.unpack_object(value_decoded, session, graph, connection)
