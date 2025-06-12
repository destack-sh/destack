from destack.language import Folder, Session, Tag


async def test_node_ordering(session: Session):
    """Add Nodes that are IsOrdered and check that they are ordered."""

    folder = Folder(name="MyFolder")
    session.create(folder)
    tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
    folder.add_children(*tags)
    await session.commit()
    assert folder.get_children(Tag) == tags
    assert [t.order_key for t in tags] == ["a0", "a1", "a2"]

    folder_unpacked = await Folder.get(
        where=Folder.property("id").eq(folder.id),
        Tags=Tag.search(),
    ).execute_one()
    assert folder_unpacked.get_children(Tag) == tags
    assert [t.order_key for t in tags] == ["a0", "a1", "a2"]
