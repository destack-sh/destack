from typing import TYPE_CHECKING, Any

from destack.language.core import BinaryReader, BinaryWriter, Type
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    from destack.language import Session

    from .encoder import KompaktEncoder


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def pack_kompakt(
    encoder: "KompaktEncoder",
    type: Type,
    value: Any,
    writer: BinaryWriter,
) -> None:
    """Pack a generic typed value to Kompakt bytes."""
    raise NotImplementedError


def unpack_kompakt(
    encoder: "KompaktEncoder",
    type: Type,
    reader: BinaryReader,
    session: "Session | None",
) -> Any:
    """Unpack Kompakt bytes to a generic typed value."""
    raise NotImplementedError
