from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field
from bench.language.project import ProjectOptions, project
from bench.language.session import Session
from bench.language.text import Text, TextLine, TextSpan


def test_project(shared_session: Session, shared_package: Package):
    GeneralInstruction = Block.new(BlockType.PAGE, "GeneralInstruction")
    SomeInstruction = Block.new_text("SomeInstruction", "Alright, so consider the elephant")
    GeneralInstruction.blocks.append(SomeInstruction)

    Writing = Block.new(BlockType.PAGE, "Writing")
    Mood = Block.new(
        BlockType.CHOICE,
        "Mood",
        fields=[Field.option("Positive"), Field.option("Neutral"), Field.option("Negative")],
    )
    Style = Block.new(
        BlockType.CLASS,
        "Style",
        fields=[
            Field.member(
                "Formality", int, text=Text(lines=[TextLine(spans=[TextSpan.new(SomeInstruction)])])
            )
        ],
    )
    JudgeWriting = Block.new_text(
        "JudgeWriting",
        "",
        fields=[Field.input("Text", str), Field.output("Style", Style), Field.output("Mood", Mood)],
    )
    Writing.blocks.extend(Mood, Style)

    # project
    projection = project(JudgeWriting, options=ProjectOptions())
    assert Style in projection
    assert Mood in projection
    assert SomeInstruction in projection

    # get pages
    containing_pages = projection.get_containing_pages()
    assert containing_pages == [Writing, GeneralInstruction]
