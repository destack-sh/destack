from typing import Optional

from bench.language.bench import PhysicalResourceNode
from bench.language.const import EnumType, NodeType, enum_
from bench.language.node import node_
from bench.language.property import p_kernel, p_regular
from bench.proto.wire.lang_pb2 import BrowserData
from bench.utils.func import IdEnum


@enum_(EnumType.BROWSER_TYPE)
class BrowserType(IdEnum):
    CHROME = 1


@node_(NodeType.BROWSER)
class Browser(PhysicalResourceNode[BrowserData]):
    """A Browser instance for web browsing."""

    type: BrowserType = p_regular(30, default=BrowserType.CHROME)

    version: str | None = p_regular(50, default=None)
    target_version: Optional[str] = p_regular(51, default=None)
    external_name: Optional[str] = p_kernel(52, require=False, default=None, sensitive=True)
