# nocheckin: just testing dataclass_transform in pyright

from typing import Any, Type, TypeVar, dataclass_transform

_T = TypeVar("_T")


def test_property(name: str, id: int) -> Any:
    return id


@dataclass_transform(kw_only_default=True, field_specifiers=(test_property,))
def simple_component(cls: Type[_T]) -> Type[_T]:
    return cls


@simple_component
class MySimpleComponent:
    a: bool = test_property("a", 1)
    b: str = test_property("b", 2)


component_1 = MySimpleComponent(b="", a=True)


# now with arguments


@dataclass_transform(kw_only_default=True, field_specifiers=(test_property,))
def register_component(name: str):
    def decorator(cls: Type[_T]) -> Type[_T]:
        return cls

    return decorator


@register_component("MyComponent")
class MyRealComponent:
    a: bool = test_property("a", 1)
    b: str = test_property("b", 2)


component_2 = MyRealComponent(b="", a=True)
