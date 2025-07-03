from destack.language import (
    REGION,
    Cursor,
    EventCursor,
    Folder,
    Node,
    NodeType,
    Session,
    Space,
    SpaceStatus,
    Tag,
)


def test_node_inheritance(session: Session):
    """Test the Node inheritance hierarchy."""
    assert Node.metatype == NodeType.NODE
    assert Node.__is_abstract__
    assert Cursor.__is_abstract__
    assert EventCursor.__base_type__ == Cursor.metatype
    assert Cursor.__inherits__ == (NodeType.NODE, NodeType.ENTITY)
    assert EventCursor.__inherits__ == (NodeType.NODE, NodeType.ENTITY, NodeType.CURSOR)
    assert Cursor.__extended_by__ == (
        NodeType.EVENT_CURSOR,
        NodeType.SCREEN_CURSOR,
        NodeType.THREAD_CURSOR,
    )
    assert Cursor.__inherited_by__ == (
        NodeType.EVENT_CURSOR,
        NodeType.SCREEN_CURSOR,
        NodeType.THREAD_CURSOR,
    )
    assert len(Node.__inherited_by__) == len(NodeType) - 1


def test_node_space_ptr(session: Session):
    """Add Nodes that are Spatial and check that they have the same space_ptr."""
    space = Space(name="MySpace", slug="my-space", status=SpaceStatus.RUNNING, region=REGION)
    session.create(space)
    folder = Folder(name="MyFolder")
    space.add_child(folder)
    assert folder.space_ptr and folder.space_ptr.id == space.id
    tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
    folder.add_children(*tags)
    for tag in tags:
        assert tag.space_ptr and tag.space_ptr.id == space.id


def test_node_ordering(session: Session):
    """Add Nodes that are IsOrdered and check that they are ordered."""

    folder = Folder(name="MyFolder")
    session.create(folder)
    tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
    folder.add_children(*tags)
    assert folder.get_children(Tag) == tags
    assert [t.order_key for t in tags] == ["a0", "a1", "a2"]
