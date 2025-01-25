import ast
import inspect
import textwrap
from dataclasses import dataclass
from datetime import datetime
from typing import Callable, Sequence, override
from uuid import UUID

import pytz

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    REGION,
    Action,
    ActionType,
    Bench,
    Block,
    BlockType,
    CallTerminationMode,
    CustomObject,
    Field,
    Node,
    NodeGraph,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    NullEngine,
    Package,
    PackageType,
    PathElement,
    PathElementType,
    PipeType,
    Region,
    Renderer,
    RenderOptions,
    Run,
    Session,
    User,
    UserStatus,
    _is_setup_complete,
    call,
    call_serial,
    coerce_custom_object_scalar,
    format_code,
    text,
)
from bench.runtime.core import Runner
from bench.utils.oracle import REAL_ORACLE

from .prompt import Prompt, PromptCode, PromptCompound, PromptPart, PromptRegion

# ruff: noqa: F401,B018
# pyright: reportUnusedExpression=false


@dataclass
class PromptExample(PromptCompound):
    """An example of a prompt."""

    text: str
    request: str
    response: str

    @override
    async def expand(self, prompt: "Prompt") -> Sequence[PromptPart]:
        return (
            PromptRegion(
                title=f"Example: {self.title}",
                weight=1,
                text=self.text,
                content=[
                    PromptCode(title="Request", code=self.request),
                    PromptCode(title="Response", code=self.response),
                ],
            ),
        )


# NOTE: only import this file after import is complete
assert _is_setup_complete(), "import this file after import is complete"


def make_example_bench() -> tuple[Bench, Package, Session, User]:
    bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(name="Global", root_ptr=bench_ptr)
    graph = NodeGraph(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES, supergraph=supergraph)
    now = datetime.now(tz=pytz.utc)
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(NullEngine(name="fake", scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES),),
        _local_epoch=0,
        _oracle=REAL_ORACLE,
        _supergraph=supergraph,
    )
    token = ACTIVE_SESSION.set(session)
    try:
        bench = Bench(
            id=UUID(int=0),
            name="Example",
            slug="example",
            region=REGION,
            encryption_key="lol",
            created_at=now,
            updated_at=now,
            _supergraph=supergraph,
            _is_new=True,
        )
        main_package = Package(
            parent=bench, type=PackageType.ROOT, name="Main", slug="main", _supergraph=supergraph
        )
        bench.main_package = main_package
        user = User(
            status=UserStatus.REGISTERED,
            region=Region.ZURICH,
            slug="example",
            email="example@symbolx.com",
            name="Example",
            _graph=graph,
            _supergraph=supergraph,
            _session=session,
        )
        supergraph._root_ptr = user.to_ref()
        return bench, bench.main_package, session, user
    finally:
        ACTIVE_SESSION.reset(token)


EXAMPLE_BENCH, EXAMPLE_PACKAGE, EXAMPLE_SESSION, EXAMPLE_USER = make_example_bench()
EXAMPLES: list[PromptExample] = []


def get_function_body(func) -> str:
    """Extract the body of a function as a nicely formatted string."""
    lines, _ = inspect.getsourcelines(func)
    src = "".join(lines)
    mod = ast.parse(src)
    func_node = mod.body[0]
    body_nodes = func_node.body  # type: ignore
    if (
        body_nodes
        and isinstance(body_nodes[0], ast.Expr)  # type: ignore
        and isinstance(body_nodes[0].value, ast.Constant)  # type: ignore
        and isinstance(body_nodes[0].value.value, str)  # type: ignore
    ):
        body_nodes = body_nodes[1:]  # type: ignore
    first_stmt_lineno = body_nodes[0].lineno
    body_lines = lines[first_stmt_lineno - 1 :]
    return textwrap.dedent("".join(body_lines))


def example_(title: str, weight: int = 1):
    """Register an example."""

    def decorator(
        func: Callable[
            [Package], tuple[Sequence[Node] | str, CustomObject | tuple[Action, dict] | str]
        ],
    ):
        text = func.__doc__
        assert text, f"example {func!r} has no docstring"

        # request
        raw_source = get_function_body(func)
        if "# ---" in raw_source:
            request = raw_source.split("# ---")[0]
        else:
            request = raw_source.split("return")[0]

        # response
        token = ACTIVE_SESSION.set(EXAMPLE_SESSION)
        try:
            response = func(EXAMPLE_PACKAGE)[1]
            if isinstance(response, tuple):
                output_type = response[0].output_type
                assert output_type is not None, f"example {func!r} has no output type"
                response = coerce_custom_object_scalar(
                    response[1], typ=output_type, supergraph=EXAMPLE_PACKAGE._supergraph
                )
            if isinstance(response, CustomObject):
                renderer = Renderer(options=RenderOptions(scope=EXAMPLE_PACKAGE))
                response = renderer.render_custom_object(response)
                response = f"return {response}"
                response = format_code(response)
        finally:
            ACTIVE_SESSION.reset(token)

        # example
        example = PromptExample(
            title=title, text=text, request=request, response=response, weight=weight
        )
        EXAMPLES.append(example)

    return decorator


#
# Basics
#


@example_("Field Reference")
def field_reference(package: Package):
    """How to reference a Field or any other Node."""
    Sentiment = Block.new(
        BlockType.CHOICE,
        name="Sentiment",
        fields=[
            Field.option("Happy"),
            Field.option("Sad"),
            Field.option("Angry"),
            Field.option("Neutral"),
        ],
    )
    Action1 = Action.new(
        ActionType.CLASSIFY,
        name="Classify",
        fields=[Field.input("Text", str), Field.output("Sentiment", Sentiment)],
    )
    # Input
    {"Text": "Feeling pretty good today."}
    return [Sentiment, Action1], (Action1, {"Sentiment": Sentiment.fields.Happy})


#
# Flows
#


@example_("Simple Action Outputs")
def simple_action(package: Package):
    """A super simple meaningless Action."""
    Action1 = Action.new(
        ActionType.GENERATE,
        name="Action1",
        text=text("Generate some example outputs"),
        fields=[Field.output("Output1", str), Field.output("Output2", str, is_required=True)],
    )
    # ---
    return [Action1], (Action1, {"Output2": "Hello World!"})


@example_("Failing an Impossible Request")
def failing_impossible_request(package: Package):
    """How to fail an impossible request."""
    Act1 = Action.new(
        ActionType.ACT,
        name="Act1",
        text=text("Generate the solution to all the worlds problem in one go"),
    )
    # ---
    return (
        [Act1],
        """\
raise ModelIncapableError("I'm afraid I cannot do that.")
""",
    )


@example_("Refusing an Illegal Request")
def refusing_illegal_request(package: Package):
    """How to refuse an illegal request."""
    Generate1 = Action.new(
        ActionType.GENERATE,
        name="Generate1",
        text=text("Generate some obviously terrible outputs for nefarious purposes"),
    )
    # ---
    return (
        [Generate1],
        """\
raise ModelRefusedError("I'm afraid I cannot do that.")
""",
    )


@example_("Basic Planning")
def basic_planning(package: Package):
    """How to plan next Actions in a simple Flow with fixed Actions."""
    Flow = Block.new(BlockType.FLOW, name="Flow1")
    Start = Action.new(ActionType.START, name="Start")
    Look1 = Action.new(ActionType.LOOK, name="Look1")
    Think1 = Action.new(ActionType.THINK, name="Think1")
    Click1 = Action.new(ActionType.CLICK, name="Click1")
    Type1 = Action.new(ActionType.TYPE, name="Type1")
    Press1 = Action.new(ActionType.PRESS, name="Press1")
    Complete = Action.new(ActionType.COMPLETE, name="Complete")
    Flow.actions.extend(Start, Look1, Think1, Click1, Type1, Press1, Complete)
    Start.connect(PipeType.CALL, Look1)
    Look1.connect(PipeType.CALL, Think1)
    Think1.connect(PipeType.SELECT, Click1)
    Think1.connect(PipeType.SELECT, Type1)
    Think1.connect(PipeType.SELECT, Press1)
    Think1.connect(PipeType.SELECT, Look1)
    Think1.connect(PipeType.SELECT, Complete)
    # Runs/Inputs
    ...  # some application with obvious element ids provided
    # We're at Think1, assume we know the next few steps
    # ---
    plans = [
        call_serial(
            call(Click1, element_id="7"),
            call(Type1, element_id="2", string="florian@symbolx.com"),
            call(Press1, element_id="3", keys="Enter"),
            on_terminate=CallTerminationMode.RETURN,  # back to Think when done
        )
    ]
    return [Flow, *Flow.actions, *Flow.pipes], (Think1, {"plans": plans})


# nocheckin
# @example_("Basic Planning with Tool Actions")
# def basic_planning_with_tools(package: Package):
#     """How to plan next Actions in a simple Flow with Tool Actions."""
#     Flow = Block.new(BlockType.FLOW, name="Flow1")
#     Start = Action.new(ActionType.START, name="Start")


@example_("Simple extract without a plan")
def simple_extract_without_plan(package: Package):
    """How to plan next Actions when the Flow is done."""
    Flow = Block.new(
        BlockType.FLOW,
        name="Flow1",
        fields=[Field.input("Text", str), Field.output("Names", str, is_list=True)],
    )
    Start = Action.new(ActionType.START, name="Start")
    Extract = Action.new(
        ActionType.EXTRACT, name="Extract", fields=[Field.output("Names", str, is_list=True)]
    )
    Complete = Action.new(ActionType.COMPLETE, name="Complete")
    Complete.set_computed(
        target=(PathElementType.RUN, Run.get_property("inputs"), Flow.fields.Names),
        source=(Extract, PathElementType.RUN, Run.get_property("outputs"), Extract.fields.Names),
    )
    Flow.actions.extend(Start, Extract, Complete)
    Start.connect(PipeType.CALL, Extract)
    Extract.connect(PipeType.CALL, Complete)
    # Inputs
    {"Text": "And then Alice met Bob at the park."}
    # ---
    return [Flow, *Flow.actions, *Flow.pipes], (Extract, {"Names": ["Alice", "Bob"], "plans": []})


def get_examples(action: Action, runner: Runner) -> Sequence[PromptExample]:
    examples: list[PromptExample] = [*EXAMPLES]
    # TODO :Tuning: select specific examples for action/runner?
    return examples
