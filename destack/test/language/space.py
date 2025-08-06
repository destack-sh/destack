from destack import (
    VERSION,
    EnumType,
    HandleType,
    NodeType,
    Session,
    Space,
    StructType,
    Universe,
)


def test_universe_constants(session: Session, space: Space):
    assert Universe.VERSION == VERSION
    assert Universe.SCHEMA.version == VERSION
    assert len(Universe.SCHEMA.nodes) == len(NodeType)
    assert len(Universe.SCHEMA.structs) == len(StructType)
    assert len(Universe.SCHEMA.handles) == len(HandleType)
    assert len(Universe.SCHEMA.enums) == len(EnumType)
