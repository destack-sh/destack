from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language import (
    RESOURCE_NODE_TYPES,
    Computer,
    GetConnection,
    GraphCapture,
    Message,
    NodeReference,
    NodeType,
    Plan,
    ResourceStatus,
    SearchConnection,
    Thread,
)
from bench.pb2 import ComputerClient

if TYPE_CHECKING:
    from bench.runtime import Runtime

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

THREAD_QUERY = Thread.include_descendants(
    *RESOURCE_NODE_TYPES,
    NodeType.MEMBERSHIP,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.AGENT,
    NodeType.CURSOR,
)


class ThreadHandle:
    """
    A handle to a Thread at runtime.
    Automatically loads the Thread and its Messages.
    """

    __slots__ = (
        "_computer_clients_by_uri",
        "_messages_connection",
        "_thread_connection",
        "capture",
        "id",
        "runtime",
        "thread_ptr",
    )

    def __init__(self, *, runtime: "Runtime", thread_ptr: NodeReference):
        self.id = thread_ptr.id
        self.runtime = runtime
        self.thread_ptr = thread_ptr
        self.capture: GraphCapture = GraphCapture()
        self._computer_clients_by_uri: dict[str, ComputerClient] = {}
        self._thread_connection: GetConnection | SearchConnection | None = None
        self._messages_connection: SearchConnection | None = None

    def __str__(self) -> str:
        content_parts: list[str] = []
        if self._thread_connection is not None:
            content_parts.append(f"thread={self.thread!r}")
        else:
            content_parts.append(f"thread={self.thread_ptr!r}")
        if self._messages_connection is not None:
            content_parts.append(f"messages={len(self.messages)}")
            if (last_message := self.last_message) is not None:
                content_parts.append(f"last_message={last_message!r}")
        return f"{', '.join(content_parts)}"

    def __repr__(self) -> str:
        return f"<{self.__class__.__name__} {self!s}>"

    @property
    def thread(self) -> Thread:
        """The Thread."""
        assert self._thread_connection is not None, f"{self!r} is not ready"
        thread = self._thread_connection.result.roots[0]
        return thread

    @property
    def main_plan(self) -> Plan | None:
        """The Plan (if any)."""
        return self.thread.main_plan

    @property
    def messages(self) -> list[Message]:
        """The Messages."""
        assert self._messages_connection is not None, f"{self!r} is not ready"
        return self._messages_connection.result.roots

    @property
    def last_message(self) -> Message | None:
        """The last Message (if any)."""
        if (
            self._messages_connection is not None
            and len(self._messages_connection.result.roots) > 0
        ):
            return self._messages_connection.result.roots[-1]
        else:
            return None

    async def open(self):
        # NOTE :Performance: limit Runtime Thread.messages (to like 100? 200?)
        async with self.capture.capture():
            _, self._thread_connection = await THREAD_QUERY.get_connection(
                self.thread_ptr, live=True
            )
            _, self._messages_connection = (
                await Message.where(Message.get_property("thread").eq(self.thread_ptr))
                .order_by(Message.get_property("created_at").asc())
                .search_connection(live=True)
            )

    def close(self):
        if self.capture is not None:
            self.capture.close_and_detach()

    async def get_computer_client(self, computer: Computer) -> ComputerClient:
        """Get a ComputerClient for the given Computer and display."""
        if (
            computer.grpc_url is None
            or not computer.is_active
            or computer.status != ResourceStatus.AVAILABLE
        ):
            raise ValueError(f"{computer!r} has no connection info")

        if computer.grpc_url not in self._computer_clients_by_uri:
            computer_client = ComputerClient(
                await self.runtime.network.get_channel(computer.grpc_url, source_id="runtime")
            )
            self._computer_clients_by_uri[computer.grpc_url] = computer_client
            logger.debug("thread_handle.connect", computer=computer)
        else:
            computer_client = self._computer_clients_by_uri[computer.grpc_url]

        return computer_client
