from collections.abc import AsyncGenerator
from contextlib import contextmanager

import grpclib
import pytest
import pytest_asyncio
import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.language import (
    REGION,
    WORLD_ORACLE,
    Branch,
    BranchType,
    NodeReference,
    NodeType,
    Session,
    Snapshot,
    SnapshotType,
    Space,
    SpaceStatus,
    StoreKey,
)
from destack.store import MemoryStore
from destack.test.conftest import _setup_test_env
from destack.utils.uuid import uuid4

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def memory_session() -> AsyncGenerator[Session, None]:
    session = Session(store=MemoryStore(keys=tuple(StoreKey)))
    await session.open()
    yield session
    await session.close()


@pytest_asyncio.fixture(loop_scope="session", scope="function")
async def session():
    """Default Session is in-memory."""
    session = Session(store=MemoryStore(keys=tuple(StoreKey)), epoch=0)
    await session.open()
    yield session
    await session.close()


def create_space(session: Session) -> tuple[Space, Branch, Snapshot]:
    space_id = uuid4()
    epoch = session.epoch
    now = session.oracle.utc()
    space_ptr = NodeReference(
        type=NodeType.SPACE,
        id=space_id,
        space_id=space_id,
    )
    snapshot_id = uuid4()
    snapshot_ptr = NodeReference(
        type=NodeType.SNAPSHOT,
        id=snapshot_id,
        space_id=space_id,
        snapshot_id=snapshot_id,
    )
    branch_id = uuid4()
    branch_ptr = NodeReference(
        type=NodeType.BRANCH,
        id=branch_id,
        space_id=space_id,
    )
    snapshot = Snapshot(
        id=snapshot_id,
        name="Root",
        space_ptr=space_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
        type=SnapshotType.FULL,
        branch_ptr=branch_ptr,
    )
    branch = Branch(
        id=branch_id,
        name="Main",
        space_ptr=space_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
        type=BranchType.ROOT,
        branch_ptr=branch_ptr,
        snapshot_ptr=snapshot_ptr,
    )
    space = Space(
        id=space_id,
        name="Test",
        slug="test",
        status=SpaceStatus.ACTIVE,
        region=REGION,
        branch_ptr=branch_ptr,
        snapshot_ptr=snapshot_ptr,
        created_epoch=epoch,
        created_at=now,
        updated_epoch=epoch,
        updated_at=now,
    )
    session.create(space)
    session.create(snapshot)
    return space, branch, snapshot


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


SHARED_SESSION = Session(oracle=WORLD_ORACLE)
