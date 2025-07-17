from itertools import chain

import pytest
from hypothesis import HealthCheck, given, settings
from pytest_async_benchmark.plugin import AsyncBenchmarkFixture
from pytest_lazy_fixtures import lf

from destack.language import (
    REGION,
    Client,
    ClientType,
    Entity,
    Folder,
    FolderType,
    FrameView,
    Join,
    JoinType,
    LabelView,
    Layer,
    Node,
    Reaction,
    Session,
    Snapshot,
    Space,
    SpaceStatus,
    Star,
    TextView,
    User,
    UserStatus,
    View,
)
from destack.test.fixtures import NODES
from destack.test.strategies import examples, nodes

ENTITY_SESSIONS = (lf("memory_session"),)

pytestmark = pytest.mark.asyncio(loop_scope="session")


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
@given(node=nodes)
@examples([{"node": node} for node in NODES])
@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
async def test_roundtrip_create_node(node: Node, session: Session, space: Space):
    assert session.store is not None, f"no store in session: {session!r}"
    if node.metatype not in session.store.node_types or not isinstance(node, Entity):
        return  # ignore custom/excluded nodes

    session.upsert(node)
    await session.commit()

    node_unpacked = await node.get(where=node.property("id").eq(node.id)).execute_one()
    assert node.equals(node_unpacked)


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
async def test_create_user_with_clients(session: Session):
    """Create and update a User with Clients, querying along the way."""
    # create user
    space = Space(name="Floof", slug="floof", status=SpaceStatus.ACTIVE, region=REGION)
    user = User(status=UserStatus.ACTIVE, name="Floof", slug="floof", space=space)
    session.create(user)
    await session.commit()
    # update user
    user.name = "Fluff"
    user.slug = "flotothemoon"
    await session.commit()
    # query user by id
    user_unpacked = await User.get(where=User.property("id").eq(user.id)).execute_one()
    assert user_unpacked.created_at == user.created_at
    assert user.equals(user_unpacked)
    # query user by slug
    user_unpacked = await User.search(where=User.property("slug").eq("flotothemoon")).execute_one()
    assert user_unpacked.equals(user)
    assert user_unpacked.name == "Fluff"
    assert user_unpacked.slug == "flotothemoon"
    assert user_unpacked.status == UserStatus.ACTIVE

    # create clients
    client_a = Client(type=ClientType.WEB, name="Client A", space=space)
    client_b = Client(type=ClientType.WEB, name="Client B", space=space)
    user.add_children(client_a, client_b)
    await session.commit()
    # query clients
    clients = await Client.search(sort=[Client.property("name").desc()]).execute_list()
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
async def test_create_folders_recursive(session: Session, space: Space):
    """Create a Folder with recursive sub-Folders, mutate it, querying along the way."""

    NUM_FOLDERS_PER_SUBTREE = 4

    # create
    root_folder = Folder(name="Folder", type=FolderType.HOME)
    session.create(root_folder)
    subtree_folder_count = NUM_FOLDERS_PER_SUBTREE * (
        1 + NUM_FOLDERS_PER_SUBTREE * (1 + NUM_FOLDERS_PER_SUBTREE)
    )
    for a in ("a", "b", "c", "d"):
        # create folder
        folder = Folder(name=f"Folder {a}")
        root_folder.add_child(folder)
        # create folder tree
        for i in range(NUM_FOLDERS_PER_SUBTREE):
            sub_folder = Folder(name=f"Folder {a}/{i}")
            folder.add_child(sub_folder)
            for j in range(NUM_FOLDERS_PER_SUBTREE):
                inner_folder = Folder(name=f"Folder {a}/{i}/{j}")
                sub_folder.add_child(inner_folder)
                for k in range(NUM_FOLDERS_PER_SUBTREE):
                    inner_inner_folder = Folder(name=f"Folder {a}/{i}/{j}/{k}")
                    inner_folder.add_child(inner_inner_folder)
        await session.commit()
    assert (
        await Folder.count(where=Folder.property("parent").eq(root_folder)).execute_count()
        == NUM_FOLDERS_PER_SUBTREE
    )

    # query
    for folder in root_folder.get_children(Folder):
        # query folder root count
        root_folder_count = await Folder.count(
            where=Folder.property("parent").eq(folder)
        ).execute_count()
        assert root_folder_count == NUM_FOLDERS_PER_SUBTREE

        # query folder down (parent, recursive)
        connection = await Folder.get(
            where=Folder.property("id").eq(folder.id),
            Folders=Folder.search(join=Join.of(JoinType.CHILD, recursive=True)),
        ).execute()
        folder_unpacked = connection.to_one()
        folder_tree_unpacked = folder_unpacked.get_descendants(Folder)
        assert len(folder_tree_unpacked) == subtree_folder_count

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

    # delete root folder (should cascade delete all folders)
    num_total_folders = await Folder.count().execute_count()
    session.delete(root_folder)
    await session.commit()
    assert await Folder.count().execute_count() == 0

    # restore root folder (should restore all folders)
    session.restore(root_folder)
    await session.commit()
    assert await Folder.count().execute_count() == num_total_folders

    # delete and restore subfolders one at a time
    for i, folder in enumerate(root_folder.get_children(Folder)):
        # delete just this subfolder (and its descendants)
        session.delete(folder)
        await session.commit()
        connection = await Folder.get(
            where=Folder.property("id").eq(folder.id),
            Folders=Folder.search(
                join=Join.of(JoinType.CHILD, recursive=True),
            ),
        ).execute()
        assert connection.to_one_or_none() is None

        assert await Folder.count().execute_count() == (
            num_total_folders - ((i + 1) * (subtree_folder_count + 1))
        )
    # restore subfolders one at a time
    for i, folder in enumerate(root_folder.get_children(Folder)):
        session.restore(folder)
        await session.commit()
        assert await Folder.count().execute_count() == 1 + ((i + 1) * (subtree_folder_count + 1))


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
async def test_create_layer_with_heterogeneous_views(session: Session, space: Space):
    """Create a Layer with heterogeneous Views, mutate it, querying along the way."""
    # create layer
    layer = Layer(name="Layer")
    session.create(layer)
    await session.commit()

    # create views
    root_view = FrameView(name="Container")
    layer.add_child(root_view)
    for i in range(4):
        frame_view = FrameView(name=f"View {i}")
        root_view.add_child(frame_view)
        for j in range(4):
            label_view = LabelView(name=f"Label {i}/{j}")
            frame_view.add_child(label_view)
            for k in range(4):
                text_view = TextView(name=f"Text {i}/{j}/{k}")
                label_view.add_child(text_view)
        label_view = LabelView(name=f"Label {i}")
        root_view.add_child(label_view)
    await session.commit()

    # query view (child, non-recursive)
    layer_tree = await FrameView.get(
        where=FrameView.property("id").eq(root_view.id),
        Views=View.search(join=Join.of(JoinType.CHILD)),
    ).execute()
    layer_unpacked = layer_tree.to_one()
    view_tree_unpacked = layer_unpacked.get_descendants(View)
    assert len(view_tree_unpacked) == 8

    # query view (child, recursive)
    layer_tree = await FrameView.get(
        where=FrameView.property("id").eq(root_view.id),
        Views=View.search(join=Join.of(JoinType.CHILD, recursive=True)),
    ).execute()
    layer_unpacked = layer_tree.to_one()
    view_tree_unpacked = layer_unpacked.get_descendants(View)
    assert len(view_tree_unpacked) == 4 + 4 * (1 + 4 * (1 + 4))

    # query view (parent, recursive)
    view_leaves = layer.get_leaves(TextView)
    layer_tree = await TextView.get(
        where=TextView.property("id").eq(view_leaves[0].id),
        Parents=View.search(
            join=Join.of(JoinType.PARENT, recursive=True),
            Layers=Layer.search(
                join=Join.of(JoinType.PARENT),
            ),
        ),
    ).execute()
    layer_unpacked = layer_tree.graph.get_roots(Layer)
    assert len(layer_unpacked) == 1
    assert layer_unpacked[0].equals(layer)


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
async def test_create_star(session: Session, space: Space):
    """Create Stars and query them."""

    users = [User(name=f"User{i}", slug=f"user{i}") for i in range(20)]
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
async def test_create_reaction_groups(session: Session, space: Space):
    """Create Reactions and query them."""

    users = [User(name=f"User{i}", slug=f"user{i}") for i in range(10)]
    for user in users:
        session.create(user)
    await session.commit()

    folder = Folder(name="Folder")
    session.create(folder)
    await session.commit()

    reactions_content: tuple[str, ...] = ("👍", "👎", "🤷", "🤔", "🤨")
    reactions: list[Reaction] = []
    for user in users:
        for reaction in reactions_content:
            reaction = Reaction(parent=folder, content=reaction, owned_by=user)
            reactions.append(reaction)
            session.create(reaction)
    await session.commit()

    # scalar by group
    folder_tree = await Folder.get(
        where=Folder.property("id").eq(folder.id),
        Reactions=Reaction.count(
            sort=[Reaction.property("created_at").asc()],
            group_by=[Reaction.property("content")],
        ),
    ).execute()
    assert folder_tree.get("Reactions").to_scalar_by_group() == dict.fromkeys(reactions_content, 10)

    # node by group
    folder_tree = await Folder.get(
        where=Folder.property("id").eq(folder.id),
        Reactions=Reaction.search(group_by=[Reaction.property("content")]),
        ReactionsTotal=Reaction.count(),
    ).execute()
    reactions_by_content = {
        content: [reaction for reaction in reactions if reaction.content == content]
        for content in reactions_content
    }
    reactions_by_content_unpacked = folder_tree.get("Reactions").to_list_by_group()
    for reaction_content in reactions_content:
        reactions = reactions_by_content[reaction_content]
        reactions_unpacked = reactions_by_content_unpacked[reaction_content]
        assert {str(r.id) for r in reactions} == {str(r.id) for r in reactions_unpacked}


@pytest.mark.parametrize("session", ENTITY_SESSIONS)
@pytest.mark.benchmark
async def test_benchmark_create_reactions(session: Session, async_benchmark: AsyncBenchmarkFixture):
    """Benchmark creating reactions without parent."""

    space = Space(name="Test", slug="test", status=SpaceStatus.ACTIVE, region=REGION)
    user = User(name="User", slug="user", space=space)
    session.create(user)
    await session.commit()

    NUM_REACTIONS = 100

    async def _create_reactions():
        reactions = []
        for _ in range(NUM_REACTIONS):
            reaction = Reaction(content="👍", owned_by=user, space=space)
            reactions.append(reaction)
            session.create(reaction)
        await session.commit()
        return reactions

    result = await async_benchmark(_create_reactions, rounds=100, iterations=1)
    assert result["mean"] < 0.01  # <10ms


@pytest.mark.parametrize("session", ENTITY_SESSIONS, indirect=True)
async def test_move_views(session: Session, space: Space):
    """Move Views around."""
    layer = Layer(name="Layer", space=space)
    session.create(layer)
    await session.commit()

    # create views
    root_view = FrameView(name="Root")
    frame_views: list[FrameView] = []
    layer.add_child(root_view)
    for i in range(4):
        frame_view = FrameView(name=f"View {i}")
        frame_views.append(frame_view)
        root_view.add_child(frame_view)
        for j in range(4):
            label_view = LabelView(name=f"Label {i}/{j}")
            frame_view.add_child(label_view)
    await session.commit()

    # detach views
    for frame_view in frame_views:
        frame_view.detach()
        assert frame_view.parent_ptr is None
    await session.commit()

    # reattach views
    for frame_view in frame_views:
        layer.add_child(frame_view)
        assert frame_view.parent_ptr == layer.to_ref()
    await session.commit()

    # detach all the leaf label views
    label_views: list[LabelView] = []
    for frame_view in frame_views:
        for label_view in frame_view.get_children(LabelView):
            label_view.detach()
            label_views.append(label_view)
            assert label_view.parent_ptr is None
    await session.commit()

    # move all views to be directly parented by layer
    for view in chain(frame_views, label_views):
        view.move_to(layer)
        assert view.parent_ptr == layer.to_ref()
    await session.commit()

    layer_children = layer.get_children(View)
    assert len(layer_children) == 1 + 4 * (4 + 1)


# @pytest.mark.parametrize("session", ENTITY_SESSIONS)
@pytest.mark.skip(reason=":Incomplete")
async def test_edit_partial_node_in_snapshot(session: Session):
    """Create a Snapshot and query it."""

    # nocheckin: support Entity branching & variants

    space = Space(name="Test", slug="test", status=SpaceStatus.ACTIVE, region=REGION)
    user = User(name="Alice", slug="alice", space=space)
    session.create(user)
    await session.commit()

    snapshot = Snapshot(name="My Little Snapshot", space=space)
    session.create(snapshot)
    await session.commit()

    # edit in snapshot
    with snapshot:
        snapshot_user = user.into(snapshot)
        assert snapshot_user.id == user.id
        assert snapshot_user.snapshot == snapshot
        assert snapshot_user.preceded_by is user
        assert snapshot_user

        snapshot_user.name = "Bob"
        assert snapshot_user.snapshot == snapshot
        assert snapshot_user.name == "Bob"

    # should still be the same in original user
    assert user.name == "Alice"
    assert user.snapshot is None

    # change original user
    user.name = "Charlie"
    user.slug = "charlie"
    assert user.slug == "charlie"

    # should also be updated in snapshot
    with snapshot:
        assert snapshot_user.name == "Charlie"
        # except for override
        assert snapshot_user.slug == "bob"


# @pytest.mark.parametrize("session", ENTITY_SESSIONS)
@pytest.mark.skip(reason=":Incomplete")
async def test_edit_partial_graph_in_snapshot(session: Session, space: Space):
    """Create a Snapshot and query it."""

    user = User(name="Alice", slug="alice", space=space)
    session.create(user)
    await session.commit()

    # ... also support 'delete overrides' and such (delete Nodes in override)
