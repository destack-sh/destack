from typing import TYPE_CHECKING, Any, Optional

from ..builtin import (
    EncoderFlag,
    Handle,
    HandleType,
    Object,
    ObjectKind,
    UInt32,
    declare_handle,
    declare_method,
)
from .binary import BinaryReader, BinaryWriter

if TYPE_CHECKING:
    from destack import Session, Type


@declare_handle(HandleType.ENCODER, is_abstract=True)
class Encoder(Handle):
    """Encoder for packing/unpacking Objects."""

    @declare_method(100)
    def pack_object(
        self,
        object: Object,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack an Object into some encoded format."""
        raise NotImplementedError

    @declare_method(101)
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

    @declare_method(102)
    def pack_object_binary(
        self,
        object: Object,
        writer: "BinaryWriter",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack an Object into the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(103)
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: UInt32 | None,
        reader: "BinaryReader",
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Object:
        """
        Unpack an Object from the byte representation of its encoded format.
        If type is not provided, the ObjectKind and metatype will be inferred/consumed from the value.
        """
        raise NotImplementedError

    @declare_method(110)
    def pack_type(
        self,
        type: "Type",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack a Type into some encoded format."""
        raise NotImplementedError

    @declare_method(111)
    def unpack_type(
        self,
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a Type from some encoded format."""
        raise NotImplementedError

    @declare_method(112)
    def pack_type_binary(
        self,
        type: "Type",
        writer: "BinaryWriter",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack a Type into the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(113)
    def unpack_type_binary(
        self,
        reader: "BinaryReader",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a Type from the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(120)
    def pack_value(
        self,
        type: "Type",
        value: Any,
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @declare_method(121)
    def unpack_value(
        self,
        type: "Type",
        value: Any,
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @declare_method(122)
    def pack_value_binary(
        self,
        type: "Type",
        value: Any,
        writer: "BinaryWriter",
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> None:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(123)
    def unpack_value_binary(
        self,
        type: "Type",
        reader: "BinaryReader",
        session: Optional["Session"],
        options: EncoderFlag = EncoderFlag.DEFAULT,
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
