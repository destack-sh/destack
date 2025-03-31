import abc
import asyncio
from typing import TYPE_CHECKING, ClassVar, Sequence, final, override

import structlog
from opentelemetry import trace

from bench.language import Bench, Node, NodeType, Package, Session, bittuple
from bench.proto import EditData, Network
from bench.utils.oracle import Oracle
from bench.utils.task import TaskManager

if TYPE_CHECKING:
    from bench.system.host import HostService

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
     Also consider plugins that affect 'external' state like the DatabasePlugin.
     Host needs to be more 'multithreaded' in general, see :ConcurrentHost.
    """

    watch_types: ClassVar[bittuple[NodeType] | None] = None

    def __init__(self, host: "HostService", bench: "Bench"):
        self.host = host
        self.bench = bench
        self.tasks = TaskManager(
            owner=self,
            logger=logger,
            on_error=lambda e: host.on_error(e),
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

    @property
    def name(self) -> str:
        return self.__class__.__name__

    @property
    def oracle(self) -> Oracle:
        return self.host.oracle

    @property
    def network(self) -> Network:
        return self.host.network

    @property
    def main_package(self) -> Package:
        return self.host.main_package

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

    async def pre_commit(self, session: Session, commit: Commit[T]) -> Sequence[EditData] | None:
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


def _patch_node(target: Node, reference: Node, track: bool = True) -> None:
    """Patch the target node *in place* from the reference node."""
    for prop in target.__wired_properties__.values():
        if prop.id < 30 or prop.is_computed:
            continue  # ignore internal properties
        target_value = getattr(target, prop.name)
        reference_value = getattr(reference, prop.name)
        if target_value != reference_value:
            target._do_set(prop.name, reference_value, track=track)


class DeferredHostPlugin[T: Node](HostPlugin, abc.ABC):
    """A Host plugin with async event handlers."""

    def __init__(self, host: "HostService", bench: "Bench"):
        super().__init__(host, bench)
        self._processing_commit: Commit[T] | None = None
        self._commit_queue: asyncio.Queue[Commit[T]] = asyncio.Queue()

    @property
    def is_idle(self) -> bool:
        """Check if the plugin is idle (no pending requests or processing)."""
        return self._commit_queue._unfinished_tasks == 0  # type: ignore

    @override
    async def start(self) -> None:
        await super().start()
        self.tasks.run(self.process_commit_queue())

    def filter_commit(self, commit: Commit) -> bool:
        """Filter a commit before adding it to the queue."""
        return True

    def _patch_commit(self, new_commit: Commit, old_commit: Commit) -> None:
        """Patch the commit based on the old commit."""
        for new_node in new_commit.edited:
            old_node = old_commit.edited_by_id.get(new_node.id)
            if old_node is not None:
                _patch_node(old_node, new_node, track=False)

    @override
    async def post_commit(self, session: Session, commit: Commit) -> None:
        if self.filter_commit(commit):
            # queue commit
            self._commit_queue.put_nowait(commit)

        # patch nodes we're currently committing
        for old_commit in self._commit_queue._queue:  # type: ignore
            self._patch_commit(commit, old_commit)
        if self._processing_commit is not None:
            self._patch_commit(commit, self._processing_commit)

    @final
    async def wait_idle(self, timeout: float) -> None:
        if self._commit_queue.empty():
            return  # nothing to wait for
        try:
            await asyncio.wait_for(self._commit_queue.join(), timeout=timeout)
        except asyncio.TimeoutError as e:
            raise RuntimeError(
                f"{self!r} timed out after {timeout}s waiting for {self._commit_queue.qsize()} commits"
            ) from e
        self.tasks.check_no_errors()

    async def process_commit_queue(self) -> None:
        """Process the commit queue."""
        while True:
            commit = await self._commit_queue.get()
            try:
                self._processing_commit = commit
                await self.post_commit_deferred(commit)
            except Exception as e:
                # suppress errors and keep going
                logger.exception("deferred.commit.error", owner=self, commit=commit, exc_info=e)
                self.host.on_error(e)
            finally:
                self._processing_commit = None
                self._commit_queue.task_done()

    async def post_commit_deferred(self, commit: Commit) -> None:
        """
        React to the committed changes (outside the request, later, one at a time).
        """
        pass  # to be overridden
