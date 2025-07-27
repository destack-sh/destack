from typing import TYPE_CHECKING

from destack.language.core import BuiltinObject, EncoderOptions, Jsonc, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    from .encoder import JsoncEncoder


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class JsoncObjectEncoder:
    """JSONC object encoder."""

    def pack_object(
        self,
        _encoder: "JsoncEncoder",
        _object: BuiltinObject,
        _options: EncoderOptions,
    ) -> Jsonc:
        raise NotImplementedError

    def unpack_object(
        self,
        _encoder: "JsoncEncoder",
        _jsonc: Jsonc,
        _session: Session | None,
        _options: EncoderOptions,
    ) -> BuiltinObject:
        raise NotImplementedError
