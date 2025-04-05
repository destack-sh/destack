import ast
import inspect
import textwrap
from datetime import datetime
from enum import StrEnum
from typing import Generator, override
from uuid import UUID

import pytz
import structlog
from opentelemetry import trace

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    REGION,
    Bench,
    BenchStatus,
    Message,
    NodeGraph,
    NodeReference,
    NodeSuperGraph,
    NodeType,
    NullEngine,
    Package,
    PackageType,
    Region,
    Session,
    Thread,
    User,
    UserStatus,
    _is_setup_complete,
    text,
    text_line,
)
from bench.utils.oracle import REAL_ORACLE

from .piece import (
    CodePiece,
    CompoundPiece,
    Piece,
    SeparatorPiece,
    TextPiece,
    piece_,
    raise_if_none,
)
from .prompt import Prompt
from .token import Tokenizer

# ruff: noqa: F401,B018
# pyright: reportUnusedExpression=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

# NOTE: only import this file after import is complete
assert _is_setup_complete(), "import this file after import is complete"


class ExampleType(StrEnum):
    SNIPPET = "Snippet"
    RESPONSE = "Response"


@piece_()
class ExamplePiece(CompoundPiece):
    type: ExampleType = raise_if_none()
    title: str = raise_if_none()
    code: str = raise_if_none()

    @override
    def compile(
        self, prompt: Prompt, tokenizer: Tokenizer, remaining_tokens: int
    ) -> Generator[Piece, int, None]:
        yield SeparatorPiece()
        yield TextPiece(text=f"{self.type.value} Example: {self.title}")
        yield SeparatorPiece()
        _ = yield CodePiece(code=self.code)
        yield SeparatorPiece()


def _make_example_bench() -> tuple[Bench, Package, Session, User]:
    """Create the Bench/Package/... used for examples."""
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
EXAMPLES: list[ExamplePiece] = []


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


def example_(example_type: ExampleType, title: str):
    def decorator(func):
        code = _get_function_body(func).strip()
        example = ExamplePiece(type=example_type, title=title, code=code)
        EXAMPLES.append(example)
        return func

    return decorator


@example_(ExampleType.SNIPPET, title="Reply to a Message directly")
def example_reply_to_message_1(Thread1: Thread, Message1: Message):  # noqa: N803
    # reply to specific message
    Reply1 = Message.new(text=text("yeah, I'll get right on this"), reply_to=Message1)
    Thread1.append(Reply1)


@example_(ExampleType.SNIPPET, title="Reply to a Message indirectly, title is missing")
def example_reply_to_message_2(Thread1: Thread, Message1: Message):  # noqa: N803
    # continue conversation
    Thread1.title = text_line("The Solar System")  # title was missing
    Reply1 = Message.new(
        text=text("The distance from Earth to the moon is **about 384,400 km** (238,855 miles)."),
    )
    Thread1.append(Reply1)


@example_(ExampleType.SNIPPET, title="Reply to a Message with code")
def example_reply_to_message_3(Thread1: Thread, Message1: Message):  # noqa: N803
    # reply to specific message
    Reply1 = Message.new(
        text=text("""\
A simple Hello World in Rust would look like this:

```rust
fn main() {
    println!("Hello, world!");
}
```
"""),
    )
    Thread1.append(Reply1)
