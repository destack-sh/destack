from datetime import datetime
from typing import (
    TYPE_CHECKING,
    Optional,
)

import structlog
from opentelemetry import trace

from destack.language.core import (
    Enum,
    EnumType,
    IsSpatial,
    NodeType,
    Resource,
    builtin_enum,
    builtin_node,
    builtin_property,
)

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@builtin_enum(EnumType.LINK_TYPE)
class LinkType(Enum):
    WEB = 1


@builtin_node(NodeType.LINK)
class Link(IsSpatial, Resource):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    type: LinkType = builtin_property(30, is_repr=True)

    # content
    url: str | None = builtin_property(50, is_repr=True)
    domain: str | None = builtin_property(51, is_repr=True)
    content_url: str | None = builtin_property(52)
    thumbnail_url: str | None = builtin_property(53)
    favicon_url: str | None = builtin_property(54)
    thumbnail_width: int | None = builtin_property(55)
    thumbnail_height: int | None = builtin_property(56)
    content: str | None = builtin_property(60)
    attribution: str | None = builtin_property(62)
    attribution_tag: str | None = builtin_property(63)
    published_at: Optional[datetime] = builtin_property(64)
    expires_at: Optional[datetime] = builtin_property(65)
    image_urls: list[str] = builtin_property(70)
