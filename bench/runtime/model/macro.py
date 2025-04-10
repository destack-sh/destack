import inspect
import textwrap
from abc import ABC
from enum import StrEnum
from typing import Any, Callable, Generator, override

import regex

from bench.language import Agent, Aliasing, Message, Node, TextIn, Thread
from bench.runtime.core import Runner
from bench.runtime.model.token import Tokenizer

from .piece import CodePiece, CompoundPiece, Piece, TextPiece
from .prompt import Prompt

# ruff: noqa: F401,B018,N802,N803,F841

MACROS: list["Macro"] = []
CONSTANT_MACROS: list["ConstantMacro"] = []
FUNCTION_MACROS: list["FunctionMacro"] = []


class MacroType(StrEnum):
    CONSTANT = "constant"
    FUNCTION = "function"


class Macro(CompoundPiece):
    def __init__(self, name: str, text: str):
        self.name = name
        self.text = text


#
# Constants
#


class ConstantMacro[T: Any](Macro):
    """A constant 'macro' (really just a global)"""

    def __init__(self, name: str, text: str, type: str, get_value: Callable[[Runner], T]):
        super().__init__(name, text)
        self.type = type
        self._get_value = get_value

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield CodePiece(code=f"# {self.text}\n{self.name}: {self.type}")

    def get_value(self, runner: Runner) -> T:
        """Get the value of the Macro (not its name, the actual value for the global)."""
        return self._get_value(runner)


def constant_macro_(name: str, text: str, type: str):
    def decorator(func):
        macro = ConstantMacro(name, text, type, func)
        CONSTANT_MACROS.append(macro)
        return func

    return decorator


@constant_macro_("ME", "You (i.e. the current Agent).", "Agent")
def ME(runner: Runner) -> Agent:
    agent = runner.agent
    assert agent is not None, "Agent is not set"
    return agent


@constant_macro_("THREAD", "The current Thread.", "Thread")
def THREAD(runner: Runner) -> Thread:
    return runner.thread.thread


#
# Functions
#


class FunctionMacro(Macro):
    """A function 'macro' (really just a function)"""

    def __init__(self, name: str, text: str, signature: str, substitution: str):
        super().__init__(name, text)
        self.signature = signature
        self.substitution = substitution
        self._compiled_pattern = regex.compile(
            f"^{self.name}\\((?P<args>.*?)\\)$", regex.MULTILINE | regex.DOTALL
        )

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield TextPiece(text=f"{self.text}\n{self.name}{self.signature}")

    def expand(self, code: str) -> str:
        """Expand the Macro for any matches in code."""
        return self._compiled_pattern.sub(
            lambda m: self.substitution.replace("<args>", m.group("args")), code
        )


def function_macro_(
    name: str, text: str, signature: str, substitution: str, base: type[FunctionMacro] | None = None
):
    def decorator(func):
        macro = (base or FunctionMacro)(name, text, signature, substitution)
        FUNCTION_MACROS.append(macro)
        return func

    return decorator


@function_macro_(
    "SEND",
    """\
Create a Message in the current Thread. SEND SHOULD ONLY BE ONE PARAGRAPH.
(NO \\n\\n except in code blocks and such.)""",
    signature="(text: str, *, nodes: list[Node] | None = None, reply_to: Message | None = None, autosplit: bool = True)",
    substitution="""\
MY_LAST_MESSAGE = Message.new(<args>)
THREAD.append(MY_LAST_MESSAGE)
# --- FLUSH ---
""",
)
def SEND(text: str, *, nodes: list[Node] | None = None, reply_to: Message | None = None): ...


@function_macro_(
    "FLUSH",
    "Flush all edits.",
    signature="()",
    substitution="""\
# --- FLUSH ---
""",
)
def FLUSH(): ...
