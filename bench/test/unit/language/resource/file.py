import io

import pytest
from PIL import Image

from bench.language import FileFormat, FileIn, FileType, extract_file_info, upload_file
from bench.test.unit.conftest import RuntimeHandle

# some random image
image = Image.new("RGB", (32, 24), color="yellow")
image_bytes = io.BytesIO()
image.save(image_bytes, format="PNG")
IMAGE_BYTES = image_bytes.getvalue()


@pytest.mark.parametrize(
    ("file_content", "file_title", "expected_file_type", "expected_file_format"),
    [
        (b"# it's a me\nmarkdown!", "test.md", FileType.TEXT, FileFormat.MARKDOWN),
        ("hello world", "test.txt", FileType.TEXT, FileFormat.TXT),
        (IMAGE_BYTES, "test", FileType.IMAGE, FileFormat.PNG),
    ],
)
async def test_upload_and_download_file(
    hosted_runtime: RuntimeHandle,
    file_content: FileIn,
    file_title: str,
    expected_file_type: FileType,
    expected_file_format: FileFormat | None,
):
    # upload
    file = await upload_file(file_content, name=file_title)
    await hosted_runtime.session.commit()
    assert file.type == expected_file_type
    assert file.format == expected_file_format

    # download
    file._clear_cache()
    assert file._cached_content is None
    await file.download()


async def test_extract_file_info_image(hosted_runtime: RuntimeHandle):
    file_info, _ = await extract_file_info(IMAGE_BYTES, name="image")
    assert file_info.mime_type == "image/png"
    assert file_info.size == len(IMAGE_BYTES)
    assert file_info.type == FileType.IMAGE
    assert file_info.format == FileFormat.PNG
    assert file_info.width == 32
    assert file_info.height == 24
    assert file_info.aspect_ratio == 32 / 24
