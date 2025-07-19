from collections.abc import AsyncGenerator
from contextlib import contextmanager

import grpclib
import pytest
import pytest_asyncio
import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env
from destack.test.fixtures import create_space

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()

from destack.language import (
    Session,
    StoreKey,
)
from destack.store import MemoryEntityStore, MemoryStore

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

pytestmark = pytest.mark.asyncio(loop_scope="session")


@pytest.fixture
def memory_store() -> MemoryStore:
    return MemoryStore(keys=tuple(StoreKey))


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def memory_session(memory_store: MemoryEntityStore) -> AsyncGenerator[Session, None]:
    session = Session(store=memory_store, epoch=0)
    await session.open()
    yield session
    await session.close()


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def session(memory_store: MemoryEntityStore):
    """Default Session is in-memory."""
    session = Session(store=memory_store, epoch=0)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def space(session: Session):
    space, branch, snapshot = create_space(session)
    with space.active(), branch.active(), snapshot.active():
        yield space


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
