from typing import TYPE_CHECKING, Any

from destack.language.core import EncoderOptions, Object, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    from .encoder import JsonEncoder


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class JsonObjectEncoder:
    """JSON object encoder."""

    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: Object,
        _options: EncoderOptions,
    ) -> dict[str, Any]:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: dict[str, Any],
        _session: Session | None,
        _options: EncoderOptions,
    ) -> Object:
        raise NotImplementedError
