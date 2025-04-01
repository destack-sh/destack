import ast
import inspect
import textwrap
from dataclasses import dataclass
from datetime import datetime
from typing import Any, Callable, NamedTuple, Sequence, override
from uuid import UUID

import pytz
import structlog
from opentelemetry import trace

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    REGION,
    Action,
    ActionType,
    Aliasing,
    Bench,
    BenchStatus,
    Block,
    BlockType,
    Channel,
    Choice,
    CustomObject,
    Field,
    Flow,
    LinkType,
    Message,
    Node,
    NodeGraph,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    NullEngine,
    Option,
    Package,
    PackageType,
    PathElementType,
    Plan,
    PlanTerminationMode,
    Region,
    Renderer,
    RenderOptions,
    Run,
    RunnableNode,
    Session,
    Task,
    User,
    UserStatus,
    _is_setup_complete,
    coerce_custom_object_scalar,
    format_code,
    text,
    text_line,
)
from bench.runtime.core import Runner
from bench.utils.oracle import REAL_ORACLE

from .prompt import Prompt, PromptCode, PromptCompound, PromptPart, PromptRegion

# ruff: noqa: F401,B018
# pyright: reportUnusedExpression=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE: only import this file after import is complete
assert _is_setup_complete(), "import this file after import is complete"


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


def _make_example_bench() -> tuple[Bench, Package, Session, User]:
    bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = NodeSuperGraph(name="Global", root_ptr=bench_ptr)
    graph = NodeGraph(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES, supergraph=supergraph)
    now = datetime.now(tz=pytz.utc)
    session = Session(
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
            status=BenchStatus.ACTIVATED,
            created_at=now,
            updated_at=now,
            _supergraph=supergraph,
            _is_new=True,
        )
        main_package = Package(
            parent=bench,
            type=PackageType.OPEN,
            name="Main",
            slug="main",
            _supergraph=supergraph,
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


EXAMPLE_BENCH, EXAMPLE_PACKAGE, EXAMPLE_SESSION, EXAMPLE_USER = _make_example_bench()
EXAMPLES: list[PromptExample] = []


def _get_function_body(func) -> str:
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


class ExampleIn(NamedTuple):
    nodes: Sequence[Node]
    response: "ExampleResponseIn"


class ExampleResponseIn(NamedTuple):
    node: RunnableNode
    code: str | None = None
    outputs: dict[str, Any] | None = None
    plans: list[Plan] | None = None
    comment: str | None = None


def example_(title: str, weight: int = 1):
    """Register an example."""

    def decorator(
        func: Callable[
            [Package],
            ExampleIn,
        ],
    ):
        text = func.__doc__
        assert text, f"example {func!r} has no docstring"

        # request
        raw_source = _get_function_body(func)
        if "# ---" in raw_source:
            request = raw_source.split("# ---")[0]
        else:
            request = raw_source.split("return")[0]

        # response
        token = ACTIVE_SESSION.set(EXAMPLE_SESSION)
        try:
            example_in = func(EXAMPLE_PACKAGE)
            response_in = example_in.response

            # outputs
            output_type = response_in.node.output_type
            assert output_type is not None, f"example {func!r} has no output type"
            outputs = coerce_custom_object_scalar(
                response_in.outputs, typ=output_type, supergraph=EXAMPLE_PACKAGE._supergraph
            )
            renderer = Renderer(options=RenderOptions(scope=EXAMPLE_PACKAGE, aliasing=Aliasing()))
            outputs_code = renderer.render_custom_object(outputs, implicit_partials=True)

            # plans
            if response_in.plans:
                plans_code = []
                for i, plan in enumerate(response_in.plans):
                    plans_code.append(f"next_plan{i + 1} = {renderer.render_object(plan)}")
                plans_code.append(
                    f"run.plans.extend({', '.join(f'next_plan{i + 1}' for i in range(len(response_in.plans)))})"
                )
                plans_code = "\n".join(plans_code)
            else:
                plans_code = None

            # response code
            response_code = response_in.code or f"return {outputs_code}"
            if plans_code:
                response_code = f"{plans_code}\n{response_code}"
            if response_in.comment:
                response_code = f"# {response_in.comment}\n{response_code}"
            response_code = format_code(response_code)

            # example
            example = PromptExample(
                title=title, text=text, request=request, response=response_code, weight=weight
            )
            EXAMPLES.append(example)
            logger.trace("example.generate", title=title)
        except Exception as e:
            logger.exception("example.generate.error", title=title, exc_info=e)
            raise
        finally:
            ACTIVE_SESSION.reset(token)

    return decorator


#
# Basics
#


@example_("Field Reference")
def basic_field_reference(package: Package):
    """How to reference a Field or any other Node."""
    Sentiment = Choice.new(
        "Sentiment",
        options=[
            Option.new("Happy"),
            Option.new("Sad"),
            Option.new("Angry"),
            Option.new("Neutral"),
        ],
    )
    Action1 = Action.new(
        ActionType.DO,
        name="Classify",
        fields=[Field.input("Text", str), Field.output("Sentiment", Sentiment)],
    )
    # Input
    {"Text": "Feeling pretty good today."}
    # ---
    return ExampleIn(
        nodes=[Sentiment, Action1],
        response=ExampleResponseIn(node=Action1, outputs={"Sentiment": Sentiment.options.Happy}),
    )


#
# Actions
#


@example_("Simple Action Outputs")
def action_simple_output(package: Package):
    """A super simple meaningless Action."""
    Action1 = Action.new(
        ActionType.DO,
        name="Action1",
        text=text("Generate some example outputs"),
        fields=[Field.output("Output1", str), Field.output("Output2", str, is_required=True)],
    )
    # ---
    return ExampleIn(
        nodes=[Action1],
        response=ExampleResponseIn(node=Action1, outputs={"Output2": "Hello World!"}),
    )


@example_("Fail Impossible Request")
def action_failing_impossible_request(package: Package):
    """How to fail an impossible request."""
    Act1 = Action.new(
        ActionType.DO,
        name="Act1",
        text=text("Generate the solution to all the worlds problem in one go"),
    )
    # ---
    return ExampleIn(
        nodes=[Act1],
        response=ExampleResponseIn(
            node=Act1,
            code="""\
raise IncapableError("I'm afraid I cannot do that.")
""",
        ),
    )


@example_("Refuse Disallowed Request")
def action_refusing_disallowed_request(package: Package):
    """How to refuse an disallowed request."""
    Generate1 = Action.new(
        ActionType.DO,
        name="Generate1",
        text=text("Implement some obviously terrible logic for nefarious purposes"),
    )
    # ---
    return ExampleIn(
        nodes=[Generate1],
        response=ExampleResponseIn(
            node=Generate1,
            code="""\
raise RefusedError("I cannot assist with that.")
""",
        ),
    )


#
# Flows
#

# nocheckin: flow/action/message/planning examples


def get_examples(action: Action, runner: Runner) -> Sequence[PromptExample]:
    examples: list[PromptExample] = [*EXAMPLES]
    # TODO :Tuning: select specific examples for action/runner?
    return examples
