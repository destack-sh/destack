import abc
from abc import ABC
from typing import TYPE_CHECKING, Any, Optional

from ..builtin import (
    EncoderFlag,
    Object,
    ObjectKind,
    UInt32,
)
from .binary import BinaryDecoder, BinaryEncoder

if TYPE_CHECKING:
    from destack import Session, Type


class Encoder(ABC):
    """Encoder for packing/unpacking Objects."""

    @abc.abstractmethod
    def pack_object(
        self,
        object: Object,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack an Object into some encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_object(
        self,
        kind: ObjectKind | None,
        type: UInt32 | None,
        value: Any,
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        """
        Unpack an Object from some encoded format.
        If type is not provided, the ObjectKind and metatype will be inferred/consumed from the value.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def pack_object_binary(
        self,
        object: Object,
        encoder: "BinaryEncoder",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack an Object into the byte representation of its encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: UInt32 | None,
        decoder: "BinaryDecoder",
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        """
        Unpack an Object from the byte representation of its encoded format.
        If type is not provided, the ObjectKind and metatype will be inferred/consumed from the value.
        """
        raise NotImplementedError

    @abc.abstractmethod
    def pack_type(
        self,
        type: "Type",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack a Type into some encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_type(
        self,
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a Type from some encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def pack_type_binary(
        self,
        type: "Type",
        encoder: "BinaryEncoder",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack a Type into the byte representation of its encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_type_binary(
        self,
        decoder: "BinaryDecoder",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a Type from the byte representation of its encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def pack_value(
        self,
        type: "Type",
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_value(
        self,
        type: "Type",
        value: Any,
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def pack_value_binary(
        self,
        type: "Type",
        value: Any,
        encoder: "BinaryEncoder",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @abc.abstractmethod
    def unpack_value_binary(
        self,
        type: "Type",
        decoder: "BinaryDecoder",
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
