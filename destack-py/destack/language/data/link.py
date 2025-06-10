from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
)

import structlog
from opentelemetry import trace

from destack.language.core import (
    BuiltinEnum,
    EnumType,
    IsAsset,
    IsInFolder,
    Node,
    NodeType,
    enum_,
    node_,
    property_,
)
from destack.pb2 import LinkData

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    WEB = 1


@node_(NodeType.LINK)
class Link(
    IsInFolder,
    IsAsset,
    Node[LinkData],
):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    type: LinkType = property_(30, is_repr=True)

    # content
    url: str | None = property_(50, is_repr=True)
    domain: str | None = property_(51, is_repr=True)
    content_url: str | None = property_(52)
    thumbnail_url: str | None = property_(53)
    favicon_url: str | None = property_(54)
    thumbnail_width: int | None = property_(55)
    thumbnail_height: int | None = property_(56)
    content: str | None = property_(60)
    attribution: str | None = property_(62)
    attribution_tag: str | None = property_(63)
    published_at: Optional[datetime] = property_(64)
    expires_at: Optional[datetime] = property_(65)
    image_urls: list[str] = property_(70)
