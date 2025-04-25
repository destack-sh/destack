from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Annotated, Any, Mapping

from bench.builtin.core import class_to_kit
from bench.language import Link, to_icon

if TYPE_CHECKING:
    pass

# ruff: noqa: N802,N803


class IWeb(ABC):
    """The Web :WebKit."""

    @abstractmethod
    async def Search(
        self, Query: str, Include_Content: bool = False, Limit: int = 5
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        """
        Search the web for the given query.
        ICON: fas fa-magnifying-glass
        """
        pass

    @abstractmethod
    async def Read(self, URL: str) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        """
        Extract the content of the given URL.
        ICON: fas fa-globe
        """
        pass


WebKit = class_to_kit(IWeb, "Web Kit", icon=to_icon("fas fa-globe"))
