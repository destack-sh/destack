from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
    Union,
)

import structlog
from opentelemetry import trace

from bench.language.core import (
    BuiltinEnum,
    EnumType,
    IsResource,
    IsTitled,
    Node,
    NodeType,
    enum_,
    node_,
    p_node_parent,
    p_system,
    property_,
)
from bench.pb2 import LinkData

if TYPE_CHECKING:
    from bench.language import Message, Package, Page, Run, Table, Thread

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    WEB = 1


@node_(NodeType.LINK)
class Link(IsTitled, IsResource, Node[LinkData]):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    parent: Union[
        "Package",
        "Page",
        "Table",
        "Link",
        "Thread",
        "Message",
        "Run",
        None,
    ] = p_node_parent()
    type: LinkType = property_(30)

    # content
    url: str | None = property_(50)
    domain: str | None = property_(51)
    content_url: str | None = property_(52)
    thumbnail_url: str | None = property_(53)
    favicon_url: str | None = property_(54)
    thumbnail_width: int | None = property_(55)
    thumbnail_height: int | None = property_(56)
    content: str | None = property_(60)
    attribution: str | None = property_(62)
    attribution_tag: str | None = property_(63)
    published_at: Optional[datetime] = p_system(64)
    expires_at: Optional[datetime] = p_system(65)
    image_urls: list[str] = property_(70)

    def __content_str__(self):
        content_parts: list[str] = [self.type.bench_name]
        if self.url:
            content_parts.append(self.url)
        if self.content_url:
            content_parts.append(self.content_url)
        if self.content:
            content_parts.append(self.content[:100] + "...")
        return ", ".join(content_parts)
