from typing import TYPE_CHECKING, Optional

from bench.language.bench import DynamicResource
from bench.language.const import EnumType, NodeType, StructType, enum_
from bench.language.node import Struct, node_, struct_
from bench.language.property import p_kernel, p_regular, p_system
from bench.proto.wire.lang_pb2 import BrowserData
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Client
    from bench.language.view import Vector2


@enum_(EnumType.DOM_NODE_TYPE)
class DomNodeType(IdEnum):
    TEXT = 1
    ELEMENT = 2


@struct_(StructType.DOM_NODE)
class DomNode(Struct):
    """A DOM node."""

    # :DomNode
    type: DomNodeType = p_regular(30)
    tag: str | None = p_regular(31, default=None)
    index: int | None = p_regular(32, default=None)
    xpath: str | None = p_regular(33, default=None)
    text: str | None = p_regular(34, default=None)
    # flags
    is_interactive: bool | None = p_regular(40, default=None)
    is_visible: bool | None = p_regular(41, default=None)
    is_top: bool | None = p_regular(42, default=None)
    # children
    children: list["DomNode"] = p_regular(50, array=True, struct=StructType.DOM_NODE)


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(IdEnum):
    CHROMIUM = 1


DEFAULT_BROWSER_WIDTH = 1920.0
DEFAULT_BROWSER_HEIGHT = 1080.0


@node_(NodeType.BROWSER)
class Browser(DynamicResource[BrowserData]):
    """A Browser instance for web browsing."""

    type: BrowserType = p_regular(30, default=BrowserType.CHROMIUM)

    version: str | None = p_regular(50, default=None)
    target_version: Optional[str] = p_regular(51, default=None)
    external_name: Optional[str] = p_kernel(52, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(53, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_system(
        54, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    debugger_uri: Optional[str] = p_system(
        55, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    view_uri: Optional[str] = p_system(
        56, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        57, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    # settings
    size: "Vector2 | None" = p_regular(60, require=False, array=False, struct=StructType.VECTOR2)

    # flags
    is_headless: bool = p_system(70, default=False)
    is_insecure: bool = p_system(71, default=False)
