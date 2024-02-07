import pytest

from bench.language.const import NodeStatus, NodeType
from bench.language.test.fabricator import Fabricator


def test_get_set_non_existing_property(fabricator: "Fabricator"):
    client = fabricator.fabricate(NodeType.CLIENT)
    client._status = NodeStatus.TRACKED
    with pytest.raises(AttributeError):
        client.wadabadaboo = "wadabadaboo"
    with pytest.raises(AttributeError):
        _ = client.wadabadaboo
