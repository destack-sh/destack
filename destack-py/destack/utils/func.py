import importlib
import sys
import types
from collections.abc import Iterator


def reload_module(module: types.ModuleType):
    """Reloads a module and all its submodules."""
    visited = set()

    def _reload(mod: types.ModuleType):
        if mod.__name__ in visited:
            return
        visited.add(mod.__name__)

        # reload submodules
        if hasattr(mod, "__path__"):
            for submodule_info in pkgutil.iter_modules(mod.__path__, prefix=f"{mod.__name__}."):
                submodule_name = submodule_info.name
                if submodule_name in sys.modules:
                    submodule = sys.modules[submodule_name]
                    _reload(submodule)

        # then reload parent
        importlib.reload(mod)

    import pkgutil

    _reload(module)


def get_superclasses[T](cls: type[T], seen: set[type[T]] | None = None) -> Iterator[type[T]]:
    """Gets all superclasses of a class recursively (BFS)."""
    seen = seen if seen is not None else set()
    queue = [cls]
    while queue:
        current = queue.pop(0)
        if current not in seen:
            seen.add(current)
            yield current
            queue.extend(current.__bases__)


def get_subclasses[T](cls: type[T], seen: set[type[T]] | None = None) -> Iterator[type[T]]:
    """Gets all subclasses of a class recursively (BFS)."""
    seen = seen if seen is not None else set()
    queue = [cls]
    while queue:
        current = queue.pop(0)
        if current not in seen:
            seen.add(current)
            yield current
            queue.extend(current.__subclasses__())
