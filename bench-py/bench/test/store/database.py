from collections.abc import AsyncGenerator

import pytest
from fastuuid import uuid4

from bench.language import (
    ACTIVE_SESSION,
    Block,
    BlockType,
    Client,
    ClientType,
    CustomNodeDefinition,
    DatabaseInfo,
    NodeReference,
    NodeType,
    Page,
    Session,
    TraitType,
    User,
    UserStatus,
    text_line,
)
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
    # create user
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        bench_ptr=NodeReference(node_type=NodeType.BENCH, id=uuid4()),
    )
    session.create(user)
    await session.commit()
    # update user
    user.name = "Fluff"
    user.slug = "flotothemoon"
    await session.commit()
    # query user by id
    user_unpacked = await User.get(where=User.property("id").eq(user.id)).execute_one()
    assert user.equals(user_unpacked)
    # query user by slug
    user_unpacked = await User.search(where=User.property("slug").eq("flotothemoon")).execute_one()
    assert user_unpacked.name == "Fluff"
    assert user_unpacked.slug == "flotothemoon"
    assert user_unpacked.status == UserStatus.ACTIVE

    # create clients
    client_a = Client(type=ClientType.WEB, name="Client A")
    client_b = Client(type=ClientType.WEB, name="Client B")
    user.add_children(client_a, client_b)
    await session.commit()
    # query clients
    clients = await Client.search(sort=[Client.property("name").descending()]).execute_list()
    assert clients == [client_b, client_a]

    # query user with clients as children
    connection = await User.get(
        where=User.property("id").eq(user.id),
        Clients=Client.search(),
    ).execute()
    user_unpacked = connection.to_one()
    assert user_unpacked.equals(user)
    clients_unpacked = user_unpacked.get_children(Client)
    assert clients_unpacked == [client_a, client_b]


async def test_create_page_blocks_recursive(session: Session):
    """Create a Page with recursive sub-Pages and Blocks, mutate it, querying along the way."""
    # create
    root_page = Page(title=text_line("*Test Root Page*"), slug="test-root")
    session.create(root_page)
    for a in ("a", "b", "c", "d"):
        # create page
        page = Page(title=text_line(f"*Test Page {a}*"), slug=f"test-{a}")
        root_page.add_child(page)
        # create block tree
        for i in range(4):
            root_block = Block(type=BlockType.PARAGRAPH, line=text_line(f"Test Block {a}/{i}"))
            page.add_child(root_block)
            for j in range(4):
                inner_block = Block(
                    type=BlockType.PARAGRAPH, line=text_line(f"Inner Block {a}/{i}/{j}")
                )
                root_block.add_child(inner_block)
                for k in range(4):
                    inner_inner_block = Block(
                        type=BlockType.PARAGRAPH,
                        line=text_line(f"Inner Inner Block {a}/{i}/{j}/{k}"),
                    )
                    inner_block.add_child(inner_inner_block)
                    # mutate block after creating to test edit optimization
                    inner_inner_block.node = page
        await session.commit()

    # query
    for page in root_page.get_children(Page):
        # query count
        root_block_count = await Block.count(
            where=Block.property("parent").eq(page)
        ).execute_count()
        assert root_block_count == 4
        # page_block_count = 4 * (1 + 4 * (1 + 4))
        # assert block_count == page_block_count


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
