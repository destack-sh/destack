from bench.language import (
    RESOURCE_NODE_TYPES,
    GetConnection,
    GraphCapture,
    Message,
    NodeReference,
    NodeType,
    Plan,
    SearchConnection,
    Thread,
)

THREAD_QUERY = Thread.include_descendants(
    *RESOURCE_NODE_TYPES,
    NodeType.MEMBERSHIP,
    NodeType.PLAN,
    NodeType.TASK,
    NodeType.CLAIM,
    NodeType.IDENTITY,
)


class RuntimeThread:
    """
    A handle to a Thread at runtime.
    Automatically loads the Thread and its Messages.
    """

    __slots__ = (
        "_messages_connection",
        "_thread_connection",
        "capture",
        "id",
        "thread_ptr",
    )

    def __init__(self, thread_ptr: NodeReference):
        self.id = thread_ptr.id
        self.thread_ptr = thread_ptr
        self.capture: GraphCapture = GraphCapture()
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

    async def close(self):
        self.capture.close_and_detach()
