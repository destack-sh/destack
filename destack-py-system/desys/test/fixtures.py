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

from destack.graph import MemoryGraph
from destack.language import Session, Universe
from destack.utils.uuid import uuid4

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

pytestmark = pytest.mark.asyncio(loop_scope="session")


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def session():
    """Default Session is in-memory."""
    session = Session(
        graph=MemoryGraph(),
        actor=Universe.ACTOR,
        client=Universe.CLIENT,
        client_nonce=uuid4(),
        epoch=0,
    )
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def space(session: Session):
    space, branch, snapshot = create_space(
        session,
        name="Test",
        slug="test",
        owned_by=Universe.ACTOR,
    )
    with space.active(), branch.active(), snapshot.active():
        yield space


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"
