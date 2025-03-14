import abc
from typing import TYPE_CHECKING, Annotated

from bench.language import (
    Computer,
    ComputerType,
    File,
    NodeMode,
    Page,
)

if TYPE_CHECKING:
    pass


from bench.builtin.core import class_to_kit

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
    async def Type(self, String: str) -> None:
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


ComputerKit = class_to_kit(IComputer, "Computer")
RuntimeComputerTemplate = Computer(
    name="Runtime Computer", type=ComputerType.RUNTIME, mode=NodeMode.TEMPLATE
)
UbuntuComputerTemplate = Computer(
    name="Ubuntu Computer", type=ComputerType.UBUNTU, mode=NodeMode.TEMPLATE
)
ComputerPage = Page.new("Computer", RuntimeComputerTemplate, UbuntuComputerTemplate, ComputerKit)
