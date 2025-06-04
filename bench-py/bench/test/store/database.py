from collections.abc import AsyncGenerator

import pytest
from fastuuid import uuid4

from bench.language import (
    ACTIVE_SESSION,
    CustomNodeDefinition,
    DatabaseInfo,
    NodeReference,
    NodeType,
    Page,
    Session,
    TraitType,
    User,
    UserStatus,
)
from bench.language.core.text import text_line
from bench.language.space.block import Block, BlockType
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
    # not exists
    assert not await User.exists().execute_exists()
    # create
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        bench_ptr=NodeReference(node_type=NodeType.BENCH, id=uuid4()),
    )
    session.create(user)
    await session.commit()
    # update
    user.name = "Fluff"
    user.slug = "flotothemoon"
    await session.commit()
    # query by id
    user_unpacked = await User.get(where=User.property("id").eq(user.id)).execute_one()
    assert user.equals(user_unpacked)
    # query by slug
    user_unpacked = await User.search(where=User.property("slug").eq("flotothemoon")).execute_one()
    assert user_unpacked.name == "Fluff"
    assert user_unpacked.slug == "flotothemoon"
    assert user_unpacked.status == UserStatus.ACTIVE


async def test_create_page_blocks_recursive(session: Session):
    """Create a Page with recursive sub-Pages and Blocks, mutate it, querying along the way."""
    for a in ("a", "b", "c", "d"):
        # create
        root_page = Page(title=text_line(f"*Test Page {a}*"), slug=f"test-{a}")
        session.create(root_page)
        await session.commit()
        # block tree
        root_block_count = await Block.count(
            where=Block.property("parent").eq(root_page)
        ).execute_count()
        assert root_block_count == 0
        for i in range(4):
            root_block = Block(type=BlockType.PARAGRAPH, line=text_line(f"Test Block {a}/{i}"))
            root_page.add_child(root_block)
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
                    inner_inner_block.node = root_page
        await session.commit()
        # query count
        root_block_count = await Block.count(
            where=Block.property("parent").eq(root_page)
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
