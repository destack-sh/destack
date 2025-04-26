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
    IsTitled,
    NodeType,
    Resource,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import LinkData

if TYPE_CHECKING:
    from bench.language import Database, Message, Package, Page, Run, Thread

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    WEB = 1


@node_(NodeType.LINK)
class Link(IsTitled, Resource[LinkData]):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    parent: Union[
        "Package",
        "Page",
        "Database",
        "Link",
        "Thread",
        "Message",
        "Run",
        None,
    ] = p_node_parent(
        4,
        NodeType.PACKAGE,
        NodeType.PAGE,
        NodeType.DATABASE,
        NodeType.LINK,
        NodeType.THREAD,
        NodeType.RUN,
        ckless=True,
    )
    type: LinkType = p_regular(30, require=True)

    # content
    url: str | None = p_regular(50, default=None)
    domain: str | None = p_regular(51, default=None)
    content_url: str | None = p_regular(52, default=None)
    thumbnail_url: str | None = p_regular(53, default=None)
    favicon_url: str | None = p_regular(54, default=None)
    thumbnail_width: int | None = p_regular(55, default=None)
    thumbnail_height: int | None = p_regular(56, default=None)
    content: str | None = p_regular(60, default=None)
    attribution: str | None = p_regular(62, default=None)
    published_at: Optional[datetime] = p_system(63)
    expires_at: Optional[datetime] = p_system(64)
    image_urls: list[str] = p_regular(70, array=True)

    def __content_str__(self):
        content_parts: list[str] = [self.type.bench_name]
        if self.url:
            content_parts.append(self.url)
        if self.content_url:
            content_parts.append(self.content_url)
        if self.content:
            content_parts.append(self.content[:100] + "...")
        return ", ".join(content_parts)
