import ast
import inspect
import textwrap
from datetime import datetime
from enum import StrEnum
from typing import Generator, cast, override

import pytz
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
    ACTIVE_SESSION,
    EMPTY_SCOPE_DATA,
    NODE_TYPES,
    REGION,
    Action,
    Agent,
    Bench,
    BenchStatus,
    Block,
    Field,
    File,
    Graph,
    Link,
    Message,
    NodeReference,
    NodeType,
    NullEngine,
    Package,
    PackageType,
    Page,
    Record,
    Region,
    Session,
    Supergraph,
    Table,
    Task,
    Thread,
    User,
    UserStatus,
    _is_setup_complete,
    text,
    text_line,
    to_icon,
)
from bench.runtime.model import (
    CodePiece,
    CompoundPiece,
    Piece,
    Prompt,
    SeparatorPiece,
    TextPiece,
    Tokenizer,
    piece_,
    raise_if_none,
)
from bench.utils.oracle import REAL_ORACLE

from .macro import ADD_CONTEXT, ADD_PAGE_TEXT, CALL, SEND

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
    role = "developer"

    @override
    def compile(self, prompt: Prompt, tokenizer: Tokenizer) -> Generator[Piece, None, None]:
        yield SeparatorPiece()
        yield TextPiece(text=f"{self.type.value} Example: {self.title}")
        yield SeparatorPiece()
        _ = yield CodePiece(code=self.code)
        yield SeparatorPiece()


def _make_example_bench() -> tuple[Bench, Package, Session, User]:
    """Create the Bench/Package/... used for examples."""
    bench_ptr = NodeReference(node_type=NodeType.BENCH, id=UUID(int=0), ck=UUID(int=0))
    supergraph = Supergraph(name="Global", root_ptr=bench_ptr)
    graph = Graph(scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES, supergraph=supergraph)
    now = datetime.now(tz=pytz.utc)
    session = Session(
        _default_scope=EMPTY_SCOPE_DATA,
        _engines=(NullEngine(name="fake", scope=EMPTY_SCOPE_DATA, node_types=NODE_TYPES),),
        _local_epoch=0,
        oracle=REAL_ORACLE,
        supergraph=supergraph,
    )
    token = ACTIVE_SESSION.set(session)
    try:
        bench = Bench(
            id=UUID(int=0),
            name="Example",
            slug="example",
            region=REGION,
            status=BenchStatus.ACTIVE,
            created_at=now,
            updated_at=now,
            _supergraph=supergraph,
            _is_new=True,
        )
        package = Package(
            parent=bench,
            type=PackageType.OPEN,
            name="Main",
            slug="main",
            _supergraph=supergraph,
        )
        bench.main_package = package
        user = User(
            status=UserStatus.CREATING,
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


@example_(ExampleType.SNIPPET, title="Call an Action")
def example_call_action(Action1: Action):
    # Action1 should do this
    CALL(Action1, Arg1="https://example.com?...", Arg2=True, object_title="example.com")


@example_(ExampleType.SNIPPET, title="Search multiple things")
def example_search_multiple_things(Action1: Action):
    # search multiple things
    SEND("good question, lemme look that up")
    CALL(Action1, object_title="european AI companies", Query="european AI companies")
    CALL(Action1, object_title="Swiss tech startups", Query="swiss tech startups")


@example_(ExampleType.SNIPPET, title="Excerpt from a Page")
def example_excerpt_from_a_page(Page1: Page):
    # refer to a Page
    SEND("""\
Well, you said here on [@Page1]:
> This is a test
> It's only a test
""")
    SEND("That's in conflict with what you asked for.")


@example_(ExampleType.SNIPPET, title="Write text on a Page")
def example_write_text_on_a_page(NotesPage1: Page, Block7: Block):
    ADD_PAGE_TEXT(
        """\
# Notes
- Item 1 *and* more
- Item 2
- ...
""",
        NotesPage1,
        after=Block7,
    )
    SEND("I've added your notes to [@NotesPage1].")


@example_(ExampleType.SNIPPET, title="Edit a text line on a Page")
def example_edit_a_text_line_on_a_page(NotesPage1: Page, Block7: Block):
    Block7.line = text_line("## New Subtitle")


@example_(ExampleType.SNIPPET, title="Create a simple Table")
def example_create_simple_table(Page1: Page, Link2: Link):
    # basic Person table (Record.title is builtin)
    Table1 = Table.new("Person", Field.member("Age", int))
    Page1.add_child(Table1)
    Record1 = Table1.records.create(name="Florian", Age=27)
    Record2 = Table1.records.create(name="John", Age=30)
    # reference directly by alias
    SEND(
        "I've created [@Table1] and added [@Record1] and [@Record2] from that article. [^NZZ](...)",
        nodes=[Link2],
    )


@example_(ExampleType.SNIPPET, title="Cite Links in Messages")
def example_cite_links_in_messages(Link1: Link, Link2: Link, Link3):
    SEND(
        "Yeah, looks like that PR was merged. [^GH123](httpsgithub.com/symbolx/bench/pull/123)",
    )
    SEND(
        "However, the issue is still open. [^JIRA123](https://symbolx.atlassian.net/browse/BENCH-123)",
    )
    SEND(None, nodes=(Link1, Link2, Link3))


@example_(ExampleType.SNIPPET, title="Add external images to response")
def example_upload_external_files():
    # pick best images from search
    images = (
        File.external("https://example.com/image1.jpg"),
        File.external("https://example.com/image2.jpg"),
    )
    SEND("Yeah, here's what that looks like:", nodes=images)


@example_(ExampleType.SNIPPET, title="Draft report for confirmation")
def example_draft_report_for_confirmation(Page1: Page):
    SEND("Sure, I can do some research and compile my findings on [@Page1].")
    # first sketch the report and ask for clarification
    ADD_CONTEXT(Page1)
    # icon/title is missing
    Page1.title = text_line("European Tech Companies since 2000")
    Page1.icon = to_icon("🇪🇺")
    # sketch out Tasks on top (Tasks usually go in some separate area)
    Task1 = Task.new("Search web for list of tech companies", owned_by=ME)
    Task2 = Task.new("Find information on each company (add notes to page)", owned_by=ME)
    Task3 = Task.new("Rewrite notes into proper report", owned_by=ME)
    TaskBlocks = Page1.add_children(Task1, Task2, Task3)
    # draft outline (with some notes based on the conversation)
    ADD_PAGE_TEXT(
        """\
---
# Notes (to be removed)
- Definitely look at Spotify, Revolut, Adyen, Klarna (as requested)
- Also research international competition and expansion success
- Be concise and and only state facts, little commentary
- No introduction or conclusion
- Add any new tasks that come to mind
- Cite sources inline with citations, add any other Links to bottom
---

# Companies
...

# Expansion
...

# Competition
...

# Further Reading
...
""",
        Page1,
        after=TaskBlocks[-1],
    )
    SEND(
        """\
I've drafted the report on European tech companies on [@Page1].
Review and edit or ask for more, or tell me to go ahead.
""",
    )
