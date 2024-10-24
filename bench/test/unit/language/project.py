from bench.language.bench import Package
from bench.language.block import Block
from bench.language.const import BlockType
from bench.language.field import Field, to_type
from bench.language.file import File, FileKind, FileType
from bench.language.project import ProjectOptions, project
from bench.language.session import Session
from bench.language.text import Text, TextLine, TextSpan


def test_project_pages(shared_session: Session, shared_package: Package):
    GeneralInstruction = Block.new(BlockType.PAGE, "GeneralInstruction")
    SomeInstruction = Block.new_text("SomeInstruction", "Alright, so consider the elephant")
    SomeOtherInstruction = Block.new_text("SomeOtherInstruction", "It's big and pink")
    GeneralInstruction.blocks.extend(SomeInstruction, SomeOtherInstruction)

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
    Writing.blocks.extend(Mood, Style)
    JudgeWriting = Block.new_text(
        "JudgeWriting",
        "",
        fields=[Field.input("Text", str), Field.output("Style", Style), Field.output("Mood", Mood)],
    )

    # project
    projection = project(JudgeWriting, options=ProjectOptions())
    assert Style in projection
    assert Mood in projection
    assert SomeInstruction in projection
    assert SomeOtherInstruction in projection  # via containing page

    # get pages
    containing_pages = projection.get_containing_pages()
    assert containing_pages == [Writing, GeneralInstruction]


def test_project_custom_objects(shared_session: Session, shared_package: Package):
    # nest node ref in nested custom object
    Class1 = Block.new(
        BlockType.CLASS,
        "Class1",
        fields=[
            Field.member("Int", int),
            Field.member("String", str),
            Field.member("Block", Block),
        ],
    )
    Class2 = Block.new(
        BlockType.CLASS,
        "Class2",
        fields=[Field.member("Count", int), Field.member("Class1", Class1)],
    )
    CustomObject1 = Class1(Int=1, String="One", Block=Class2)
    CustomObject2 = Class2(Count=2, Class1=CustomObject1)

    # nested node ref in custom object in builtin object
    Text1 = Block.new_text("Text1", "Hello, world!")
    Variable1 = Block.new(
        BlockType.VALUE,
        "Variable1",
        value_type=to_type(Block),
        value=Text1,
    )

    # project
    projection = project(CustomObject2, Variable1, options=ProjectOptions())
    assert Class2 in projection  # via CustomObject2.Class1.Block
    assert Text1 in projection  # via CustomObject2.Variable1.Text1


def test_project_missing_nodes(shared_session: Session, shared_package: Package):
    """'Missing' nodes that aren't loaded in the graph should also be projected."""
    Class1 = Block.new(BlockType.CLASS, "Class1", fields=[Field.member("File", File)])
    File1 = File(
        parent=shared_package.bench.main_drive,
        kind=FileKind.DRIVE,
        title="myfile1.txt",
        type=FileType.TEXT,
        size=1024,
    )
    File2 = File(
        parent=shared_package.bench.main_drive,
        kind=FileKind.DRIVE,
        title="myfile2.txt",
        type=FileType.TEXT,
        size=1024,
    )
    File3 = File(
        parent=shared_package.bench.main_drive,
        kind=FileKind.DRIVE,
        title="myfile3.txt",
        type=FileType.TEXT,
        size=1024,
    )
    Variable1 = Block.new(
        BlockType.VALUE,
        "Variable1",
        value_type=to_type(File),
        value=File1,
    )
    Variable2 = Block.new(
        BlockType.VALUE,
        "Variable2",
        value_type=to_type(Class1),
        value=Class1(File=File3),
    )
    Text3 = Block.new_text(
        "Text3",
        Text(lines=[TextLine(spans=[TextSpan(node=File2)])]),
    )
    File1._unload_rec()
    File2._unload_rec()
    File3._unload_rec()

    # project
    projection = project(Variable1, Variable2, Text3, options=ProjectOptions())
    assert Variable1 in projection
    assert Variable2 in projection
    assert projection.has_missing(File1)
    assert projection.has_missing(File2)
    assert projection.has_missing(File3)
