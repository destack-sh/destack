from destack import (
    Folder,
    Materialization,
    NodeType,
    Session,
    Space,
    Tag,
)


def test_entity_ordering(session: Session, space: Space):
    """Add Entities and check that they are ordered."""

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


def test_entity_instantiation(session: Session, space: Space):
    """Test the Entity instantiation."""

    folder = Folder(name="MyFolder")
    session.create(folder)

    tag_a = Tag(name="A")
    tag_b = Tag(name="B")
    tag_c = Tag(name="C")
    folder.add_children(tag_a, tag_b, tag_c)

    folder_instance = folder.instantiate()
    assert folder_instance.metatype == NodeType.FOLDER
    assert folder_instance.materialization == Materialization.PARTIAL
    assert folder_instance.definition_ptr == folder.to_ref()

    tags_instance = folder_instance.get_children(Tag)
    for tag_instance, tag in zip(tags_instance, folder.get_children(Tag)):
        assert tag_instance.id == tag.id
        assert tag_instance.metatype == NodeType.TAG
        assert tag_instance.materialization == Materialization.PARTIAL
        assert tag_instance.definition_ptr == tag.to_ref()
