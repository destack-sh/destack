from destack import (
    LineShape2D,
    Node,
    NodeType,
    Session,
    Shape2D,
)


def test_node_inheritance(session: Session):
    """Test the Node inheritance hierarchy."""
    assert Node.metatype == NodeType.NODE
    assert Node.__definition__.is_abstract
    assert Shape2D.metatype == NodeType.SHAPE2D
    assert Shape2D.__definition__.is_abstract
    assert LineShape2D.__definition__.base_type == Shape2D.metatype
    assert LineShape2D.__definition__.inherits == [
        NodeType.NODE,
        NodeType.ENTITY,
        NodeType.ENTITY2D,
        NodeType.SHAPE2D,
    ]
    assert len(Node.__definition__.inherited_by) == len(NodeType) - 1
