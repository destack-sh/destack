from abc import ABC
from typing import TYPE_CHECKING

from bench.builtin.core import class_to_kit
from bench.language import to_icon

if TYPE_CHECKING:
    pass


class IWeb(ABC):
    """The Web :WebKit."""

    pass


WebKit = class_to_kit(IWeb, "Web Kit", icon=to_icon("fas fa-globe"))
