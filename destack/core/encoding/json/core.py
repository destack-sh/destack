from typing import TYPE_CHECKING, Any

from destack.core import EncoderOptions, Object, Session

if TYPE_CHECKING:
    from .encoder import JsonEncoder


type_ = type


class JsonObjectEncoder[T: Object = Object]:
    """JSON object encoder."""

    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: T,
        _options: EncoderOptions,
    ) -> dict[str, Any]:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: dict[str, Any],
        _session: Session | None,
        _options: EncoderOptions,
    ) -> T:
        raise NotImplementedError
