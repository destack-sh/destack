from abc import abstractmethod
from typing import TYPE_CHECKING, Any

from ..builtin import (
    EnumType,
    FlagEnum,
    Handle,
    HandleType,
    Object,
    ObjectKind,
    declare_enum,
    declare_handle,
    declare_method,
    declare_option,
)
from .binary import BinaryReader, BinaryWriter

if TYPE_CHECKING:
    from destack import Session, Type


@declare_enum(EnumType.ENCODER_OPTIONS)
class EncoderOptions(FlagEnum):
    """Options for encoding."""

    DEFAULT = declare_option(0)
    # whether to omit the metatype of the object (if possible)
    OMIT_METATYPE = declare_option(1)
    # whether to omit the key of properties (if possible)
    # OMIT_KEY = declare_option(1 << 1)
    # whether to omit the type of the value (if possible)
    # OMIT_TYPE = declare_option(1 << 2)
    # whether to omit None values (if possible)
    OMIT_NONE = declare_option(1 << 3)
    # whether to unwrap Values (if possible)
    UNWRAP_VALUE = declare_option(1 << 4)


@declare_handle(HandleType.ENCODER, is_abstract=True)
class Encoder[T: Any = Any](Handle):
    """Encoder for packing/unpacking Objects."""

    @abstractmethod
    @declare_method(100)
    def pack_object(
        self,
        object: Object,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> T:
        """Pack an Object into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(101)
    def unpack_object(
        self,
        kind: ObjectKind | None,
        type: int | None,
        value: T,
        session: "Session | None",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        """
        Unpack an Object from some encoded format.
        If type is not provided, the ObjectKind and metatype will be inferred/consumed from the value.
        """
        raise NotImplementedError

    @abstractmethod
    @declare_method(102)
    def pack_object_binary(
        self,
        object: Object,
        writer: "BinaryWriter",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        """Pack an Object into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(103)
    def unpack_object_binary(
        self,
        kind: ObjectKind | None,
        type: int | None,
        reader: "BinaryReader",
        session: "Session | None",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Object:
        """
        Unpack an Object from the byte representation of its encoded format.
        If type is not provided, the ObjectKind and metatype will be inferred/consumed from the value.
        """
        raise NotImplementedError

    @abstractmethod
    @declare_method(110)
    def pack_type(
        self,
        type: "Type",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> T:
        """Pack a Type into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(111)
    def unpack_type(
        self,
        value: T,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        """Unpack a Type from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(112)
    def pack_type_binary(
        self,
        type: "Type",
        writer: "BinaryWriter",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        """Pack a Type into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(113)
    def unpack_type_binary(
        self,
        reader: "BinaryReader",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        """Unpack a Type from the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(120)
    def pack_value(
        self,
        type: "Type",
        value: Any,
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> T:
        """Pack a value into some encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(121)
    def unpack_value(
        self,
        type: "Type",
        value: T,
        session: "Session | None",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        """Unpack a value from some encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(122)
    def pack_value_binary(
        self,
        type: "Type",
        value: Any,
        writer: "BinaryWriter",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> None:
        """Pack a value into the byte representation of its encoded format."""
        raise NotImplementedError

    @abstractmethod
    @declare_method(123)
    def unpack_value_binary(
        self,
        type: "Type",
        reader: "BinaryReader",
        session: "Session | None",
        options: EncoderOptions = EncoderOptions.DEFAULT,
    ) -> Any:
        """Unpack a value from the byte representation of its encoded format."""
        raise NotImplementedError
