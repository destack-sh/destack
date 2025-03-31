import abc
from typing import TYPE_CHECKING, Annotated

from bench.language import Computer, ComputerType, File, NodeMode, NodeType, Page, upload_file
from bench.pb2 import (
    ClickRequest,
    ComputerClient,
    MoveRequest,
    PressRequest,
    ScreenshotRequest,
    ScrollRequest,
    ShellCommandRequest,
    TypeRequest,
)

if TYPE_CHECKING:
    from bench.runtime import Runner


from bench.builtin.core import class_to_kit

# ruff: noqa: N802,N803


class IComputer(Runner if TYPE_CHECKING else object):
    """The basic interface to any sort of a Computer :ComputerKit."""

    def _get_service(self, Self: "Computer") -> "ComputerClient":
        """Get the ComputerService."""
        raise NotImplementedError(f"nocheckin: get {Self!r} service")

    @abc.abstractmethod
    async def Screenshot(self, Self: "Computer") -> Annotated[dict[str, File], {"Image": File}]:
        """
        Take a screenshot of the current screen
        ICON: fas fa-camera
        """
        computer_client = self._get_service(Self)
        response = await computer_client.screenshot(ScreenshotRequest())
        screenshot_timestamp = self.session._oracle.utc().strftime("%Y-%m-%d %H:%M:%S")
        screenshot = await upload_file(
            response.image, name=f"Screenshot {screenshot_timestamp}.jpeg", parent=self.tracked_run
        )
        return {"Image": screenshot}

    @abc.abstractmethod
    async def Click(self, Self: "Computer", X: int, Y: int, Button: str = "left") -> None:
        """
        Click an element
        ICON: fas fa-arrow-pointer
        """
        computer_client = self._get_service(Self)
        await computer_client.click(ClickRequest(x=X, y=Y, button=Button))

    @abc.abstractmethod
    async def Double_Click(self, Self: "Computer", X: int, Y: int) -> None:
        """
        Double click an element
        ICON: fas fa-arrow-pointer
        """
        computer_client = self._get_service(Self)
        await computer_client.double_click(ClickRequest(x=X, y=Y))

    @abc.abstractmethod
    async def Press(self, Self: "Computer", Keys: list[str]) -> None:
        """
        Press a key
        ICON: fas fa-keyboard
        """
        computer_client = self._get_service(Self)
        await computer_client.press(PressRequest(keys=Keys))

    @abc.abstractmethod
    async def Type(self, Self: "Computer", String: str) -> None:
        """
        Type a string on the keyboard
        ICON: fas fa-keyboard
        """
        computer_client = self._get_service(Self)
        await computer_client.type(TypeRequest(text=String))

    @abc.abstractmethod
    async def Move(self, Self: "Computer", X: int, Y: int) -> None:
        """
        Move the mouse to a position
        ICON: fas fa-mouse
        """
        computer_client = self._get_service(Self)
        await computer_client.move(MoveRequest(x=X, y=Y))

    @abc.abstractmethod
    async def Scroll(self, Self: "Computer", X: int, Y: int, Scroll_X: int, Scroll_Y: int) -> None:
        """
        Scroll the mouse
        ICON: fas fa-mouse
        """
        computer_client = self._get_service(Self)
        await computer_client.scroll(ScrollRequest(x=X, y=Y, scroll_x=Scroll_X, scroll_y=Scroll_Y))

    @abc.abstractmethod
    async def Shell(self, Self: "Computer", Command: str) -> None:
        """
        Execute a shell command
        ICON: fas fa-terminal
        """
        computer_client = self._get_service(Self)
        await computer_client.shell(ShellCommandRequest(command=Command))


ComputerKit = class_to_kit(IComputer, "Computer Kit", icon=NodeType.COMPUTER.icon)
UbuntuComputerTemplate = Computer(
    name="Ubuntu Computer", type=ComputerType.UBUNTU, mode=NodeMode.TEMPLATE
)
ComputerPage = Page.new("Computer", UbuntuComputerTemplate, ComputerKit)
