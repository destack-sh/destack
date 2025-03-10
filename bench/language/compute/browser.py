from typing import TYPE_CHECKING, Optional

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    NodeType,
    PrimitiveType,
    Resource,
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

if TYPE_CHECKING:
    from bench.language import Client, Vector2


@struct_(StructType.DOM_NODE)
class DomNode(Struct):
    """A DOM node."""

    # :DomNode
    id: str = p_regular(30)
    tag: str | None = p_regular(31, default=None)
    text: str | None = p_regular(34, default=None)
    attributes: dict[str, str] | None = p_regular(
        35, default=None, primitive_type=PrimitiveType.JSON
    )
    is_focused: bool | None = p_regular(36, default=None)

    def __content_str__(self) -> str:
        content_parts: list[str] = []
        if self.tag:
            content_parts.append(self.tag)
        if self.text:
            content_parts.append(repr(self.text))
        if self.is_focused:
            content_parts.append("focused")
        if self.attributes:
            attribute_parts = [f"{k}={v}" for k, v in self.attributes.items()]
            content_parts.append(", ".join(attribute_parts))
        return f"{self.id} [{', '.join(content_parts)}]"


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(BuiltinEnum):
    CHROMIUM = 1


DEFAULT_BROWSER_WIDTH = 1920.0
DEFAULT_BROWSER_HEIGHT = 1080.0


@node_(NodeType.BROWSER, has_subtypes=True)
class Browser(Resource[BrowserData]):
    """A Browser instance for web browsing."""

    type: BrowserType = p_regular(30, default=BrowserType.CHROMIUM)

    version: str | None = p_regular(60, default=None)
    target_version: Optional[str] = p_regular(61, default=None)
    external_name: Optional[str] = p_kernel(62, require=False, default=None, sensitive=True)
    external_id: Optional[str] = p_kernel(63, require=False, default=None, sensitive=True)
    connection_uri: Optional[str] = p_system(
        64, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    debugger_uri: Optional[str] = p_system(
        65, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    view_uri: Optional[str] = p_system(
        66, require=False, default=None, encrypt=True, defer=True, sensitive=True
    )
    client: Optional["Client"] = p_system(
        67, require=False, array=False, references=NodeType.CLIENT, fk=True, same_bench=True
    )

    # settings
    size: "Vector2 | None" = p_regular(70, require=False, array=False, struct=StructType.VECTOR2)
    is_headless: bool = p_system(75, default=False)
    is_insecure: bool = p_system(76, default=False)
