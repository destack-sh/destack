import atexit
import signal
from typing import Callable

#
# Signals
#


_signal_handlers: dict[int, list[Callable[[], None]]] = {}


def _run_on_signal(signal: int):
    for handler in _signal_handlers.get(signal, []):
        handler()


def on_signal(signal: int, handler: Callable[[], None]):
    """Registers a handler to be called when the given signal is received."""
    if signal not in _signal_handlers:
        _signal_handlers[signal] = []
    _signal_handlers[signal].append(handler)
    return lambda: _signal_handlers[signal].remove(handler)


signal.signal(signal.SIGINT, lambda *args: _run_on_signal(signal.SIGINT))
signal.signal(signal.SIGABRT, lambda *args: _run_on_signal(signal.SIGABRT))
signal.signal(signal.SIGTERM, lambda *args: _run_on_signal(signal.SIGTERM))


#
# Exit
#

_exit_handlers: list[Callable[[], None]] = []


def _run_on_exit():
    for handler in _exit_handlers:
        handler()


def on_exit(handler: Callable[[], None]):
    """Registers a handler to be called on exit (in all cases)."""
    _exit_handlers.append(handler)
    return lambda: _exit_handlers.remove(handler)


atexit.register(_run_on_exit)
on_signal(signal.SIGINT, _run_on_exit)
on_signal(signal.SIGABRT, _run_on_exit)
on_signal(signal.SIGTERM, _run_on_exit)
