import contextlib
import contextvars
from typing import TYPE_CHECKING, Literal, assert_never

import structlog
from opentelemetry import trace

if TYPE_CHECKING:
    from bench.language import Connection, NodeGraph, NodeLink


# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

_graph_capture: contextvars.ContextVar["GraphCapture | None"] = contextvars.ContextVar(
    "graph_capture", default=None
)


class GraphCapture:
    """A registry of graph-related objects for collective capture and cleanup."""

    __slots__ = ("connections", "graphs", "links", "parent")

    def __init__(self, parent: "GraphCapture | None" = None):
        self.parent = parent
        self.connections: list[Connection] = []
        self.links: list[NodeLink] = []
        self.graphs: list[NodeGraph] = []

    def add_connection(self, connection: "Connection"):
        """Capture a Connection."""
        if self.parent is not None:
            self.parent.add_connection(connection)
        self.connections.append(connection)

    def add_link(self, link: "NodeLink"):
        """Capture a NodeLink."""
        if self.parent is not None:
            self.parent.add_link(link)
        self.links.append(link)

    def add_graph(self, graph: "NodeGraph"):
        """Capture a NodeGraph."""
        if self.parent is not None:
            self.parent.add_graph(graph)
        self.graphs.append(graph)

    def remove_connection(self, connection: "Connection"):
        """Remove a Connection (if it exists)."""
        if self.parent is not None:
            self.parent.remove_connection(connection)
        if connection in self.connections:
            self.connections.remove(connection)

    def remove_link(self, link: "NodeLink"):
        """Remove a NodeLink (if it exists)."""
        if self.parent is not None:
            self.parent.remove_link(link)
        if link in self.links:
            self.links.remove(link)

    def remove_graph(self, graph: "NodeGraph"):
        """Remove a NodeGraph (if it exists)."""
        if self.parent is not None:
            self.parent.remove_graph(graph)
        if graph in self.graphs:
            self.graphs.remove(graph)

    def close_and_detach(self):
        """Close and detach all captured objects."""
        for link in self.links:
            link.close()
        for connection in self.connections:
            connection.close(detach=True)
        for graph in self.graphs:
            if graph in graph.supergraph._graphs:
                graph.supergraph.remove_graph(graph)

    @contextlib.asynccontextmanager
    async def capture(self):
        """Capture the graph objects in this context."""
        token = _graph_capture.set(self)
        try:
            yield self
        finally:
            _graph_capture.reset(token)


@contextlib.asynccontextmanager
async def isolated_graph(  # noqa: RUF029
    on_exit: Literal["close_and_detach"] = "close_and_detach",
):
    """
    Capture and isolate NodeGraphs, NodeLinks and Connections created in this context.
    """
    # create capture
    capture = GraphCapture(parent=_graph_capture.get())
    token = _graph_capture.set(capture)

    try:
        yield capture.connections
    finally:
        _graph_capture.reset(token)
        if on_exit == "close_and_detach":
            capture.close_and_detach()


def capture(obj: "Connection | NodeGraph | NodeLink"):
    """Capture the graph object in this context."""
    capture = _graph_capture.get()
    if capture is None:
        return

    from bench.language import Connection, NodeGraph, NodeLink

    if isinstance(obj, Connection):
        capture.add_connection(obj)
    elif isinstance(obj, NodeLink):
        capture.add_link(obj)
    elif isinstance(obj, NodeGraph):
        capture.add_graph(obj)
    else:
        assert_never(obj)


def uncapture(obj: "Connection | NodeGraph | NodeLink"):
    """Remove the graph object from this context."""
    capture = _graph_capture.get()
    if capture is None:
        return

    from bench.language import Connection, NodeGraph, NodeLink

    if isinstance(obj, Connection):
        capture.remove_connection(obj)
    elif isinstance(obj, NodeLink):
        capture.remove_link(obj)
    elif isinstance(obj, NodeGraph):
        capture.remove_graph(obj)
    else:
        assert_never(obj)
