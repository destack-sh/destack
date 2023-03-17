import enum


class GenerationErrorType(enum.Enum):
    INTERNAL = 0, "internal error"
    EXCEEDED_CONTEXT = 1, "ran out of tokens"

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
