from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Any, ClassVar, NamedTuple

from ..builtin import BuiltinObject, Encoding, NodeType, ObjectKind, StructType
from .binary import BinaryReader, BinaryWriter

if TYPE_CHECKING:
    from destack.language.core import Session, Type

# nocheckin: pack/unpack Struct subclasses properly
#  (in general, but specifically for Errors/Frames/Packets/...)
# nocheckin: pack partial Nodes properly (.materialization<FULL)


class EncoderOptions(NamedTuple):
    """Options for encoding."""

    """Whether to include the key of properties."""
    include_key: bool
    # include_type?
    prefer_omit_none: bool
    unwrap_value: bool


class Encoder[T: Any = Any](ABC):
    """Encoder for packing/unpacking BuiltinObjects."""

    encoding: ClassVar[Encoding]

    TAGGED: ClassVar[EncoderOptions] = EncoderOptions(
        include_key=True, prefer_omit_none=True, unwrap_value=False
    )
    UNTAGGED: ClassVar[EncoderOptions] = EncoderOptions(
        include_key=False, prefer_omit_none=True, unwrap_value=False
    )

    @abstractmethod
    def pack_object(
        self,
        kind: ObjectKind,
        metatype: NodeType | StructType,
        object: BuiltinObject,
        options: EncoderOptions,
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
        options: EncoderOptions,
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
        options: EncoderOptions,
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
        options: EncoderOptions,
    ) -> BuiltinObject:
        """Unpack a BuiltinObject from the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_type(
        self,
        type: "Type",
        options: EncoderOptions,
    ) -> T:
        """Pack a Type into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_type_binary(
        self,
        type: "Type",
        writer: "BinaryWriter",
        options: EncoderOptions,
    ) -> None:
        """Pack a Type into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_type(
        self,
        value: T,
        options: EncoderOptions,
    ) -> Any:
        """Unpack a Type from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_type_binary(
        self,
        reader: "BinaryReader",
        options: EncoderOptions,
    ) -> Any:
        """Unpack a Type from the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value(
        self,
        type: "Type",
        value: Any,
        options: EncoderOptions,
    ) -> T:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def pack_value_binary(
        self,
        type: "Type",
        value: Any,
        writer: "BinaryWriter",
        options: EncoderOptions,
    ) -> None:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value(
        self,
        type: "Type",
        value: T,
        session: "Session | None",
        options: EncoderOptions,
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    def unpack_value_binary(
        self,
        type: "Type",
        reader: "BinaryReader",
        session: "Session | None",
        options: EncoderOptions,
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
