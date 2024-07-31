import pytest
from PIL import Image

from bench.language.block import Block
from bench.language.const import BlockType, RunStatus
from bench.language.field import Field
from bench.language.file import File, FileType, upload
from bench.language.run import ModelOptions, ModelProvider, RunErrorType, RunOptions
from bench.language.text import md
from bench.language.validation import constraint
from bench.test.unit.conftest import RuntimeHandle

TEST_MODEL_PROVIDERS = (
    ModelProvider.OPENAI,
    ModelProvider.ANTHROPIC,
)


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
async def test_run_text_with_images(hosted_runtime: RuntimeHandle, model_provider: ModelProvider):
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

    red_image = Image.new("RGB", (320, 240), color="red")
    image_file = await upload(red_image, "red.png")
    await runtime.commit()
    image_file._unload_rec()
    image_file._clear_cache()

    await runtime.run(DetectColor, inputs={"Image": image_file.to_ref()})
