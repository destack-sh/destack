from contextlib import contextmanager
from dataclasses import dataclass
from datetime import datetime
from inspect import getsource
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
    Region,
    Session,
    User,
    UserStatus,
    coerce_custom_object_scalar,
    text,
)
from bench.language.core.object import _is_setup_complete
from bench.language.source.render import Renderer, RenderOptions
from bench.runtime.core import Runner
from bench.utils.oracle import REAL_ORACLE

from .prompt import (
    CompilationContext,
    PromptCode,
    PromptCompound,
    PromptPart,
    PromptRegion,
)


@dataclass
class PromptExample(PromptCompound):
    """An example of a prompt."""

    text: str
    request: str
    response: str

    @override
    async def expand(self, context: "CompilationContext") -> Sequence[PromptPart]:
        return (
            PromptRegion(
                title=self.title,
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
    session = Session(
        parent=None,
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(NullEngine(name="fake", scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES),),
        _local_epoch=0,
        _oracle=REAL_ORACLE,
        _supergraph=supergraph,
    )
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


EXAMPLE_BENCH, EXAMPLE_PACKAGE, EXAMPLE_SESSION, EXAMPLE_USER = make_example_bench()


@contextmanager
def example_session():
    token = ACTIVE_SESSION.set(EXAMPLE_SESSION)
    try:
        yield EXAMPLE_SESSION
    finally:
        ACTIVE_SESSION.reset(token)


# nocheckin: render general and specific examples (categorize?)

EXAMPLES: list[PromptExample] = []


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
        raw_source = getsource(func)
        if "# ---" in raw_source:
            request = raw_source.split("# ---")[0]
        else:
            request = raw_source.split("return")[0]
        request = request.split("\n", 1)[1].strip()

        # response
        response = func(EXAMPLE_PACKAGE)[1]
        if isinstance(response, tuple):
            output_type = response[0].output_type
            assert output_type is not None, f"example {func!r} has no output type"
            response = coerce_custom_object_scalar(response[1], typ=output_type)
        if isinstance(response, CustomObject):
            renderer = Renderer(options=RenderOptions(scope=EXAMPLE_PACKAGE))
            response = renderer.render_custom_object(response)
            response = f"return {response}"

        # example
        example = PromptExample(
            title=title, text=text, request=request, response=response, weight=weight
        )
        EXAMPLES.append(example)

    return decorator


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


def get_examples(action: Action, runner: Runner) -> Sequence[PromptExample]:
    examples: list[PromptExample] = []

    return examples
