import io

import pytest
from PIL import Image

from bench.language.file import FileFormat, FileIn, FileType, upload
from bench.test.unit.runtime.conftest import RuntimeHandle

# some random image
image = Image.new("RGB", (32, 32), color="yellow")
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
    file = await upload(file_content, title=file_title)
    await hosted_runtime.session.commit()
    assert file.coarse_type == expected_file_type
    assert file.format == expected_file_format

    # download
    file.clear_cache()
    assert file._cached_content is None
    await file.download()
