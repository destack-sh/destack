from destack.core.utils.uuid import UUID, uuid7


def test_generate_uuid7() -> None:
    for _ in range(10):
        uuid = uuid7()
        assert isinstance(uuid, UUID)
