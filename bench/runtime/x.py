import enum
import json
from typing import Any

from bench.language import TypeNode
from bench.language.typer import check_type


def parse_string_output(value: str, type: TypeNode) -> Any:
    try:
        json_value = json.loads(value)
        check_type(json_value, type)
        return json_value
    except json.JSONDecodeError as e:
        raise ValueError(f"invalid JSON: {e}")


class GenerationErrorType(enum.Enum):
    INTERNAL = 0, "internal error"
    EXCEEDED_CONTEXT = 1, "ran out of tokens"
    INVALID_OUTPUT = 2, "invalid output"

    def __init__(self, code: int, message: str):
        self.code = code
        self.message = message


class GenerationError(ValueError):
    def __init__(
        self,
        _t: GenerationErrorType,
        message_detail: str | None = None,
        cause: Exception | None = None,
    ):
        super().__init__(_t.message)
        self.type = _t
        self.cause = cause
        self.message_detail = message_detail
