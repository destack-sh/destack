import pytest
import pytest_asyncio
import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()

from destack.graph import MemoryGraph
from destack.language import REGION, Session, Space, Universe
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
    result = Space.create_space(
        session,
        name="Test",
        slug="test",
        owned_by=Universe.ACTOR,
        region=REGION,
    )
    with (
        result.space.active(),
        result.root_branch.active(),
        result.head_snapshot.active(),
    ):
        yield result.space
