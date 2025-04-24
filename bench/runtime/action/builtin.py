from typing import Callable
from uuid import UUID

from bench.language import Action
from bench.runtime.core import NotSupportedError

from .web import ExaWeb

_registered = False
_builtin_action_runners_by_id: dict[UUID, Callable] = {}


def _register_builtin_action(action: Action, runner: Callable):
    """Registers a function that implements a builtin Action."""
    if (existing := _builtin_action_runners_by_id.get(action.id)) is not None:
        raise ValueError(f"builtin {action.id} already registered: {existing!r} != {runner!r}")
    _builtin_action_runners_by_id[action.id] = runner


def _register_builtins():
    """Registers all builtin Actions."""
    global _registered
    from bench.builtin import WebKit

    for kit, impl_cls in ((WebKit, ExaWeb),):
        impl = impl_cls()
        for action in kit.actions:
            assert action.name is not None, f"{action!r} has no name"
            method_name = action.name.replace(" ", "_")
            method = getattr(impl, method_name, None)
            assert method is not None, f"{impl.__class__} has no method {method_name!r}"
            _register_builtin_action(action, method)

    _registered = True


def get_builtin_action_runner(action: Action) -> Callable:
    """Get the runner for the given Action."""
    if not _registered:
        _register_builtins()
    action_runner = _builtin_action_runners_by_id[action.id]
    if action_runner is None:
        raise NotSupportedError(f"{action} is not available")
    return action_runner
