from bench.language.file import upload
from bench.language.session import Session
from bench.proto.wire import HostClient


async def test_upload_file(real_session: Session, host: HostClient):
    await upload("# it's a me\nmarkdown!", title="test.md")
    await real_session.commit()
