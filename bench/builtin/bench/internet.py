from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Annotated, Any, Mapping

from bench.builtin.core import class_to_kit
from bench.language import File, Link, Page, to_icon

if TYPE_CHECKING:
    pass

# ruff: noqa: N802,N803


class IInternet(ABC):
    """The Internet :InternetKit."""

    @abstractmethod
    async def Search(
        self, Query: str, Include_Content: bool = False, Limit: int = 5
    ) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        """
        Search the internet for the given query.
        ICON: fas fa-magnifying-glass
        """
        pass

    @abstractmethod
    async def Search_Images(
        self, Query: str, Limit: int = 3
    ) -> Annotated[Mapping[str, Any], {"Images": list[File]}]:
        """
        Search the internet for images matching the given query.
        Currently, only Unsplash is supported, so only generic (non-recent) queries work.
        ICON: fas fa-image
        """

        pass

    @abstractmethod
    async def Read(self, URL: str) -> Annotated[Mapping[str, Any], {"Links": list[Link]}]:
        """
        Extract the content of the given URL.
        ICON: fas fa-globe
        """
        pass


InternetKit = class_to_kit(IInternet, "Internet Kit", icon=to_icon("fas fa-globe"))
InternetPage = Page.new("Internet")
InternetPage.append(InternetKit)
