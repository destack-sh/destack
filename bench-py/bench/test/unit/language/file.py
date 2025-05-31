import base64
import io

from PIL import Image

from bench.language import File, FileFormat, FileIn, FileType, extract_file_info, upload_file
from bench.test.simulation.core import Simulation
from bench.test.simulation.workload import RuntimeLambdaWorkload
from bench.test.unit.conftest import simulated_runtime

# some random image
image = Image.new("RGB", (32, 24), color="yellow")
image_bytes = io.BytesIO()
image.save(image_bytes, format="PNG")
IMAGE_BYTES = image_bytes.getvalue()
IMAGE_B64 = base64.b64encode(IMAGE_BYTES).decode("utf-8")
IMAGE_B64_URI = f"data:image/png;base64,{IMAGE_B64}"


async def _test_upload_and_download_file(
    runtime: RuntimeLambdaWorkload,
    file_content: FileIn,
    file_title: str,
    expected_file_type: FileType,
    expected_file_format: FileFormat | None,
):
    # upload
    file = await upload_file(file_content, name=file_title)
    await runtime.commit()
    assert file.type == expected_file_type
    assert file.format == expected_file_format

    # download
    file._clear_cache()
    assert file._cached_content is None
    await file.download()


@simulated_runtime()
async def test_markdown_file(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    await _test_upload_and_download_file(
        runtime,
        b"# it's a me\nmarkdown!",
        "test.md",
        FileType.TEXT,
        FileFormat.MARKDOWN,
    )


@simulated_runtime()
async def test_text_file(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    await _test_upload_and_download_file(
        runtime,
        b"hello world",
        "test.txt",
        FileType.TEXT,
        FileFormat.TXT,
    )


@simulated_runtime()
async def test_image_file(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    await _test_upload_and_download_file(
        runtime,
        IMAGE_BYTES,
        "test",
        FileType.IMAGE,
        FileFormat.PNG,
    )


@simulated_runtime()
async def test_extract_file_info_image(simulation: Simulation, runtime: RuntimeLambdaWorkload):
    file_info, _ = await extract_file_info(IMAGE_BYTES, name="image")
    assert file_info.mime_type == "image/png"
    assert file_info.size == len(IMAGE_BYTES)
    assert file_info.type == FileType.IMAGE
    assert file_info.format == FileFormat.PNG
    assert file_info.width == 32
    assert file_info.height == 24
    assert file_info.aspect_ratio == 32 / 24


@simulated_runtime()
async def test_file_from_url(simulation: Simulation, runtime: RuntimeLambdaWorkload):  # noqa: RUF029
    file = File.external(
        "https://en.wikipedia.org/wiki/ETH_Zurich#/media/File:ETH_Z%C3%BCrich_im_Abendlicht.jpg"
    )
    assert file.type == FileType.IMAGE
    assert file.format == FileFormat.JPEG
