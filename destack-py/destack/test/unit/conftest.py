# ruff: noqa: E402

import warnings
from collections.abc import Mapping

import pytest
import structlog
from opentelemetry import trace

from destack.test.conftest import _setup_test_env

# NOTE: must run setup before importing from destack
_setup_test_env()


from destack.language import (
    ACTIVE_SESSION,
    NODE_TYPES,
    STRUCT_TYPES,
    BuiltinObjectBase,
    Database,
    EnvironmentType,
    Folder,
    FolderType,
    NodeType,
    Session,
    Space,
    SpaceStatus,
    StructType,
)
from destack.test.conftest import _setup_test_env
from destack.utils.oracle import REAL_ORACLE

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def make_session(name: str):
    """Make a 'fake' session for context"""
    session = Session(mode=EnvironmentType.STAGING, oracle=REAL_ORACLE)
    return session


# NOTE: we manually set session sync context since it's not propagated across pytest tasks (see above)
# https://github.com/pytest-dev/pytest-asyncio/issues/127#issuecomment-862817549 :PytestAsyncContext


@pytest.fixture  # :PytestAsyncContext
async def session_async(request):
    session = make_session(request.node.name)
    await session.open()
    yield session
    await session.close()


def make_package(session: Session):
    space = Space(name="test", slug="test", status=SpaceStatus.RUNNING)
    package = Folder(type=FolderType.HOME, name="Home", slug="home")
    space.add_child(package)
    database = Database(name="Database")
    package.add_child(database)
    space.database = database
    session.space = space
    return package


@pytest.fixture
def session(session_async: Session):
    token = ACTIVE_SESSION.set(session_async)
    yield session_async
    ACTIVE_SESSION.reset(token)


@pytest.fixture
def package(session: Session):
    package = make_package(session)
    return package


SHARED_SESSION = make_session("shared")


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


# TODO :Test! :Performance: re-use Simulations somehow (databases?)
