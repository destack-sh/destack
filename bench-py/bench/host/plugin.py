from functools import cached_property
from typing import TYPE_CHECKING, ClassVar

import structlog
from opentelemetry import trace

from bench.language import Bench, Node, NodeType, Session, bittuple
from bench.proto import Network
from bench.utils.oracle import Oracle
from bench.utils.string import Casing, to_casing
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    from .host import HostService

from .commit import Commit

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class HostPlugin[T: Node]:
    """
    A plugin into the Host operating system of a Bench.
    TODO :Architecture! :Performance!: revisit HostPlugins for Package/transaction isolation
     Maybe this could work a bit like in ProseMirror where (some) state is isolated outside
     of the individual plugins - that would also make it easier to reset on error.
     We probably also want to load in/out Packages at some point (?)
     Also consider plugins that affect 'external' state like the TablePlugin.
    """

    watch_types: ClassVar[bittuple[NodeType] | None] = None

    def __init__(self, host: "HostService", bench: "Bench"):
        self.host = host
        self.bench = bench
        self.tasks = TaskManager(
            owner=self,
            logger=logger,
            on_error=host.on_error,
            task_id_prefix=f"{self.bench.slug}_{self.__class__.__name__}",
            oracle=host.oracle,
        )

    def __str__(self) -> str:
        return ""

    def __repr__(self) -> str:
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str} in '{self.bench.slug}'>"
        else:
            return f"<{self.__class__.__name__} in '{self.bench.slug}'>"

    @cached_property
    def name(self) -> str:
        return to_casing(self.__class__.__name__, Casing.SNAKE)

    @property
    def oracle(self) -> Oracle:
        return self.host.oracle

    @property
    def network(self) -> Network:
        return self.host.network

    #
    # Lifecycle
    #

    @property
    def is_idle(self) -> bool:
        """Check if the plugin is idle (no pending requests or processing)."""
        return True

    async def start(self) -> None:
        """Start any work for this plugin, returning when the plugin is ready."""
        pass

    def close(self) -> None:
        """Close any stuff you need to close (if any)."""
        self.tasks.close()

    async def wait_closed(self) -> None:
        """After closing, wait for any stuff you need to wait for (if any)."""
        await self.tasks.wait_closed()

    async def wait_idle(self, timeout: float) -> None:
        """Wait for any pending events to finish processing."""
        pass

    #
    # Events
    #

    async def pre_commit(self, session: Session, commit: Commit[T]) -> None:
        """
        Add edits that logically belong to the same Transaction.
        The commit contains only direct edits, not cascaded edits.
        Add any new Edits to the Session *or* return them.
        """
        pass

    async def post_commit(self, session: Session, commit: Commit[T]) -> None:
        """
        React to the commit in a new Transaction (but still in the request lifecycle).
        The commit contains edits and cascaded edits.
        """
        pass

    async def post_commit_failed(self, session: Session, error: BaseException) -> None:
        """
        React to a failed commit.
        """
        pass
