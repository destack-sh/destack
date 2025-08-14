from typing import TYPE_CHECKING, Any

from ...builtin import Object

if TYPE_CHECKING:
    from .encoder import EncoderFlag, JsonEncoder, Session


type_ = type


class JsonObjectEncoder[T: Object = Object]:
    """JSON object encoder."""

    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: T,
        _options: "EncoderFlag",
    ) -> dict[str, Any]:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: dict[str, Any],
        _session: "Session | None",
        _options: "EncoderFlag",
    ) -> T:
        raise NotImplementedError
