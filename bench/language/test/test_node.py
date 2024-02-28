from itertools import chain

import pytest

from bench.language import Property
from bench.language.const import InterpStatus, NodeType
from bench.language.setup import NODE_CLASSES, STRUCT_CLASSES
from bench.language.test.fabricator import Fabricator


def test_struct_regular_properties_are_available():
    for cls in chain(STRUCT_CLASSES, NODE_CLASSES):
        for prop in cls.__properties__.values():
            if prop.is_introspectable:
                attr = getattr(cls, prop.name)
                assert isinstance(attr, Property), f"{prop!r}->{attr!r} is not a Property"


def test_get_set_non_existing_property(fabricator: "Fabricator"):
    client = fabricator.fabricate(NodeType.CLIENT)
    client._status = InterpStatus.TRACKED
    with pytest.raises(AttributeError):
        client.wadabadaboo = "wadabadaboo"
    with pytest.raises(AttributeError):
        _ = client.wadabadaboo
