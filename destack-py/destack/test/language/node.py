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
        assert folder.parent_ptr and folder.parent_ptr.id == space.id
        assert folder.space_ptr and folder.space_ptr.id == space.id
        tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
        folder.add_children(*tags)
        for tag in tags:
            assert tag.space_ptr and tag.space_ptr.id == space.id
