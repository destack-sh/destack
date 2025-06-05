import abc
import functools
from collections.abc import Generator, Sequence
from datetime import date, datetime
from enum import StrEnum
from typing import TYPE_CHECKING, Any, Callable, cast, final, override

from bench.language import (
    Action,
    Agent,
    Block,
    Claim,
    ClaimType,
    CursorType,
    EnvironmentType,
    File,
    Interruption,
    Message,
    Node,
    Package,
    Page,
    ProcessStatus,
    Run,
    Span,
    TextIn,
    TextLineIn,
    Thread,
    coerce_custom_object_scalar,
    text_line,
    to_icon,
)
from bench.runtime.core import create_run
from bench.runtime.model import CodePiece, CompoundPiece, Piece, Prompt, Tokenizer

if TYPE_CHECKING:
    from bench.runtime import AgentRunner

# ruff: noqa: N802

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
    def compile(self, prompt: Prompt, tokenizer: Tokenizer) -> Generator[Piece, None, None]:
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


@constant_macro_("TODAY", "The current date (without time).", "date")
def TODAY(runner: "AgentRunner") -> date:
    return runner.runtime.oracle.utc().date()


@constant_macro_("NOW", "The current date and time.", "datetime")
def NOW(runner: "AgentRunner") -> datetime:
    return runner.runtime.oracle.utc()


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
    def compile(self, prompt: Prompt, tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        code_parts = [f"# {self.name}"]
        text = "\n".join([f"# {line}" for line in self.text.split("\n")])
        if self.is_terminal:
            text += f"\n# TERMINAL: You MAY only PUT {self.name} at the end of your turn."
        if self.is_edit:
            text += "\n# EDIT: You MUST have the appropriate access to do this. Refuse otherwise."
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


@function_macro_(
    "SEND",
    """\
Create a Message in the current Thread.
One Message SHOULD be just one 'paragraph' (use multiple SENDs as needed).
`nodes` should only contain Nodes *new* to this chat.
""",
    signature="(str | None, *, nodes: Sequence[Node] | None = None, reply_to: Message | None = None) -> Message",
)
def SEND(
    text: TextIn | None,
    *,
    nodes: Sequence[Node] | None = None,
    reply_to: Message | None = None,
    runner: "AgentRunner" = _INJECTED_RUNNER,
):
    thread = runner.thread
    # nodes
    new_nodes: list[Node] = []
    prev_nodes: set[Node] = {n for m in thread.messages for n in m.nodes}
    for node in nodes or ():
        # ensure nodes are relevant and nodes
        if not isinstance(node, (Run, Span, Interruption, Message)) and node not in prev_nodes:
            new_nodes.append(node)
            prev_nodes.add(node)
    # message
    message = Message.new(
        text=text,
        owned_by=runner.agent,
        nodes=new_nodes,
        reply_to=reply_to,
        model_developer=runner.model_settings.model_developer,
        model_provider=runner.model_settings.model_provider,
        model_id=runner.model_settings.model_id,
        model_name=runner.model_settings.model_name,
    )
    for node in new_nodes:
        if node.parent_ptr is None:
            message.add_child(node)
    runner.thread.thread.add_child(message)
    runner.thread.add_optimistic_message(message)
    # cursor
    if (
        cursor := runner.thread.thread.get_cursor(type=CursorType.THREAD, owned_by=runner.agent)
    ) is not None:
        cursor.seen_at = message.created_at
    return message


@function_macro_(
    "CALL",
    """\
Call an Action as a tool. Results arrive on next turn.
The call will be presented as `<action.name> <object_title>`, thus `<object_title>` should be the object ONLY.
Example `<object_title>`: "history of computing", "[@Page7]", "login button"
""",
    signature="(action: Action, object_title: str | None = None, **inputs) -> None",
    is_terminal=True,
)
def CALL(
    action: Action,
    object_title: TextLineIn | None = None,
    runner: "AgentRunner" = _INJECTED_RUNNER,
    **inputs,
):
    from bench.builtin import InternetService

    # title
    object_title = text_line(object_title) if object_title is not None else None
    if (
        action.environment_type == EnvironmentType.BUILTIN
    ):  # use known good title for builtin actions
        if action.id == InternetService.child(Action, "Search").id and "Query" in inputs:
            object_title = text_line(inputs["Query"])
        elif action.id == InternetService.child(Action, "Read").id and "URL" in inputs:
            object_title = text_line(inputs["URL"])

    # create run
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
        title=object_title,
    )
    runner.call(run)


@function_macro_(
    "ADD_CONTEXT",
    """\
Add context to the current Thread for future reference (noop if already present).
""",
    signature="(*nodes: Page | File) -> Sequence[Claim]",
    is_edit=True,
)
def ADD_CONTEXT(
    *nodes: Page | File,
    runner: "AgentRunner" = _INJECTED_RUNNER,
) -> Sequence[Claim]:
    thread = runner.thread.thread
    existing_claims = list(thread.get_children(Claim))
    claims: list[Claim] = []
    for node in nodes:
        for claim in existing_claims:
            if claim.target_id == node.id:
                existing_claims.append(claim)
                break
        else:
            claim = Claim(type=ClaimType.WRITE, target=node, owned_by=runner.agent)
            thread.add_child(claim)
            claims.append(claim)
    return claims


@function_macro_(
    "CREATE_PAGE",
    """\
Create a new Page. Also adds it to context by default.
""",
    signature="(title: str, icon: str | None = None, parent: Page | Package | None = None, add_to_context: bool = True) -> Page",
    is_edit=True,
)
def CREATE_PAGE(
    title: str,
    icon: str | None = None,
    parent: Page | Package | None = None,
    add_to_context: bool = True,
    runner: "AgentRunner" = _INJECTED_RUNNER,
) -> Page:
    page = Page.new(title=text_line(title), icon=to_icon(icon) if icon else None)
    if parent is None:
        parent = runner.tracked.package
    assert parent is not None, f"no parent for {page!r}"
    parent.add_child(page)
    if add_to_context:
        claim = Claim(type=ClaimType.WRITE, target=page, owned_by=runner.agent)
        runner.thread.thread.add_child(claim)
    return page


@function_macro_(
    "ADD_PAGE_TEXT",
    """\
ADD text to a Page.
""",
    signature="(text: str, page: Page, after: Block | None = None, before: Block | None = None) -> tuple[Block, ...]",
    is_edit=True,
)
def ADD_PAGE_TEXT(
    text: str,
    page: Page,
    after: Block | None = None,
    before: Block | None = None,
    runner: "AgentRunner" = _INJECTED_RUNNER,
) -> tuple[Block, ...]:
    blocks = page.add_text(text, after, before)
    return tuple(blocks)
