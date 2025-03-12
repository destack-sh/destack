import abc
from typing import Annotated

from bench.builtin.reflect import class_to_kit
from bench.language import File, NodeMode
from bench.language.source.page import Page


class IComputer(abc.ABC):
    """The basic interface to any sort of Computer Resource."""

    @abc.abstractmethod
    def screenshot(self) -> Annotated[dict[str, File], {"image": File}]:
        """
        Take a screenshot of the current screen
        ICON: fas fa-camera
        """
        ...

    @abc.abstractmethod
    def click(self, x: int, y: int, button: str = "left") -> None:
        """
        Click an element
        ICON: fas fa-arrow-pointer
        """
        ...

    @abc.abstractmethod
    def double_click(self, x: int, y: int) -> None:
        """
        Double click an element
        ICON: fas fa-arrow-pointer
        """
        ...

    @abc.abstractmethod
    def press(self, keys: list[str]) -> None:
        """
        Press a key
        ICON: fas fa-keyboard
        """
        ...

    @abc.abstractmethod
    def type(self, text: str) -> None:
        """
        Type a string on the keyboard
        ICON: fas fa-keyboard
        """
        ...

    @abc.abstractmethod
    def move(self, x: int, y: int) -> None:
        """
        Move the mouse to a position
        ICON: fas fa-mouse
        """
        ...

    @abc.abstractmethod
    def scroll(self, x: int, y: int, scroll_x: int, scroll_y: int) -> None:
        """
        Scroll the mouse
        ICON: fas fa-mouse
        """
        ...


class IBrowser(IComputer): ...


ComputerPage = Page.new("Computer")
ComputerKit = class_to_kit(IComputer, "Computer", mode=NodeMode.TEMPLATE)
BrowserKit = class_to_kit(IBrowser, "Browser")
ComputerPage.extend(ComputerKit, BrowserKit)
