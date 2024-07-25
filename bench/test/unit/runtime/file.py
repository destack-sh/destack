from bench.language.file import upload
from bench.language.session import Session


async def test_upload_file(real_session: Session):
    file = await upload("# it's a me\nmarkdown!", title="test.md")
    await real_session.commit()
