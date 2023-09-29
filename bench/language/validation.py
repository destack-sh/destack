import re
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from bench.language.module import ModuleNode, NodeProperty


class ValidationError(ValueError):
    def __init__(
        self,
        subject: "ModuleNode",
        properties: list[str] | None,
        message: str,
        cause: Exception | None = None,
    ):
        super().__init__(message)
        self.subject = subject
        self.properties = properties
        self.message = message
        self.cause = cause


class ValidationHandler:
    def __call__(
        self,
        subject: "ModuleNode",
        message: str,
        properties: list[str] | None,
        cause: Exception | None = None,
    ):
        pass


class PropertyValidationHandler:
    def __init__(self, subject: "ModuleNode", prop: "NodeProperty", handler: ValidationHandler):
        self.subject = subject
        self.handler = handler
        self.prop = prop

    def __call__(
        self,
        message: str,
        cause: Exception | None = None,
    ):
        message = f"{self.prop.name}: {message}"
        self.handler(self.subject, message, [self.prop.name], cause)


def on_issue_raise(
    subject: "ModuleNode",
    message: str,
    properties: list[str] | None,
    cause: Exception | None = None,
):
    raise ValidationError(subject, properties, message, cause)


# :NameValidation
# names can be alphanumeric, hyphen, underscore, dot, spaces (but no tabs or newlines)
# leading and trailing spaces are fine

MAX_NAME_LENGTH = 256
NAME_REGEX = re.compile(r"^[a-zA-Z0-9_.\-:/ \xa0]*$")


def validate_name(value: str, on_issue: PropertyValidationHandler):
    if len(value) > MAX_NAME_LENGTH:
        on_issue(f"too long ({len(value)} > {MAX_NAME_LENGTH})")
    if not NAME_REGEX.match(value):
        on_issue(f"invalid characters ('{value}')")


MAX_TEXT_LENGTH = 2048

# not used in modules right now?
MAX_DESCRIPTION_LENGTH = 512
