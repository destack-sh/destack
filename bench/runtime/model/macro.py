import abc
import functools
import inspect
import textwrap
from abc import ABC
from enum import StrEnum
from typing import Any, Callable, Generator, Sequence, cast, final, override

import regex

from bench.language import Agent, Aliasing, FileIn, Message, Node, TextIn, Thread, upload_file
from bench.runtime.core import Runner

from .piece import CodePiece, CompoundPiece, Piece, TextPiece
from .prompt import Prompt
from .token import Tokenizer

# ruff: noqa: F401,B018,N802,N803,F841

MACROS: list["Macro"] = []
MACROS_BY_NAME: dict[str, "Macro"] = {}
CONSTANT_MACROS: list["ConstantMacro"] = []
FUNCTION_MACROS: list["FunctionMacro"] = []


def _add_macro(macro: "Macro"):
    MACROS.append(macro)
    if macro.name in MACROS_BY_NAME:
        raise ValueError(
            f"macro with name {macro.name!r} already exists: {MACROS_BY_NAME[macro.name]!r}"
        )
    MACROS_BY_NAME[macro.name] = macro


class MacroType(StrEnum):
    CONSTANT = "constant"
    FUNCTION = "function"


class Macro(CompoundPiece):
    def __init__(self, name: str, text: str):
        self.name = name
        self.text = text

    @abc.abstractmethod
    def bind(self, runner: Runner) -> Any:
        """Bind the Macro to a Runner."""


#
# Constants
#


class ConstantMacro[T: Any](Macro):
    """A constant 'macro' (really just a global)"""

    def __init__(self, name: str, text: str, type: str, func: Callable[[Runner], T]):
        super().__init__(name, text)
        self.type = type
        self._get_value = func

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield CodePiece(code=f"# {self.text}\n{self.name}: {self.type}")

    @final
    def bind(self, runner: Runner) -> T:
        """Get the value of the Macro (not its name, the actual value for the global)."""
        return self._get_value(runner)


def constant_macro_(name: str, text: str, type: str):
    def decorator(func):
        macro = ConstantMacro(name=name, text=text, type=type, func=func)
        CONSTANT_MACROS.append(macro)
        _add_macro(macro)
        return func

    return decorator


@constant_macro_("ME", "You (i.e. the current Agent).", "Agent")
def ME(runner: Runner) -> Agent:
    agent = runner.agent
    assert agent is not None, f"no agent in {runner!r}"
    return agent


@constant_macro_("THREAD", "The current Thread.", "Thread")
def THREAD(runner: Runner) -> Thread:
    thread = runner.thread.thread
    assert thread is not None, f"no thread in {runner!r}"
    return thread


#
# Functions
#


class FunctionMacro(Macro):
    """A function 'macro' (really just a function)"""

    def __init__(
        self,
        name: str,
        text: str,
        signature: str,
        func: Callable[[Runner], Callable[..., None]],
    ):
        self.name = name
        self.text = text
        self.signature = signature
        self.func = func

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield TextPiece(text=f"{self.text}\n{self.name}{self.signature}")

    @final
    def bind(self, runner: Runner) -> Callable:
        return functools.partial(self.func, runner=runner)  # type: ignore


def function_macro_(name: str, text: str, signature: str):
    def decorator(func):
        macro = FunctionMacro(name=name, text=text, signature=signature, func=func)
        FUNCTION_MACROS.append(macro)
        _add_macro(macro)
        return func

    return decorator


_INJECTED_RUNNER = cast(Runner, None)


@function_macro_("FLUSH", "Flush all pending edits.", signature="() -> None")
def FLUSH(*, runner: Runner = _INJECTED_RUNNER):
    runner.session.stage()


@function_macro_(
    "SEND",
    """\
Create a Message in the current Thread.
Text SHOULD almost always be just one 'paragraph'. 
Use multiple SENDs as needed, interleaved with other actions.
""",
    signature="(str, *, nodes: list[Node] | None = None, reply_to: Message | None = None) -> Message",
)
def SEND(
    text: TextIn,
    *,
    nodes: list[Node] | None = None,
    reply_to: Message | None = None,
    runner: Runner = _INJECTED_RUNNER,
):
    thread = runner.thread.thread
    message = Message.new(text=text, nodes=nodes, reply_to=reply_to)
    thread.append(message)
    runner.session.stage()
    return message


@function_macro_("UPLOAD", "Upload a file to the current Run.", signature="(str) -> File")
def UPLOAD(file_in: FileIn, name: str, runner: Runner = _INJECTED_RUNNER):
    return upload_file(file_in, name, parent=runner.closest_tracked_run)
