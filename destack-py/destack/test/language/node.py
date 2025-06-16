from destack.language import REGION, Folder, Session, Space, SpaceStatus, Tag


async def test_node_space_ptr(session: Session):
    """Add Nodes that are Spatial and check that they have the same space_ptr."""
    space = Space(name="MySpace", slug="my-space", status=SpaceStatus.RUNNING, region=REGION)
    session.create(space)
    folder = Folder(name="MyFolder")
    space.add_child(folder)
    assert folder.space_id == space.id
    tags = [Tag(name="A"), Tag(name="B"), Tag(name="C")]
    folder.add_children(*tags)
    for tag in tags:
        assert tag.space_id == space.id
    await session.commit()

    folders_unpacked = await Folder.search(where=Folder.property("space").eq(space)).execute_list()
    assert folders_unpacked == [folder]


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
