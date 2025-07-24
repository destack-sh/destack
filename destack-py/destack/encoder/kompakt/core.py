from typing import TYPE_CHECKING

from destack.language.core import BinaryReader, BinaryWriter, BuiltinObject, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class KompaktObjectEncoder:
    """Kompakt object encoder."""

    def pack_object(self, object: BuiltinObject, writer: BinaryWriter) -> None:
        raise NotImplementedError

    def unpack_object(self, reader: BinaryReader, session: Session | None) -> BuiltinObject:
        raise NotImplementedError
