from contextlib import asynccontextmanager
from datetime import datetime

import structlog

from bench.language import Session
from bench.language.builtin import _active_session
from bench.proto.mesh import BenchServiceBase
from bench.proto.wire import (
    ReadNodesRequest,
    ReadNodesResponse,
    SearchNodesRequest,
    SearchNodesResponse,
    AggregateNodesRequest,
    AggregateNodesResponse,
    CommitEditsRequest,
    CommitEditsResponse,
    ModuleHostBase,
)

logger = structlog.get_logger(__name__)


class SystemBenchHost(BenchServiceBase, ModuleHostBase):
    """
    Implement the Bench IO related parts of a module host as the system (with full permissions).
    This is never intended to be exposed to the network, only used in the same process.
    """

    async def read_nodes(self, read_nodes_request: "ReadNodesRequest") -> "ReadNodesResponse":
        raise NotImplementedError

    async def search_nodes(
        self, search_nodes_request: "SearchNodesRequest"
    ) -> "SearchNodesResponse":
        raise NotImplementedError

    async def aggregate_nodes(
        self, aggregate_nodes_request: "AggregateNodesRequest"
    ) -> "AggregateNodesResponse":
        raise NotImplementedError

    async def commit_edits(
        self, commit_edits_request: "CommitEditsRequest"
    ) -> "CommitEditsResponse":
        raise NotImplementedError


_system_host = SystemBenchHost().to_loopback_stub()


@asynccontextmanager
async def detached_session(commit: bool = False, readonly: bool = False) -> "Session":
    """Get a global session."""
    assert not readonly or not commit, "readonly and commit are mutually exclusive"
    assert _active_session.get() is None, f"already in active session {_active_session.get()}"
    session = Session(parent=None, _host=_system_host)
    _active_session.set(session)
    try:
        yield session
        if commit:
            await session.commit()
        elif session.has_regular_edits:
            if readonly:
                raise RuntimeError(f"readonly session {session!r} has edits")
            logger.warning("session.discard", session=session)
    finally:
        session.closed_at = datetime.utcnow()  # pretend close to prevent further use
        _active_session.set(None)
