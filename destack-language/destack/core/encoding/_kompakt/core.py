from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from destack import (
        BinaryDecoder,
        BinaryEncoder,
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
        _binary_encoder: "BinaryEncoder",
        _options: "EncoderFlag",
    ) -> None:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _binary_decoder: "BinaryDecoder",
        _session: "Session | None",
        _options: "EncoderFlag",
    ) -> T:
        raise NotImplementedError
