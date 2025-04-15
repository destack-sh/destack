from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Annotated

from bench.builtin.core import class_to_kit
from bench.language import Computer, ComputerType, File, NodeMode, NodeType, Page

if TYPE_CHECKING:
    pass

# ruff: noqa: N802,N803


class IComputer(ABC):
    """A Computer to control."""

    @abstractmethod
    async def Screenshot(self, Self: "Computer") -> Annotated[dict[str, File], {"Image": File}]:
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

    @abstractmethod
    async def Shell(self, Self: "Computer", Command: str) -> None:
        """
        Execute a shell command
        ICON: fas fa-terminal
        """
        pass


ComputerKit = class_to_kit(IComputer, "Computer Kit", icon=NodeType.COMPUTER.icon)
UbuntuComputerTemplate = Computer(
    name="Ubuntu Computer", type=ComputerType.UBUNTU, mode=NodeMode.TEMPLATE
)
ComputerPage = Page.new("Computer", UbuntuComputerTemplate, ComputerKit)
