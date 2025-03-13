import abc
from typing import TYPE_CHECKING, Annotated, override

from bench.language import Browser, File, FileFormat, FileType, NodeMode, Page, upload_file

if TYPE_CHECKING:
    from playwright.async_api import Page as PlaywrightPage

    from bench.runtime.core.runner import Runner


from .reflect import class_to_kit

# ruff: noqa: N802,N803


class IComputer(abc.ABC):
    """The basic interface to any sort of a Computer."""

    @abc.abstractmethod
    async def Screenshot(self) -> Annotated[dict[str, File], {"image": File}]:
        """
        Take a screenshot of the current screen
        ICON: fas fa-camera
        """
        ...

    @abc.abstractmethod
    async def Click(self, X: int, Y: int, Button: str = "left") -> None:
        """
        Click an element
        ICON: fas fa-arrow-pointer
        """
        ...

    @abc.abstractmethod
    async def Double_Click(self, X: int, Y: int) -> None:
        """
        Double click an element
        ICON: fas fa-arrow-pointer
        """
        ...

    @abc.abstractmethod
    async def Press(self, Keys: list[str]) -> None:
        """
        Press a key
        ICON: fas fa-keyboard
        """
        ...

    @abc.abstractmethod
    async def Type(self, Text: str) -> None:
        """
        Type a string on the keyboard
        ICON: fas fa-keyboard
        """
        ...

    @abc.abstractmethod
    async def Move(self, X: int, Y: int) -> None:
        """
        Move the mouse to a position
        ICON: fas fa-mouse
        """
        ...

    @abc.abstractmethod
    async def Scroll(self, X: int, Y: int, Scroll_X: int, Scroll_Y: int) -> None:
        """
        Scroll the mouse
        ICON: fas fa-mouse
        """
        ...


class IBrowser(IComputer, Runner if TYPE_CHECKING else None):
    """The common interface for a Browser."""

    def _get_pw_page(self) -> PlaywrightPage: ...

    #
    # Computer
    #

    @override
    async def Screenshot(self) -> Annotated[dict[str, File], {"image": File}]:
        browser = self._get_resource(Browser)
        pw_page = self._get_pw_page()
        screenshot_bytes = await pw_page.screenshot(full_page=False, animations="disabled")
        now = self.session._oracle.utc()
        screenshot = await upload_file(
            screenshot_bytes,
            type=FileType.IMAGE,
            format=FileFormat.PNG,
            name=f"{browser.name} Screenshot {now.strftime('%Y-%m-%d %H:%M:%S.%f')}",
        )
        return {"image": screenshot}

    @override
    async def Double_Click(self, X: int, Y: int) -> None:
        pw_page = self._get_pw_page()
        await pw_page.mouse.dblclick(X, Y)

    #
    # Browser-specific
    #

    async def Get_Current_Url(self) -> str:
        """Get the current URL of the browser."""
        ...

    async def Go_To_Url(self, Url: str) -> None:
        """Go to a URL."""
        ...

    async def Go_To_Tab(self, Tab_Index: int) -> None:
        """Go to a tab."""
        ...

    async def Go_Forward(self) -> None:
        """Go forward in the browser history."""
        ...

    async def Go_Back(self) -> None:
        """Go back in the browser history."""
        ...


ComputerPage = Page.new("Computer")
ComputerKit = class_to_kit(IComputer, "Computer", mode=NodeMode.TEMPLATE)
BrowserKit = class_to_kit(IBrowser, "Browser", template=ComputerKit)
ComputerPage.extend(ComputerKit, BrowserKit)
