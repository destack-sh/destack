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

from ..cson import CsonEncoder

_cson_encoder = CsonEncoder()  # for generic Values


class KompaktEncoder(Encoder[bytes]):
    """Encoder for our kompaktbuf format."""

    encoding: ClassVar[Encoding] = Encoding.KOMPAKT

    @override
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
        raise NotImplementedError

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
        raise NotImplementedError

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
        value: Any,
        type: Type,
    ) -> Any:
        raise NotImplementedError

    @override
    def pack_value_binary(
        self,
        value: Any,
        type: Type,
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
