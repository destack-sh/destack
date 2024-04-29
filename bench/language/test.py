# nocheckin: just testing dataclass_transform in pyright

from typing import Any, Type, TypeVar, dataclass_transform

_T = TypeVar("_T")


def test_property(name: str, id: int) -> Any:
    return id


# now with arguments


@dataclass_transform(kw_only_default=True, field_specifiers=(test_property,))
def register_component(name: str):
    def decorator(cls: Type[_T]) -> Type[_T]:
        return cls

    return decorator


@register_component("Node")
class Node:
    id: str = test_property("id", 1)


@register_component("User")
class User(Node):
    a: bool = test_property("a", 2)
    b: str = test_property("b", 3)


component_2 = User(id="hey", b="", a=True)
