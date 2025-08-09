from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from destack import (
        BinaryReader,
        BinaryWriter,
        EncoderFlag,
        Object,
        Session,
    )

    from .encoder import KompaktEncoder


type_ = type


class KompaktObjectEncoder[T: Object = Object]:
    """Kompakt object encoder."""

    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: T,
        _writer: "BinaryWriter",
        _options: "EncoderFlag",
    ) -> None:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: "BinaryReader",
        _session: "Session | None",
        _options: "EncoderFlag",
    ) -> T:
        raise NotImplementedError
