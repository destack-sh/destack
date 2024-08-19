import io

import numpy as np
import pytest
from PIL import Image

from bench.language.block import Block
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field, to_type
from bench.language.file import File, FileFormat, FileType, upload
from bench.language.run import ModelOptions, ModelProvider, RunErrorType, RunOptions
from bench.language.text import md
from bench.language.validation import constraint
from bench.test.unit.conftest import RuntimeHandle

TEST_MODEL_PROVIDERS = (ModelProvider.OPENAI, ModelProvider.ANTHROPIC)


def _for_every_provider():
    return pytest.mark.parametrize(
        "model_provider", TEST_MODEL_PROVIDERS, ids=lambda p: p.name if p else p
    )


async def test_run_text_empty(local_runtime: RuntimeHandle):
    """Empty Text without any fields should fail."""
    runtime = local_runtime
    Text1 = Block.new_text("Text1", "")
    runtime.page().blocks.append(Text1)
    await runtime.commit()

    run = await runtime.run(Text1, return_error=True)
    assert run.status == RunStatus.FAILED
    assert run.error and run.error.type == RunErrorType.RUN_IMPOSSIBLE
    assert len(run.attempts) == 1


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_output_scalar(local_runtime: RuntimeHandle, model_provider: ModelProvider):
    runtime = local_runtime
    AnalyzeSentiment = Block.new_text(
        "AnalyzeSentiment",
        "",
        fields=[Field.input("Text", str), Field.output("IsHappy", bool)],
        run_options=RunOptions(max_attempts=1, model_options=ModelOptions(provider=model_provider)),
    )
    runtime.page().blocks.append(AnalyzeSentiment)
    await runtime.commit()

    run = await runtime.run(
        AnalyzeSentiment, inputs={"Text": "Today was a great day."}, return_error=True
    )
    assert run.status == RunStatus.COMPLETED
    assert run.outputs and run.outputs.IsHappy is True


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_output_dict(local_runtime: RuntimeHandle, model_provider: ModelProvider):
    runtime = local_runtime
    Mood = Block.new(
        BlockType.CHOICE,
        "Mood",
        fields=[Field.option("Positive"), Field.option("Neutral"), Field.option("Negative")],
        run_options=RunOptions(max_attempts=1, model_options=ModelOptions(provider=model_provider)),
    )
    WritingStyle = Block.new(
        BlockType.CLASS,
        "WritingStyle",
        fields=[
            Field.member(
                "formality",
                int,
                constraint(min_value=0, max_value=10),
                text=md("0 is super casual slang, 5 for normal, 10 is high formal prose"),
            )
        ],
    )
    AnalyzeSentiment = Block.new_text(
        "AnalyzeSentiment",
        "",
        fields=[
            Field.input("Text", str),
            Field.output("Style", WritingStyle),
            Field.output("Mood", Mood),
        ],
        run_options=RunOptions(model_options=ModelOptions(provider=model_provider)),
    )
    runtime.page().blocks.extend(Mood, WritingStyle, AnalyzeSentiment)
    await runtime.commit()

    run = await runtime.run(
        AnalyzeSentiment, inputs={"Text": "today's a great day"}, return_error=True
    )
    assert run.status == RunStatus.COMPLETED
    assert run.outputs
    assert run.outputs.Mood == Mood.fields.Positive
    assert run.outputs.Style and run.outputs.Style.formality > 0  # type: ignore


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_with_solid_images(
    hosted_runtime: RuntimeHandle, model_provider: ModelProvider
):
    # task
    runtime = hosted_runtime
    Hue = Block.new(
        BlockType.CHOICE,
        "Hue",
        fields=[Field.option("Red"), Field.option("Green"), Field.option("Blue")],
    )
    DetectColor = Block.new_text(
        "DetectColor",
        "Detect the primary color of the given image",
        fields=[
            Field.input("Image", File, constraint(file_type=FileType.IMAGE)),
            Field.output("Hue", Hue),
        ],
        run_options=RunOptions(model_options=ModelOptions(provider=model_provider)),
    )
    runtime.page().blocks.extend(Hue, DetectColor)
    await runtime.commit()

    # input file
    red_image = Image.new("RGB", (320, 240), color="red")
    image_file = await upload(red_image, "red.png")
    await runtime.commit()
    image_file._unload_rec()
    image_file._clear_cache()

    run = await runtime.run(DetectColor, inputs={"Image": image_file.to_ref()})
    assert run.outputs and run.outputs.Hue is Hue.fields.Red


@pytest.mark.model()
@_for_every_provider()
async def test_run_text_with_giant_images(
    hosted_runtime: RuntimeHandle, model_provider: ModelProvider
):
    # task
    runtime = hosted_runtime
    TitleImage = Block.new(
        BlockType.TEXT,
        "TitleImage",
        fields=[
            Field.input("image", File, constraint(file_type=FileType.IMAGE)),
            Field.output("title", str, text=md("A fitting title of the image")),
        ],
    )
    runtime.page().blocks.append(TitleImage)
    await runtime.commit()

    # input file (giant noise image)
    width, height = 4096, 4096
    noise = np.random.rand(height, width, 3) * 255
    noise = Image.fromarray(noise.astype(np.uint8))
    image_file = await upload(noise, "noise.png")
    await runtime.commit()
    image_file._unload_rec()
    image_file._clear_cache()

    _ = await runtime.run(TitleImage, inputs={"image": image_file.to_ref()})


def generate_docx_file(text: str) -> bytes:
    """Generate a docx file with the given text."""
    from docx import Document

    document = Document()
    document.add_paragraph(text)
    buffer = io.BytesIO()
    document.save(buffer)
    return buffer.getvalue()


@pytest.mark.model()
@pytest.mark.parametrize(
    ("format", "secret"),
    [
        (FileFormat.DOCX, "blobfish"),
    ],
)
async def test_run_text_with_single_document(
    hosted_runtime: RuntimeHandle,
    format: FileFormat,
    secret: str,
):
    """Get the secret phrase from a single document"""
    # task
    runtime = hosted_runtime
    ExtractSecretPhrase = Block.new(
        BlockType.TEXT,
        "ExtractSecretPhrase",
        fields=[
            Field.input("Document", FileType.DOCUMENT),
            Field.output("SecretPhrase", str),
        ],
    )
    runtime.page().blocks.extend(ExtractSecretPhrase)

    # input file (from path relative to this file)
    text = f"The secret phrase is: '{secret}'"
    if format == FileFormat.DOCX:
        file_content = generate_docx_file(text)
    else:
        raise ValueError(f"unexpected format: {format!r}")

    file = await upload(file_content, f"test_text.{format.extension}")
    await runtime.commit()
    file._unload_rec()
    file._clear_cache()

    run = await runtime.run(ExtractSecretPhrase, inputs={"Document": file.to_ref()})
    assert run.outputs and run.outputs.SecretPhrase == secret


@pytest.mark.model()
async def test_run_text_with_multiple_documents(hosted_runtime: RuntimeHandle):
    """Provide multiple 'example' documents as context and another one as input. Ensure it's clear which is which."""
    runtime = hosted_runtime

    # task
    example_doc_1 = await upload(
        generate_docx_file("The secret phrase is: 'beluga'"), "example_doc_1.docx"
    )
    variable_1 = Block.new(
        BlockType.VALUE, "example_doc_1", value_type=to_type(File), value=example_doc_1
    )
    example_doc_2 = await upload(
        generate_docx_file("The secret phrase is: 'blobfish'"), "example_doc_2.docx"
    )
    variable_2 = Block.new(
        BlockType.VALUE, "example_doc_2", value_type=to_type(File), value=example_doc_2
    )
    GetSecretPhrase = Block.new(
        BlockType.TEXT,
        "GetSecretPhrase",
        text=md(
            "Get the secret phrase from the input document. If not mentioned, get it from example_doc_2"
        ),
        fields=[Field.input("Document", File), Field.output("SecretPhrase", str)],
    )
    runtime.page().blocks.extend(variable_1, variable_2, GetSecretPhrase)
    await runtime.commit()

    # run with document
    #  (to ensure that it's aliased properly)
    input_doc = await upload(generate_docx_file("The secret phrase is: 'zebra'"), "input_doc.docx")
    await runtime.commit()
    input_doc._clear_cache()
    input_doc._unload_rec()
    run = await runtime.run(GetSecretPhrase, inputs={"Document": input_doc.to_ref()})
    assert run.outputs and run.outputs.SecretPhrase == "zebra"

    # run without document
    #  (to ensure files in context are available)
    run = await runtime.run(GetSecretPhrase)
    assert run.outputs and run.outputs.SecretPhrase == "blobfish"
