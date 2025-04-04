from bench.language import Flow, Session
from bench.runtime.model import Prompt, RegionPiece, StupidTokenizer, TextPiece, compile_prompt


def test_prompt_compile(session: Session):
    # setup
    flow = Flow.new("Flow")

    # prompt
    prompt = Prompt(node=flow, system_prompt="You are a helpful assistant.")
    prompt.text("1" * 100)
    prompt.text("2" * 100, priority=2)
    prompt.text("3" * 30, priority=10)
    prompt.text("4" * 30, priority=4)
    prompt.region(
        "nested",
        TextPiece(text="5" * 10, priority=1),
        TextPiece(text="6" * 10, priority=6),
        RegionPiece(
            text="nested inner",
            pieces=[
                TextPiece(text="7" * 10, priority=7),
                TextPiece(text="8" * 10, priority=8),
                TextPiece(text="9" * 100, priority=9),
            ],
            priority=2,
        ),
        priority=3,
    )

    # compile
    tokenizer = StupidTokenizer()
    pieces, used_tokens = compile_prompt(prompt=prompt, tokenizer=tokenizer, max_tokens=200)
    print(used_tokens)
    for piece in pieces:
        print(piece)
