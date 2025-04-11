from datetime import datetime
from typing import TYPE_CHECKING

import structlog
from opentelemetry import trace

from bench.language import (
    Agent,
    Computer,
    GetConnection,
    GraphCapture,
    LocalNodeList,
    Message,
    NodeReference,
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

    def __init__(
        self,
        *,
        runtime: "Runtime",
        capture: GraphCapture,
        thread_ptr: NodeReference,
        thread_connection: GetConnection | SearchConnection,
        messages_connection: SearchConnection,
    ):
        self.id = thread_ptr.id
        self.runtime = runtime
        self.capture = capture
        self.thread_ptr = thread_ptr
        self._computer_clients_by_uri: dict[str, ComputerClient] = {}
        self._thread_connection: GetConnection | SearchConnection | None = thread_connection
        self._messages_connection: SearchConnection | None = messages_connection

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
    def agents(self) -> LocalNodeList[Agent]:
        """The Agents."""
        return self.thread.agents

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

    @property
    def last_message_at(self) -> datetime | None:
        """The timestamp of the last Message (if any)."""
        if (message := self.last_message) is not None:
            return message.created_at
        else:
            return None

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
