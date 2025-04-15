from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Annotated, Any, Mapping

from bench.builtin.core import class_to_kit
from bench.language import to_icon

if TYPE_CHECKING:
    pass

# ruff: noqa: N802,N803


class IWeb(ABC):
    """The Web :WebKit."""

    # nocheckin: proper web search - Links? Citations? raw JSON? what do we return..?

    @abstractmethod
    async def Search(
        self, Query: str, Results: int = 10
    ) -> Annotated[Mapping[str, Any], {"Result": str}]:
        """
        Search the web for the given query.
        ICON: fas fa-globe
        """
        pass

    @abstractmethod
    async def Extract(self, URL: str) -> str:
        """
        Extract the text content of the given URL.
        ICON: fas fa-globe-pointer
        """
        pass

    @abstractmethod
    async def Extract_Many(self, URLs: list[str]) -> list[str]:
        """
        Extract the text content of the given URLs.
        ICON: fas fa-globe-pointer
        """
        pass


WebKit = class_to_kit(IWeb, "Web Kit", icon=to_icon("fas fa-globe"))
