from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    EnumType,
    NodeType,
    PrimitiveType,
    Struct,
    StructType,
    enum_,
    node_,
    p_kernel,
    p_regular,
    p_system,
    struct_,
)
from bench.pb2.lang_pb2 import BrowserData
from bench.utils.func import IdEnum

from .resource import DynamicResource

if TYPE_CHECKING:
    from bench.language import Client, Vector2


@struct_(StructType.DOM_NODE)
class DomNode(Struct):
    """A DOM node."""

    # :DomNode
    id: str = p_regular(30)
    tag: str | None = p_regular(31, default=None)
    xpath: str | None = p_regular(33, default=None)
    text: str | None = p_regular(34, default=None)
    attributes: dict[str, str] | None = p_regular(
        35, default=None, primitive_type=PrimitiveType.JSON
    )

    def __content_str__(self) -> str:
        content_parts: list[str] = []
        if self.tag:
            content_parts.append(self.tag)
        if self.text:
            content_parts.append(repr(self.text))
        if self.attributes:
            attribute_parts = [f"{k}={v}" for k, v in self.attributes.items()]
            content_parts.append(", ".join(attribute_parts))
        return f"{self.id} [{', '.join(content_parts)}]"


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(IdEnum):
    CHROMIUM = 1


DEFAULT_BROWSER_WIDTH = 1920.0
DEFAULT_BROWSER_HEIGHT = 1080.0


@node_(NodeType.BROWSER, has_subtypes=True)
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
