from collections.abc import AsyncGenerator

import pytest
import pytest_asyncio

from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.graph import MemoryGraph
from destack.language import REGION, Context, Session, Space, Universe
from destack.test.conftest import _setup_test_env
from destack.utils.uuid import uuid4


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def memory_session() -> AsyncGenerator[Session, None]:
    root_context = Context(
        actor_ptr=Universe.ACTOR,
        client_ptr=Universe.CLIENT,
        client_nonce=uuid4(),
    )
    session = Session(
        root_context=root_context,
        graph=MemoryGraph(),
        remote_epoch=0,
        local_epoch=0,
    )
    root_context._session = session
    async with session.active():
        yield session


session = memory_session


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
