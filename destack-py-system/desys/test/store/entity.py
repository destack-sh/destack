import pytest
from hypothesis import HealthCheck, given, settings
from pytest_async_benchmark.plugin import AsyncBenchmarkFixture
from pytest_lazy_fixtures import lf

from destack.language import (
    Client,
    ClientType,
    CustomView,
    Folder,
    FolderType,
    FrameView,
    Join,
    JoinType,
    LabelView,
    Message,
    Node,
    NodeReference,
    NodeType,
    Reaction,
    Scene,
    Session,
    Star,
    TextView,
    User,
    UserStatus,
    View,
)
from destack.test.fixtures import NODES
from destack.test.strategies import examples, nodes
from destack.utils.uuid import uuid4

ENTITY_SESSIONS = (lf("memory_session"), lf("postgres_session"))


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
@given(node=nodes)
@examples([{"node": node} for node in NODES])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
async def test_roundtrip_create_node(node: Node, session: Session):
    assert session.store is not None, f"no store in session: {session!r}"
    if node.metatype == NodeType.CUSTOM_ENTITY or node.metatype not in session.store.node_types:
        return  # ignore custom/excluded nodes

    session.upsert(node)
    await session.commit()

    node_unpacked = await node.get(where=node.property("id").eq(node.id)).execute_one()
    assert node.equals(node_unpacked)


@pytest.mark.parametrize("_session", ENTITY_SESSIONS)
async def test_create_user_with_clients(_session: Session):
    """Create and update a User with Clients, querying along the way."""
    # create user
    user = User(
        status=UserStatus.ACTIVE,
        name="Floof",
        slug="floof",
        space_ptr=NodeReference(node_type=NodeType.SPACE, id=uuid4()),
    )
    _session.create(user)
    await _session.commit()
    # update user
    user.name = "Fluff"
    user.slug = "flotothemoon"
    await _session.commit()
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
    await _session.commit()
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


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
async def test_create_folders_recursive(session: Session):
    """Create a Folder with recursive sub-Folders, mutate it, querying along the way."""
    # create
    root_folder = Folder(name="Folder", type=FolderType.HOME)
    session.create(root_folder)
    target_folder_count = 4 * (1 + 4 * (1 + 4))
    for a in ("a", "b", "c", "d"):
        # create folder
        folder = Folder(name=f"Folder {a}")
        root_folder.add_child(folder)
        # create folder tree
        for i in range(4):
            sub_folder = Folder(name=f"Folder {a}/{i}")
            folder.add_child(sub_folder)
            for j in range(4):
                inner_folder = Folder(name=f"Folder {a}/{i}/{j}")
                sub_folder.add_child(inner_folder)
                for k in range(4):
                    inner_inner_folder = Folder(name=f"Folder {a}/{i}/{j}/{k}")
                    inner_folder.add_child(inner_inner_folder)
        await session.commit()
    assert await Folder.count(where=Folder.property("parent").eq(root_folder)).execute_count() == 4

    # query
    for folder in root_folder.get_children(Folder):
        # query folder root count
        root_folder_count = await Folder.count(
            where=Folder.property("parent").eq(folder)
        ).execute_count()
        assert root_folder_count == 4

        # query folder down (parent, recursive)
        connection = await Folder.get(
            where=Folder.property("id").eq(folder.id),
            Folders=Folder.search(join=Join.of(JoinType.CHILD, recursive=True)),
        ).execute()
        folder_unpacked = connection.to_one()
        folder_tree_unpacked = folder_unpacked.get_descendants(Folder)
        assert len(folder_tree_unpacked) == target_folder_count

        # query folder up (parent, recursive)
        folder_leaves = folder._graph.get_leaves(Folder, folder)
        connection = await Folder.get(
            where=Folder.property("id").eq(folder_leaves[0].id),
            Folders=Folder.search(
                join=Join.of(JoinType.PARENT, recursive=True),
            ),
        ).execute()
        folders_unpacked = connection.graph.get_roots(Folder)
        assert len(folders_unpacked) == 1
        assert folders_unpacked[0].equals(root_folder)


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
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

    # query view (child, non-recursive)
    scene_tree = await FrameView.get(
        where=FrameView.property("id").eq(root_view.id),
        Views=View.search(join=Join.of(JoinType.CHILD)),
    ).execute()
    scene_unpacked = scene_tree.to_one()
    view_tree_unpacked = scene_unpacked.get_descendants(View)
    assert len(view_tree_unpacked) == 8

    # query view (child, recursive)
    scene_tree = await FrameView.get(
        where=FrameView.property("id").eq(root_view.id),
        Views=View.search(join=Join.of(JoinType.CHILD, recursive=True)),
    ).execute()
    scene_unpacked = scene_tree.to_one()
    view_tree_unpacked = scene_unpacked.get_descendants(View)
    assert len(view_tree_unpacked) == 4 + 4 * (1 + 4 * (1 + 4))

    # query view (parent, recursive)
    view_leaves = scene._graph.get_leaves(TextView, scene)
    scene_tree = await TextView.get(
        where=TextView.property("id").eq(view_leaves[0].id),
        Parents=View.search(join=Join.of(JoinType.PARENT, recursive=True)),
    ).execute()
    scene_unpacked = scene_tree.graph.get_roots(View)
    assert len(scene_unpacked) == 1
    assert scene_unpacked[0].equals(scene)


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
async def test_create_star(session: Session):
    """Create Stars and query them."""

    users = [
        User(
            name=f"User{i}",
            slug=f"user{i}",
            space_ptr=NodeReference(node_type=NodeType.SPACE, id=uuid4()),
        )
        for i in range(20)
    ]
    for user in users:
        session.create(user)
    await session.commit()

    folder = Folder(name="Folder")
    session.create(folder)
    await session.commit()

    for user in users:
        star = Star(parent=folder, owned_by=user)
        session.create(star)
    await session.commit()

    assert await Star.count(where=Star.property("parent").eq(folder)).execute_count() == 20


@pytest.mark.parametrize("session", ENTITY_SESSIONS, indirect=True)
async def test_create_reaction_groups(session: Session):
    """Create Reactions and query them."""

    users = [
        User(
            name=f"User{i}",
            slug=f"user{i}",
            space_ptr=NodeReference(node_type=NodeType.SPACE, id=uuid4()),
        )
        for i in range(10)
    ]
    for user in users:
        session.create(user)
    await session.commit()

    message = Message()
    session.create(message)
    await session.commit()

    reactions_content: tuple[str, ...] = ("👍", "👎", "🤷", "🤔", "🤨")
    reactions: list[Reaction] = []
    for user in users:
        for reaction in reactions_content:
            reaction = Reaction(parent=message, content=reaction, owned_by=user)
            reactions.append(reaction)
            session.create(reaction)
    await session.commit()

    # scalar by group
    message_tree = await Message.get(
        where=Message.property("id").eq(message.id),
        Reactions=Reaction.count(
            sort=[Reaction.property("created_at").asc()],
            group_by=[Reaction.property("content")],
        ),
    ).execute()
    assert message_tree.get("Reactions").to_scalar_by_group() == dict.fromkeys(
        reactions_content, 10
    )

    # node by group
    message_tree = await Message.get(
        where=Message.property("id").eq(message.id),
        Reactions=Reaction.search(group_by=[Reaction.property("content")]),
        ReactionsTotal=Reaction.count(),
    ).execute()
    reactions_by_content = {
        content: [reaction for reaction in reactions if reaction.content == content]
        for content in reactions_content
    }
    reactions_by_content_unpacked = message_tree.get("Reactions").to_list_by_group()
    for reaction_content in reactions_content:
        reactions = reactions_by_content[reaction_content]
        reactions_unpacked = reactions_by_content_unpacked[reaction_content]
        assert {str(r.id) for r in reactions} == {str(r.id) for r in reactions_unpacked}


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
@pytest.mark.benchmark
async def test_benchmark_create_reactions(session: Session, async_benchmark: AsyncBenchmarkFixture):
    """Benchmark creating reactions without parent."""

    user = User(
        name="User", slug="user", space_ptr=NodeReference(node_type=NodeType.SPACE, id=uuid4())
    )
    session.create(user)
    await session.commit()

    NUM_REACTIONS = 100

    async def _create_reactions():
        reactions = []
        for _ in range(NUM_REACTIONS):
            reaction = Reaction(content="👍", owned_by=user)
            reactions.append(reaction)
            session.create(reaction)
        await session.commit()
        return reactions

    result = await async_benchmark(_create_reactions, rounds=100, iterations=1)
    assert result["mean"] < 0.005  # <5ms
