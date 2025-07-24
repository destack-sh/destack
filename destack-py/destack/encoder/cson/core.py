from typing import TYPE_CHECKING

from destack.language.core import BuiltinObject, Cson, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class CsonObjectEncoder:
    """CSON object encoder."""

    def pack_object(self, object: BuiltinObject) -> Cson:
        raise NotImplementedError

    def unpack_object(
        self,
        cson: Cson,
        session: Session | None,
    ) -> BuiltinObject:
        raise NotImplementedError
