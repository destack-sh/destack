import ast
import inspect
import textwrap
from datetime import datetime
from enum import StrEnum
from typing import Generator, cast, override
from uuid import UUID

import pytz
import structlog
from opentelemetry import trace

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    REGION,
    Agent,
    Bench,
    BenchStatus,
    File,
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
    to_icon,
)
from bench.utils.oracle import REAL_ORACLE

from .macro import SEND
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

# ruff: noqa: F401,B018,N803,F841
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
    # get source
    lines, _ = inspect.getsourcelines(func)
    src = "".join(lines)
    mod = ast.parse(src)
    func_node = mod.body[0]
    body_lines = lines[func_node.lineno :]
    # dedent
    first_line = body_lines[0]
    indent = len(first_line) - len(first_line.lstrip())
    trimmed_lines = []
    for line in body_lines:
        if line.strip():
            trimmed_lines.append(line[min(indent, len(line) - len(line.lstrip())) :])
        else:
            trimmed_lines.append(line)
    inner_code = "".join(trimmed_lines)
    return inner_code


def example_(example_type: ExampleType, title: str):
    def decorator(func):
        code = _get_function_body(func).strip()
        example = ExamplePiece(type=example_type, title=title, code=code)
        EXAMPLES.append(example)
        return func

    return decorator


# constant macros
THREAD: Thread = cast(Thread, None)
ME: Agent = cast(Agent, None)


@example_(ExampleType.SNIPPET, title="Reply to a Message directly")
def example_reply_to_message_1(Message1: Message):
    # reply to specific message
    SEND("yeah, I'll get right on this, sorry I missed it", reply_to=Message1)


@example_(ExampleType.SNIPPET, title="Reply to a Message indirectly, title is missing")
def example_reply_to_message_2():
    # continue conversation
    THREAD.title = text_line("The Solar System")  # title was missing
    THREAD.icon = to_icon("☀️")  # there's an appropriate icon we could use
    SEND("The distance from Earth to the moon is **about 384,400 km** (238,855 miles).")
    SEND("(FYI, The Earth is about 12742 km (7918 miles) in diameter.)")


@example_(ExampleType.SNIPPET, title="Reply to a Message with code in two parts")
def example_reply_to_message_3():
    THREAD.title = text_line("Rust Basics")
    THREAD.icon = to_icon("fab fa-rust")
    SEND("""\
A simple Hello World in Rust would look like this:
```rust
fn main() {
    println!("Hello, world!");
}
```""")
    SEND("""\
Or you could do it like this with your `hilib` crate:
```rust
use hilib::{hello, goodbye};

fn main() {
    hello();
    goodbye();
}
```""")


@example_(ExampleType.SNIPPET, title="Don't do anything if not needed")
def example_reply_to_message_4(Thread1: Thread, Message1: Message):
    # ignore, isn't related to me and wasn't asked
    pass


@example_(ExampleType.SNIPPET, title="Mention specific things")
def example_mention_specific_things():
    # tag the relevant agent, and mention the specific file
    SEND("yup [@Agent2], please look at the file I was talking about: [@File1]\n(cc [@User2])")


@example_(ExampleType.SNIPPET, title="Use quotes as needed")
def example_use_quotes_as_needed(Thread1: Thread, File1: File):
    SEND("""\
This is the text from [@File1]:
> Total amount: $100.00
> Date: 2021-01-01
> Description: 79kg of rice
""")
    SEND("(btw the stuff at the borders wasn't really legible)")
    SEND("""\
And here's how to file it:
1. Go to the accounting dashboard
2. Ask someone who knows
""")


@example_(ExampleType.SNIPPET, title="Include code block language")
def example_always_include_code_block_language():
    # always to include the code language (even for markdown)
    text1 = text("""\
Sure, here's how you make lists in markdown:
```markdown
- Item 1
- Item 2
```
""")
