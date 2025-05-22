from datetime import datetime
from typing import TYPE_CHECKING, Collection

import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    Computer,
    Cursor,
    GetConnection,
    GraphCapture,
    IsSubject,
    Message,
    MessageType,
    NodeReference,
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
    # TODO :Robustness :RichGraph: ThreadHandle.messages is delayed because search connections are updated from Host only
    """

    __slots__ = (
        "_computer_clients_by_uri",
        "_messages",
        "_messages_connection",
        "_optimistic_messages",
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
        self._thread_connection: GetConnection | SearchConnection = thread_connection
        self._messages_connection = messages_connection
        self._messages: list[Message] | None = None
        self._optimistic_messages: list[Message] = []
        self._messages_connection.on_update(lambda *args: self._on_messages_update())

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
    def messages(self) -> list[Message]:
        """The Messages in this Thread (sorted by created_at)."""
        if self._messages is None:
            self._on_messages_update()
            assert self._messages is not None
        return self._messages

    def add_optimistic_message(self, message: Message):
        # NOTE: obviously optimistic Messages like this are awful :RichGraph
        self._optimistic_messages.append(message)
        self._on_messages_update()

    def _on_messages_update(self):
        # messages = all messages from connection + optimistic messages (deduped, sorted)
        messages_by_id: dict[UUID, Message] = {}
        for message in self._messages_connection.result.roots:
            messages_by_id[message.id] = message
        for message in self._optimistic_messages:
            messages_by_id[message.id] = message
        self._messages = list(messages_by_id.values())
        self._messages.sort(key=lambda m: m.created_at)

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

    def has_new_messages_for(
        self,
        owner: IsSubject,
        cursor: Cursor | None,
        ignore_types: Collection[MessageType] = (MessageType.JOIN, MessageType.LEAVE),
    ) -> bool:
        """Check if there are new Messages for the given Cursor."""
        return any(
            m.created_by_id != owner.id
            and (cursor is None or cursor.seen_at is None or m.created_at > cursor.seen_at)
            and m.type not in ignore_types
            for m in self.messages
        )

    def close(self):
        if self.capture is not None:
            self.capture.close_and_detach()

    async def get_computer_client(self, computer: Computer) -> ComputerClient:
        """Get a ComputerClient for the given Computer and display."""
        if computer.grpc_url is None or computer.status != ResourceStatus.AVAILABLE:
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
