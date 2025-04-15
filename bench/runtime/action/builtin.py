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

    web = ExaWeb()
    _register_builtin_action(WebKit.actions.Search, web.Search)

    _registered = True


def get_builtin_action_runner(action: Action) -> Callable:
    """Get the runner for the given Action."""
    if not _registered:
        _register_builtins()
    action_runner = _builtin_action_runners_by_id[action.id]
    if action_runner is None:
        raise NotSupportedError(f"{action} is not available")
    return action_runner
