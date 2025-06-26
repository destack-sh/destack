from dataclasses import dataclass
from typing import Any, Callable


@dataclass(slots=True, frozen=True)
class ConstantDeclaration:
    """Declaration of a builtin Constant (may be deferred)."""

    name: str
    module: str
    description: str | None
    value: Any | None
    getter: Callable[[], Any] | None


CONSTANT_DECLARATIONS: dict[str, ConstantDeclaration] = {}


def register_constant(name: str, value: Any | Callable[[], Any], description: str | None = None):
    """Register a constant value for the current module (may be deferred)."""
    # get caller module
    import inspect

    frame = inspect.currentframe()
    assert frame is not None
    module = inspect.getmodule(frame.f_back)
    assert module is not None

    declaration = ConstantDeclaration(
        name=name,
        module=module.__name__,
        description=description,
        value=value if not callable(value) else None,
        getter=value if callable(value) else None,
    )
    if existing_declaration := CONSTANT_DECLARATIONS.get(name):
        raise ValueError(f"constant {name} already registered: {existing_declaration!r}")
    CONSTANT_DECLARATIONS[name] = declaration
    return declaration
