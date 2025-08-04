from typing import TYPE_CHECKING

from destack.core import BinaryReader, BinaryWriter, EncoderOptions, Object, Session

if TYPE_CHECKING:
    from .encoder import KompaktEncoder


type_ = type


class KompaktObjectEncoder[T: Object = Object]:
    """Kompakt object encoder."""

    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: T,
        _writer: BinaryWriter,
        _options: EncoderOptions,
    ) -> None:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: Session | None,
        _options: EncoderOptions,
    ) -> T:
        raise NotImplementedError
