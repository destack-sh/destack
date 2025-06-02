from collections.abc import AsyncGenerator

import pytest
from fastuuid import uuid4

from bench.language import (
    ACTIVE_SESSION,
    DatabaseInfo,
    NodeReference,
    NodeType,
    Session,
    User,
    UserStatus,
)
from bench.store import DatabaseStore


@pytest.fixture
def database_store(omni_database: DatabaseInfo) -> DatabaseStore:
    return DatabaseStore(database=omni_database)


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


# from bench.language import *

# q = User.get(
#     where=User.property("id").eq(5),
#     Clients=Client.search(),
#     BenchMemberships=BenchMembership.search(
#         sort=[BenchMembership.property("created_at").desc()],
#     ),
# )

# q = Bench.get(
#     "Bench",
#     where=Bench.property("id").eq(5),
#     Packages=Package.search(
#         Pages=Page.search(limit=10, count=True),
#     ),
#     Pages=Page.search(count=True),
# )


# q = Thread.get(
#     where=Thread.property("id").eq(5),
#     Messages=Message.search(
#         sort=[Message.property("created_at").asc()],
#         limit=100,
#         count=True,
#     ),
# )

# q = Thread.search(
#     sort=[Thread.property("last_active_at").asc()],
#     limit=25,
#     count=True,
#     Cursor=Cursor.get(
#         join=join(JoinType.LEFT, on=Cursor.property("owned_by").eq(5)),
#         UnreadCount=Message.count(
#             where=Message.property("read_at").greater_than(attribute_ref("Cursor.last_read_at")),
#         ),
#     ),
# )

# q = Space.get(
#     where=Space.property("id").eq(5),
#     Scenes=Scene.search(
#         Fields=Field.search(),
#         Themes=Theme.search(),
#         Views=IsView.search(join=join(JoinType.PARENT, recursive=True)),
#         Styles=IsStyle.search(join=join(JoinType.PARENT, recursive=True)),
#     ),
#     Route=Route.search(),
# )


async def test_create_node(session: Session):
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        bench_ptr=NodeReference(node_type=NodeType.BENCH, id=uuid4()),
    )
    session.create(user)
    await session.commit()
    await session.commit()
    user = await User.get(where=User.property("id").eq(user.id)).execute_one()
