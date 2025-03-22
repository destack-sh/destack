from typing import TYPE_CHECKING, Optional, Union

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    Resource,
    enum_,
    node_,
    p_kernel,
    p_node_parent,
    p_regular,
)
from bench.pb2 import ApplicationData

if TYPE_CHECKING:
    from bench.language import Channel, Computer, Package, Page, Thread

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.APPLICATION_TYPE)
class ApplicationType(BuiltinEnum):
    # basics
    SHELL = 100, "Shell", "Computer shell", "fa-terminal"
    # browsers
    CHROME_BROWSER = 1000, "Chrome", "Chrome Browser", "fab fa-chrome"


@node_(NodeType.APPLICATION, has_subtypes=True)
class Application(Resource[ApplicationData]):
    """An Application on some Computer."""

    # meta
    parent: Union["Package", "Page", "Channel", "Thread", "Computer", None] = p_node_parent(
        4, NodeType.PACKAGE, NodeType.THREAD, NodeType.PAGE, NodeType.COMPUTER, ckless=True
    )
    type: ApplicationType = p_regular(30)

    # content
    external_name: Optional[str] = p_kernel(62, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(63, require=False, default=None, sensitive=True)

    @staticmethod
    def new(type: ApplicationType, name: str, **kwargs) -> "Application":
        return Application(type=type, name=name, **kwargs)
