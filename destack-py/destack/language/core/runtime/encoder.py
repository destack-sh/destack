from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, ClassVar

from ..builtin import BuiltinObject, Encoding, NodeType, ObjectKind, StructType
from .binary import BinaryReader, BinaryWriter

if TYPE_CHECKING:
    from destack.language.core import BasicType, Session


class Encoder[T: Any = Any](ABC):
    """Encoder for packing/unpacking BuiltinObjects."""

    encoding: ClassVar[Encoding]

    @abstractmethod
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> T:
        """Pack a BuiltinObject into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        writer: "BinaryWriter",
    ) -> None:
        """Pack a BuiltinObject into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: T,
        session: "Session | None",
    ) -> BuiltinObject:
        """Unpack a BuiltinObject from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_object_binary(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        reader: "BinaryReader",
        session: "Session | None",
    ) -> BuiltinObject:
        """Unpack a BuiltinObject from the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value(
        self,
        type: "BasicType",
        value: Any,
    ) -> T:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value_binary(
        self,
        type: "BasicType",
        value: Any,
        writer: "BinaryWriter",
    ) -> None:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value(
        self,
        type: "BasicType",
        value: T,
        session: "Session | None",
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value_binary(
        self,
        type: "BasicType",
        reader: "BinaryReader",
        session: "Session | None",
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
