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
    StructType,
    Text,
    enum_,
    node_,
    p_node_parent,
    p_regular,
    p_system,
)
from bench.pb2 import LinkData

if TYPE_CHECKING:
    from bench.language import Channel, Database, File, Package, Page, Run, Thread

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# pyright: reportIncompatibleVariableOverride=false


@enum_(EnumType.LINK_TYPE)
class LinkType(BuiltinEnum):
    WEB = 1


@node_(NodeType.LINK, has_subtypes=True)
class Link(IsTitled, Resource[LinkData]):
    """
    A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
    """

    # meta
    parent: Union["Package", "Page", "Database", "Link", "Channel", "Thread", "Run", None] = (
        p_node_parent(
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
    )
    type: LinkType = p_regular(30, require=True)

    # content
    url: str | None = p_regular(50, require=False)
    image: Optional["File"] = p_regular(51, require=False, references=NodeType.FILE)
    image_url: Optional[str] = p_regular(52, require=False)
    text: Optional["Text"] = p_regular(53, require=False, struct=StructType.TEXT)

    expires_at: Optional[datetime] = p_system(60)
