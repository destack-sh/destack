import warnings
from collections.abc import AsyncGenerator, Mapping
from contextlib import contextmanager

import grpclib
import pytest
import structlog
from opentelemetry import trace

from destack.store import MemoryStore
from destack.test.conftest import _setup_test_env

# ruff: noqa: E402
# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.language import (
    ACTIVE_SESSION,
    Session,
    StoreType,
)

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


# NOTE: we manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture
def memory_store() -> MemoryStore:
    return MemoryStore(types=tuple(StoreType))


@pytest.fixture  # :PytestAsyncContext
async def memory_session_async(memory_store: MemoryStore) -> AsyncGenerator[Session, None]:
    session = Session(store=memory_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def memory_session(memory_session_async: Session):
    token = ACTIVE_SESSION.set(memory_session_async)
    yield memory_session_async
    ACTIVE_SESSION.reset(token)


@pytest.fixture
def session(memory_session_async: Session):
    """Default Session is in-memory."""
    token = ACTIVE_SESSION.set(memory_session_async)
    yield memory_session_async
    ACTIVE_SESSION.reset(token)


@contextmanager
def raises_grpc_error(*statuses: grpclib.const.Status):
    with pytest.raises(grpclib.GRPCError) as exc_info:
        yield
    if statuses:
        assert exc_info.value.status in statuses, f"expected {statuses}, got {exc_info!r}"


from destack.language import (
    NODE_TYPES,
    STRUCT_TYPES,
    WORLD_ORACLE,
    BuiltinObjectBase,
    NodeType,
    Session,
    StructType,
)
from destack.test.conftest import _setup_test_env

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


SHARED_SESSION = Session(oracle=WORLD_ORACLE)


# init shared builtin objects (in shared session)
with warnings.catch_warnings(action="ignore"):
    ACTIVE_SESSION.set(SHARED_SESSION)
    BUILTIN_OBJECTS = [
        # draw_direct(from_object_type(object_type, reject_invalid=False))
        # for object_type in OBJECT_TYPES
    ]
    ACTIVE_SESSION.set(None)

BUILTIN_OBJECTS_BY_TYPE: Mapping[StructType | NodeType, BuiltinObjectBase] = {
    obj.metatype: obj for obj in BUILTIN_OBJECTS
}
STRUCTS = [BUILTIN_OBJECTS_BY_TYPE[t] for t in STRUCT_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
NODES = [BUILTIN_OBJECTS_BY_TYPE[t] for t in NODE_TYPES if t in BUILTIN_OBJECTS_BY_TYPE]
