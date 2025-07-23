from destack.language import (
    Folder,
    LineShape2D,
    Node,
    NodeType,
    Session,
    Shape2D,
    Space,
    Tag,
)


def test_node_inheritance(session: Session):
    """Test the Node inheritance hierarchy."""
    assert Node.metatype == NodeType.NODE
    assert Node.__is_abstract__
    assert Shape2D.metatype == NodeType.SHAPE2D
    assert Shape2D.__is_abstract__
    assert LineShape2D.__base_type__ == Shape2D.metatype
    assert LineShape2D.__inherits__ == (
        NodeType.NODE,
        NodeType.ENTITY,
        NodeType.ENTITY2D,
        NodeType.SHAPE2D,
    )
    assert len(Node.__inherited_by__) == len(NodeType) - 1


def test_node_space_ptr(session: Session, space: Space):
    """Add Nodes that are Spatial and check that they have the same space_ptr."""
    with space.active():
        folder = Folder(name="MyFolder")
        space.add_child(folder)
        assert folder.space_ptr and folder.space_ptr.id == space.id
        tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
        folder.add_children(*tags)
        for tag in tags:
            assert tag.space_ptr and tag.space_ptr.id == space.id


def test_node_ordering(session: Session, space: Space):
    """Add Nodes that are IsOrdered and check that they are ordered."""

    folder = Folder(name="MyFolder")
    session.create(folder)

    # create tags
    tag_a = Tag(name="A")
    tag_b = Tag(name="B")
    tag_c = Tag(name="C")
    folder.add_children(tag_a, tag_b, tag_c)
    assert folder.get_children(Tag) == [tag_a, tag_b, tag_c]
    assert [t.order_key for t in folder.get_children(Tag)] == ["a0", "a1", "a2"]

    # move tags
    tag_b1 = Tag(name="B1")
    tag_b1.move_to(folder, after=tag_b, before=tag_c)
    assert folder.get_children(Tag) == [tag_a, tag_b, tag_b1, tag_c]
    assert [t.order_key for t in folder.get_children(Tag)] == ["a0", "a1", "a1P", "a2"]

    # insert tags
    tag_b2 = Tag(name="B2")
    tag_b2.move_to(folder, after=tag_b1, before=tag_c)
    assert folder.get_children(Tag) == [tag_a, tag_b, tag_b1, tag_b2, tag_c]
    assert [t.order_key for t in folder.get_children(Tag)] == ["a0", "a1", "a1P", "a1h", "a2"]
