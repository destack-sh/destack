import warnings
from collections.abc import AsyncGenerator, Generator, Mapping
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
    ACTIVE_SESSION,
    NODE_TYPES,
    REGION,
    STRUCT_TYPES,
    WORLD_ORACLE,
    BuiltinObject,
    NodeType,
    Session,
    Space,
    SpaceStatus,
    StoreKey,
    StructType,
)
from destack.store import MemoryStore
from destack.test.conftest import _setup_test_env

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
    session = Session(store=MemoryStore(keys=tuple(StoreKey)))
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def space(session: Session) -> Generator[Space, None, None]:
    space = Space(
        name="Test",
        slug="test",
        status=SpaceStatus.ACTIVE,
        region=REGION,
    )
    session.create(space)
    with space.active():
        yield space


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"


SHARED_SESSION = Session(oracle=WORLD_ORACLE)


# init shared builtin objects (in shared session)
with warnings.catch_warnings(action="ignore"):
    ACTIVE_SESSION.set(SHARED_SESSION)
    BUILTIN_OBJECTS = [
        # draw_direct(from_object_type(object_type, reject_invalid=False))
        # for object_type in OBJECT_TYPES
    ]
    ACTIVE_SESSION.set(None)

BUILTIN_OBJECTS_BY_TYPE: Mapping[StructType | NodeType, BuiltinObject] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
