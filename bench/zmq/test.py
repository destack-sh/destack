import dataclasses
import random
import uuid
from typing import Optional, Union
from uuid import UUID

from bench.zmq.serialize import from_dict, to_dict


@dataclasses.dataclass(eq=False)
class Node:
    id: UUID
    other_id: Optional[UUID]
    reference: Union[None, tuple[str, str], UUID]
    name: str
    flags: list[bool]
    children: list["Node"] = dataclasses.field(default_factory=list)
    root: "Node" = None
    parent: "Node" = None

    @property
    def root_id(self):
        return self.root.id if self.root else self.id

    @property
    def parent_id(self):
        return self.parent.id if self.parent else None

    def __eq__(self, other):
        # compare descending only into children and contents (not references)
        return (
            self.id == other.id
            and self.other_id == other.other_id
            and self.reference == other.reference
            and self.name == other.name
            and self.flags == other.flags
            and self.root_id == other.root_id
            and self.parent_id == other.parent_id
            and len(self.children) == len(other.children)
            and all(
                self_child == other_child
                for self_child, other_child in zip(self.children, other.children)
            )
        )


def random_flags() -> [bool]:
    return [random.choice([True, False]) for _ in range(random.randint(1, 3))]


def _add_children(node: Node, count: int) -> [Node]:
    for i in range(count):
        other_id = random.choice([uuid.uuid4(), None])
        reference = random.choice([None, (node.name, str(i)), other_id])
        child = Node(
            id=uuid.uuid4(),
            other_id=other_id,
            reference=reference,
            name=f"child_{i}",
            flags=random_flags(),
        )
        child.root = node.root or node
        child.parent = node
        node.children.append(child)
    return node.children


def test_serialize_round_trip():
    # create three level
    root = Node(
        id=uuid.uuid4(), other_id=None, reference=None, name="root", flags=[True, False, True]
    )
    _add_children(root, 3)
    for child in root.children:
        _add_children(child, 2)

    serialized = to_dict(root, set())
    deserialized = from_dict(Node, serialized, {})
    assert deserialized == root
