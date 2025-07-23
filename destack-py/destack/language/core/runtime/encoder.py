from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, ClassVar

from ..builtin import BuiltinObject, Encoding, NodeType, ObjectKind, StructType

if TYPE_CHECKING:
    from destack.language.core import Session, Type


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
    def pack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
    ) -> bytes:
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
    def unpack_object_bytes(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        value: bytes,
        session: "Session | None",
    ) -> BuiltinObject:
        """Unpack a BuiltinObject from the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value(
        self,
        value: Any,
        type: "Type",
    ) -> T:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value_bytes(
        self,
        value: Any,
        type: "Type",
    ) -> bytes:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value(
        self,
        type: "Type",
        value: T,
        session: "Session | None",
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value_bytes(
        self,
        type: "Type",
        value: bytes,
        session: "Session | None",
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
