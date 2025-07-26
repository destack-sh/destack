from typing import Any, ClassVar, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    BuiltinObject,
    Encoder,
    Encoding,
    NodeType,
    ObjectKind,
    Session,
    StructType,
    Type,
)


class KompaktEncoder(Encoder[bytes]):
    """Encoder for our Kompakt format."""

    encoding: ClassVar[Encoding] = Encoding.KOMPAKT

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
        writer = BinaryWriter()
        self.pack_object_binary(kind, metatype, object, writer)
        object_bytes = writer.to_bytes()
        return object_bytes

    @override
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: BinaryWriter,
    ) -> None:
        raise NotImplementedError

    @override
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: bytes,
        session: Session | None,
    ) -> BuiltinObject:
        reader = BinaryReader(value)
        object = self.unpack_object_binary(kind, metatype, reader, session)
        return object

    @override
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: BinaryReader,
        session: Session | None,
    ) -> BuiltinObject:
        raise NotImplementedError

    @override
    def pack_value(
        self,
        type: Type,
        value: Any,
    ) -> Any:
        raise NotImplementedError

    @override
    def pack_value_binary(
        self,
        type: Type,
        value: Any,
        writer: BinaryWriter,
    ) -> None:
        raise NotImplementedError

    @override
    def unpack_value(
        self,
        type: Type,
        value: Any,
        session: Session | None,
    ) -> Any:
        raise NotImplementedError

    @override
    def unpack_value_binary(
        self,
        type: Type,
        reader: BinaryReader,
        session: Session | None,
    ) -> Any:
        raise NotImplementedError
