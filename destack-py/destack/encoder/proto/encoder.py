from typing import Any, ClassVar, override

from destack.language.core import (
    BuiltinObject,
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
from destack.proto import AnyObjectProto

from ..cson import CsonEncoder
from .generate import PROTO_CLASSES, PROTO_OBJECT_ENCODERS

_cson_encoder = CsonEncoder()  # for generic Values


class ProtoEncoder(Encoder[AnyObjectProto]):
    """Encoder for our protobuf format."""

    encoding: ClassVar[Encoding] = Encoding.PROTO

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> AnyObjectProto:
        encoder = PROTO_OBJECT_ENCODERS[kind, metatype]
        return encoder.pack_object(object)

    @override
    def pack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
        encoder = PROTO_OBJECT_ENCODERS[kind, metatype]
        object_packed = encoder.pack_object(object)
        return object_packed.SerializeToString()

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: AnyObjectProto,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> BuiltinObject:
        encoder = PROTO_OBJECT_ENCODERS[kind, metatype]
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
        encoder = PROTO_OBJECT_ENCODERS[kind, metatype]
        proto_cls = PROTO_CLASSES[kind, metatype]
        value_decoded = proto_cls.FromString(value)
        return encoder.unpack_object(value_decoded, session, graph, connection)

    @override
    def pack_value(
        self,
        value: Any,
        type: Type,
    ) -> Any:
        raise NotImplementedError

    @override
    def pack_value_bytes(
        self,
        value: Any,
        type: Type,
    ) -> bytes:
        raise NotImplementedError

    @override
    def unpack_value(
        self,
        type: Type,
        value: Any,
        *,
        session: Session | None,
        graph: Graph | None,
        connection: GraphConnection | None,
    ) -> Any:
        raise NotImplementedError

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
        raise NotImplementedError
