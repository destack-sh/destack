from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Annotated, Mapping

from bench.builtin.core import class_to_service
from bench.language import Computer, ComputerType, File, NodeMode, NodeType, Page

if TYPE_CHECKING:
    pass

# ruff: noqa: N802,N803


class IComputerTerminal(ABC):
    """A Computer terminal to control."""

    @abstractmethod
    async def Shell(self, Self: "Computer", Command: str) -> None:
        """
        Execute a shell command
        ICON: fas fa-terminal
        """
        pass


class IComputerDesktop(ABC):
    """A Computer desktop to control."""

    @abstractmethod
    async def Screenshot(self, Self: "Computer") -> Annotated[Mapping[str, File], {"Image": File}]:
        """
        Take a screenshot of the current screen
        ICON: fas fa-camera
        """
        pass

    @abstractmethod
    async def Click(self, Self: "Computer", X: int, Y: int, Button: str = "left") -> None:
        """
        Click an element
        ICON: fas fa-arrow-pointer
        """
        pass

    @abstractmethod
    async def Double_Click(self, Self: "Computer", X: int, Y: int) -> None:
        """
        Double click an element
        ICON: fas fa-arrow-pointer
        """
        pass

    @abstractmethod
    async def Press(self, Self: "Computer", Keys: list[str]) -> None:
        """
        Press a key
        ICON: fas fa-keyboard
        """
        pass

    @abstractmethod
    async def Type(self, Self: "Computer", String: str) -> None:
        """
        Type a string on the keyboard
        ICON: fas fa-keyboard
        """
        pass

    @abstractmethod
    async def Move(self, Self: "Computer", X: int, Y: int) -> None:
        """
        Move the mouse to a position
        ICON: fas fa-mouse
        """
        pass

    @abstractmethod
    async def Scroll(self, Self: "Computer", X: int, Y: int, Scroll_X: int, Scroll_Y: int) -> None:
        """
        Scroll the mouse
        ICON: fas fa-mouse
        """
        pass


ComputerTerminalService = class_to_service(
    IComputerTerminal, "Computer Terminal Service", icon=NodeType.COMPUTER.icon
)
ComputerDesktopService = class_to_service(
    IComputerDesktop, "Computer Desktop Service", icon=NodeType.COMPUTER.icon
)
UbuntuComputerTemplate = Computer(
    name="Ubuntu Desktop", type=ComputerType.UBUNTU, mode=NodeMode.TEMPLATE
)
UbuntuHeadlessComputerTemplate = Computer(
    name="Ubuntu Terminal",
    type=ComputerType.UBUNTU,
    mode=NodeMode.TEMPLATE,
    is_headless=True,
)
ComputerPage = Page.new(
    "Computer", UbuntuComputerTemplate, UbuntuHeadlessComputerTemplate, ComputerTerminalService
)
