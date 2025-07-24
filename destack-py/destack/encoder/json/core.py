from typing import TYPE_CHECKING, Any

from destack.language.core import BuiltinObject, Session
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass


# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


class JsonObjectEncoder:
    """JSON object encoder."""

    def pack_object(self, object: BuiltinObject) -> dict[str, Any]:
        raise NotImplementedError

    def unpack_object(
        self,
        json_obj: dict[str, Any],
        session: Session | None,
    ) -> BuiltinObject:
        raise NotImplementedError
