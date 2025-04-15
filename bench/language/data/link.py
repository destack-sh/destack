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
    BuiltinObject,
    EnumType,
    IsTitled,
    NodeType,
    Resource,
    Struct,
    StructType,
    TextLine,
    enum_,
    node_,
    object_,
    p_node_parent,
    p_regular,
    p_system,
    struct_,
)
from bench.pb2 import LinkData

if TYPE_CHECKING:
    from bench.language import Channel, Database, Message, Package, Page, Run, Thread

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    WEB = 1


@object_()
class LinkBase(BuiltinObject):
    """Common Link info."""

    url: str | None = p_regular(50, default=None)
    content_url: str | None = p_regular(51, default=None)
    thumbnail_url: str | None = p_regular(52, default=None)
    favicon_url: str | None = p_regular(53, default=None)
    thumbnail_width: int | None = p_regular(54, default=None)
    thumbnail_height: int | None = p_regular(55, default=None)
    content: str | None = p_regular(60, default=None)
    attribution: str | None = p_regular(62, default=None)
    published_at: Optional[datetime] = p_system(63)
    expires_at: Optional[datetime] = p_system(64)


# NOTE :Architecture: having separate Link and LinkPreview feels funky


@struct_(StructType.LINK_PREVIEW)
class LinkPreview(LinkBase, Struct):
    """
    A preview of a Link.
    """

    title: TextLine | None = p_regular(32, default=None, struct=StructType.TEXT_LINE)


@node_(NodeType.LINK, has_subtypes=True)
class Link(IsTitled, LinkBase, Resource[LinkData]):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    parent: Union[
        "Package",
        "Page",
        "Database",
        "Link",
        "Channel",
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
        NodeType.CHANNEL,
        NodeType.THREAD,
        NodeType.RUN,
        ckless=True,
    )
    type: LinkType = p_regular(30, require=True)

    # content
    # ...LinkBase[50-70]
