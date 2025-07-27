from typing import TYPE_CHECKING

from destack.language.core import BinaryReader, BinaryWriter, BuiltinObject, EncoderOptions, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    from .encoder import KompaktEncoder


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class KompaktObjectEncoder:
    """Kompakt object encoder."""

    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: BuiltinObject,
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
    ) -> BuiltinObject:
        raise NotImplementedError
