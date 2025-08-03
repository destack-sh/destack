from destack import VERSION, NodeType, Session, Space, Universe


def test_universe_constants(session: Session, space: Space):
    assert Universe.VERSION == VERSION
    assert len(Universe.NODES) == len(NodeType)
