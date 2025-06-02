from collections.abc import AsyncGenerator

import pytest
from fastuuid import uuid4

from bench.language import (
    ACTIVE_SESSION,
    CustomNodeDefinition,
    DatabaseInfo,
    NodeReference,
    NodeType,
    Session,
    User,
    UserStatus,
)
from bench.language.core.const import TraitType
from bench.store import DatabaseStore


@pytest.fixture
def database_store(omni_database: DatabaseInfo) -> DatabaseStore:
    return DatabaseStore(database=omni_database, area=None)


@pytest.fixture  # :PytestAsyncContext
async def session_async(database_store: DatabaseStore) -> AsyncGenerator[Session, None]:
    session = Session(store=database_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def session(session_async: Session):
    token = ACTIVE_SESSION.set(session_async)
    yield session_async
    ACTIVE_SESSION.reset(token)


async def test_create_user(session: Session):
    """Create and update a User, querying along the way."""
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        bench_ptr=NodeReference(node_type=NodeType.BENCH, id=uuid4()),
    )
    session.create(user)
    await session.commit()

    user.name = "Fluff"
    user.slug = "flotothemoon"
    await session.commit()

    user_unpacked = await User.get(where=User.property("id").eq(user.id)).execute_one()
    assert user.equals(user_unpacked)

    user_unpacked = await User.get(where=User.property("slug").eq("flotothemoon")).execute_one()
    assert user_unpacked.name == "Fluff"
    assert user_unpacked.slug == "flotothemoon"
    assert user_unpacked.status == UserStatus.ACTIVE


async def test_create_custom_node(session: Session):
    """Create a custom Node, mutate it, querying along the way."""
    # nocheckin: custom nodes
    custom_node_definition = CustomNodeDefinition(
        name="Event",
        traits=[
            TraitType.NAMED,
            TraitType.SLUG,
            TraitType.DELETABLE,
            TraitType.ARCHIVABLE,
            TraitType.OWNABLE,
        ],
    )
    session.create(custom_node_definition)
    await session.commit()
