from bench.language.bench import AnonymousResourceNode
from bench.language.const import NodeType
from bench.language.node import node_
from bench.proto.wire.lang_pb2 import BrowserData


@node_(NodeType.BROWSER)
class Browser(AnonymousResourceNode[BrowserData]):
    """Browser instance for web browsing."""

    ...
