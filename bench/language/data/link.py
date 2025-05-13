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
    ResourceBase,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
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
class Link(IsTitled, ResourceBase[LinkData]):
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
    ] = p_node_parent(
        4,
        NodeType.PACKAGE,
        NodeType.PAGE,
        NodeType.TABLE,
        NodeType.LINK,
        NodeType.THREAD,
        NodeType.RUN,
        ckless=True,
    )
    type: LinkType = p_regular(30, require=True)

    # content
    url: str | None = p_regular(
        50,
        default=None,
        description="The URL of the link (e.g., https://www.srf.ch/meteo/meteo-stories/meatball-rain)",
    )
    domain: str | None = p_regular(
        51,
        default=None,
        description="The domain of the link (e.g, srf.ch)",
    )
    content_url: str | None = p_regular(52, default=None)
    thumbnail_url: str | None = p_regular(53, default=None)
    favicon_url: str | None = p_regular(54, default=None)
    thumbnail_width: int | None = p_regular(55, default=None)
    thumbnail_height: int | None = p_regular(56, default=None)
    content: str | None = p_regular(
        60,
        default=None,
        description='The text content of the link (e.g., "The meatballs are expected to ...")',
    )
    attribution: str | None = p_regular(
        62,
        default=None,
        description="The attribution of the link (e.g., 'Max Mustermann')",
    )
    attribution_tag: str | None = p_regular(
        63, default=None, description="The attribution tag of the link (e.g., 'SRF')"
    )
    published_at: Optional[datetime] = p_system(64)
    expires_at: Optional[datetime] = p_system(65)
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
