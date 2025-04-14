from bench.language import Agent, Flow, Session
from bench.runtime.model import Prompt, RegionPiece, StupidTokenizer, TextPiece, compile_prompt


def test_prompt_compile(session: Session):
    # setup
    flow = Flow.new("Flow")
    agent = Agent.new("Agent")

    # prompt
    prompt = Prompt(
        subject=agent, session=session, node=flow, system_prompt="You are a helpful assistant."
    )
    prompt.text("1" * 100, role="user")
    prompt.text("2" * 100, priority=2, role="user")
    prompt.text("3" * 30, priority=10, role="user")
    prompt.text("4" * 30, priority=4, role="user")
    prompt.region(
        "nested",
        None,
        TextPiece(text="5" * 10, priority=1, role="user"),
        TextPiece(text="6" * 10, priority=6, role="user"),
        RegionPiece(
            title="nested inner",
            pieces=[
                TextPiece(text="7" * 10, priority=7, role="user"),
                TextPiece(text="8" * 10, priority=8, role="user"),
                TextPiece(text="9" * 100, priority=9, role="user"),
            ],
            priority=2,
            role="user",
        ),
        priority=3,
        role="user",
    )

    # compile
    tokenizer = StupidTokenizer()
    pieces, used_tokens = compile_prompt(prompt=prompt, tokenizer=tokenizer, max_tokens=200)
    print(used_tokens)  # noqa: T201
    for piece in pieces:
        print(piece)  # noqa: T201
