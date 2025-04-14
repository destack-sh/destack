import abc
import asyncio
import functools
from enum import StrEnum
from typing import TYPE_CHECKING, Any, Callable, Generator, cast, final, override

from bench.language import (
    Action,
    Agent,
    Block,
    CursorType,
    FileIn,
    Message,
    Node,
    Page,
    ProcessStatus,
    TextIn,
    Thread,
    coerce_custom_object_scalar,
    upload_file,
)
from bench.runtime.core import create_run
from bench.runtime.model import CodePiece, CompoundPiece, Piece, Prompt, Tokenizer

if TYPE_CHECKING:
    from bench.runtime import AgentRunner

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
    role = "developer"

    def __init__(self, name: str, text: str):
        self.name = name
        self.text = text

    @abc.abstractmethod
    def bind(self, runner: "AgentRunner") -> Any:
        """Bind the Macro to a Runner."""


#
# Constants
#


class ConstantMacro[T: Any](Macro):
    """A constant 'macro' (really just a global)"""

    def __init__(self, name: str, text: str, type: str, func: Callable[["AgentRunner"], T]):
        super().__init__(name, text)
        self.type = type
        self._get_value = func

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield CodePiece(code=f"# {self.text}\n{self.name}: {self.type}")

    @final
    def bind(self, runner: "AgentRunner") -> T:
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
def ME(runner: "AgentRunner") -> Agent:
    agent = runner.agent
    assert agent is not None, f"no agent in {runner!r}"
    return agent


@constant_macro_("THREAD", "The current Thread.", "Thread")
def THREAD(runner: "AgentRunner") -> Thread:
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
        func: Callable[["AgentRunner"], Callable[..., None]],
        is_terminal: bool = False,
        is_edit: bool = False,
    ):
        self.name = name
        self.text = text
        self.signature = signature
        self.func = func
        self.is_terminal = is_terminal
        self.is_edit = is_edit

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        code_parts = [f"# {self.name}"]
        text = "\n".join([f"# {line}" for line in self.text.split("\n")])
        if self.is_terminal:
            text += f"\n# You MAY only USE {self.name} at the end of your response ONCE."
        if self.is_edit:
            text += "\n# You MUST have the right access to edit. Refuse otherwise."
        code_parts.append(f"{self.name}: {self.signature}")
        yield CodePiece(code="\n".join(code_parts))

    @final
    def bind(self, runner: "AgentRunner") -> Callable:
        return functools.partial(self.func, runner=runner)  # type: ignore


def function_macro_(
    name: str, text: str, signature: str, is_terminal: bool = False, is_edit: bool = False
):
    def decorator(func):
        macro = FunctionMacro(
            name=name,
            text=text,
            signature=signature,
            func=func,
            is_terminal=is_terminal,
            is_edit=is_edit,
        )
        FUNCTION_MACROS.append(macro)
        _add_macro(macro)
        return func

    return decorator


# just for type-checking, function macros are never called directly (only after binding)
_INJECTED_RUNNER = cast("AgentRunner", None)


@function_macro_("FLUSH", "Flush all pending edits.", signature="() -> None")
def FLUSH(*, runner: "AgentRunner" = _INJECTED_RUNNER):
    runner.session.stage()


@function_macro_(
    "SEND",
    """\
Create a Message in the current Thread.
SHOULD be just one 'paragraph' (use multiple SENDs if needed).
Returns the Message.
""",
    signature="(str, *, nodes: list[Node] | None = None, reply_to: Message | None = None) -> Message",
)
def SEND(
    text: TextIn,
    *,
    nodes: list[Node] | None = None,
    reply_to: Message | None = None,
    runner: "AgentRunner" = _INJECTED_RUNNER,
):
    thread = runner.thread.thread
    message = Message.new(text=text, owned_by=runner.agent, nodes=nodes, reply_to=reply_to)
    thread.append(message)
    if (cursor := runner.node.get_cursor(type=CursorType.THREAD)) is not None:
        cursor.seen_at = message.created_at
    runner.session.stage()
    return message


@function_macro_("UPLOAD", "Upload a File.", signature="(file_in: bytes | Path, name: str) -> File")
def UPLOAD(file_in: FileIn, name: str, runner: "AgentRunner" = _INJECTED_RUNNER):
    return upload_file(file_in, name, parent=runner.closest_tracked_run)


@function_macro_(
    "WAIT",
    """\
Wait for some time before thinking again. 
If you expect something to happen soon (<1min), just wait for a bit.
""",
    signature="(seconds: float = 2) -> None",
    is_terminal=True,
)
def WAIT(
    seconds: float = 3,
    runner: "AgentRunner" = _INJECTED_RUNNER,
):
    assert seconds < 60, f"WAIT must be less than 60 seconds: {seconds}"
    runner.wait(seconds)


@function_macro_(
    "CALL",
    """\
Call an Action as a tool.
""",
    signature="(action: Action, **inputs) -> None",
    is_terminal=True,
)
def CALL(action: Action, runner: "AgentRunner" = _INJECTED_RUNNER, **inputs):
    agent_run = runner.tracked_run
    assert agent_run is not None, f"no agent run in {runner!r}"
    input_type = action.input_type
    assert input_type is not None, f"no input type for {action!r}"
    inputs = coerce_custom_object_scalar(inputs, input_type)
    run, _ = create_run(
        action,
        parent=agent_run,
        thread=runner.thread.thread,
        status=ProcessStatus.QUEUED,
        agent=runner.agent,
        inputs=inputs,
    )
    runner.call(run)


@function_macro_(
    "REPLACE_TEXT",
    """\
Replace text on a current Page.
After and before are *exclusive* (if both are unspecified, the whole Page is replaced!).
Returns the *new* Blocks in order.
""",
    signature="(text: str, page: Page, after: Block | None = None, before: Block | None = None) -> tuple[Block, ...]",
    is_edit=True,
)
def REPLACE_TEXT(
    text: str,
    page: Page,
    after: Block | None = None,
    before: Block | None = None,
    runner: "AgentRunner" = _INJECTED_RUNNER,
) -> tuple[Block, ...]:
    _ = page.remove_text(after, before)
    blocks = page.add_text(text, after, before)
    runner.session.stage()
    return tuple(blocks)
