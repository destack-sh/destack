from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from destack.language.core.builtin.entity import Entity


class EntityInstanceProxy:
    __slots__ = ("target",)

    def __init__(self, target: "Entity | EntityInstanceProxy"):
        self.target = target

    def __getattr__(self, name: str):
        return getattr(self.target, name)

    def __setattr__(self, name: str, value: Any):
        setattr(self.target, name, value)
