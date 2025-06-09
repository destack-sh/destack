from collections.abc import AsyncGenerator

import pytest
from fastuuid import uuid4
from hypothesis import HealthCheck, given, settings

from bench.language import (
    ACTIVE_SESSION,
    Block,
    BlockType,
    Client,
    ClientType,
    CustomView,
    DatabaseInfo,
    FrameView,
    IsView,
    JoinType,
    LabelView,
    Node,
    NodeReference,
    NodeType,
    Page,
    Scene,
    Session,
    TextView,
    User,
    UserStatus,
    join,
    text_line,
)
from bench.store import PostgresStore
from bench.test.strategies import examples, nodes
from bench.test.unit.conftest import NODES


@pytest.fixture
def postgres_store(omni_postgres_database: DatabaseInfo) -> PostgresStore:
    return PostgresStore(database=omni_postgres_database, area=None)


@pytest.fixture  # :PytestAsyncContext
async def session_async(postgres_store: PostgresStore) -> AsyncGenerator[Session, None]:
    session = Session(store=postgres_store)
    await session.open()
    yield session
    await session.close()


@pytest.fixture
def session(session_async: Session):
    token = ACTIVE_SESSION.set(session_async)
    yield session_async
    ACTIVE_SESSION.reset(token)


@given(node=nodes)
@examples([{"node": node} for node in NODES])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
async def test_roundtrip_create_node(node: Node, session: Session):
    session.upsert(node)
    await session.commit()

    node_unpacked = await node.get(where=node.property("id").eq(node.id)).execute_one()
    assert node.equals(node_unpacked)


async def test_create_user_with_clients(session: Session):
    """Create and update a User with Clients, querying along the way."""
    # create user
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        space_ptr=NodeReference(node_type=NodeType.SPACE, id=uuid4()),
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

    # query clients with user as parent
    connection = await Client.search(
        where=Client.property("parent").eq(user),
        Parent=User.get(join=JoinType.PARENT),
    ).execute()
    clients_unpacked = connection.to_list()
    assert clients_unpacked == [client_a, client_b]


async def test_create_page_blocks_recursive(session: Session):
    """Create a Page with recursive sub-Pages and Blocks, mutate it, querying along the way."""
    # create
    root_page = Page(title=text_line("Page"))
    session.create(root_page)
    target_page_count = 4
    target_block_count = 4 * (1 + 4 * (1 + 4))
    for a in ("a", "b", "c", "d"):
        # create page
        page = Page(title=text_line(f"Page {a}"))
        root_page.add_child(page)
        # create block tree
        for i in range(4):
            root_block = Block(type=BlockType.PARAGRAPH, line=text_line(f"Block {a}/{i}"))
            page.add_child(root_block)
            for j in range(4):
                inner_block = Block(type=BlockType.PARAGRAPH, line=text_line(f"Block {a}/{i}/{j}"))
                root_block.add_child(inner_block)
                for k in range(4):
                    inner_inner_block = Block(
                        type=BlockType.PARAGRAPH,
                        line=text_line(f"Block {a}/{i}/{j}/{k}"),
                    )
                    inner_block.add_child(inner_inner_block)
                    # mutate block after creating to test edit optimization
                    inner_inner_block.node = page
        await session.commit()
    assert (
        await Page.count(where=Page.property("parent").eq(root_page)).execute_count()
        == target_page_count
    )

    # query
    for page in root_page.get_children(Page):
        # query block root count
        root_block_count = await Block.count(
            where=Block.property("parent").eq(page)
        ).execute_count()
        assert root_block_count == 4

        # query block tree down
        connection = await Page.get(
            where=Page.property("id").eq(page.id),
            Blocks=Block.search(join=join(JoinType.CHILD, recursive=True)),
        ).execute()
        page_unpacked = connection.to_one()
        block_tree_unpacked = page_unpacked.get_descendants(Block)
        assert len(block_tree_unpacked) == target_block_count

        # query block tree up
        page_block_leaves = page._graph.get_leaves(Block, of=page)
        connection = await Block.get(
            where=Block.property("id").eq(page_block_leaves[0].id),
            Blocks=Block.search(
                join=join(JoinType.PARENT, recursive=True),
                Page=Page.get(join=JoinType.PARENT),
            ),
        ).execute()
        pages_unpacked = connection.graph.get_roots(Page)
        assert len(pages_unpacked) == 1
        assert pages_unpacked[0].equals(page)


async def test_create_scene_with_heterogeneous_views(session: Session):
    """Create a Scene with heterogeneous Views, mutate it, querying along the way."""
    # create
    scene = Scene(name="Scene")
    session.create(scene)
    await session.commit()

    root_view = FrameView(name="Container")
    scene.add_child(root_view)
    # create views
    for i in range(4):
        frame_view = FrameView(name=f"View {i}")
        root_view.add_child(frame_view)
        for j in range(4):
            label_view = LabelView(name=f"Label {i}/{j}")
            frame_view.add_child(label_view)
            for k in range(4):
                text_view = TextView(name=f"Text {i}/{j}/{k}")
                label_view.add_child(text_view)
        custom_view = CustomView(
            name=f"Custom {i}",
            definition_ptr=NodeReference(node_type=NodeType.CUSTOM_VIEW_DEFINITION, id=uuid4()),
        )
        root_view.add_child(custom_view)
    await session.commit()

    # query trait (non-recursive)
    scene_tree = await FrameView.get(
        where=FrameView.property("id").eq(root_view.id),
        Views=IsView.search(join=join(JoinType.CHILD)),
    ).execute()
    scene_unpacked = scene_tree.to_one()
    view_tree_unpacked = scene_unpacked.get_descendants(IsView)
    assert len(view_tree_unpacked) == 8
