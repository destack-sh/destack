import pytest

from bench.language import Region
from bench.system.plugin.neon import NEON_API_KEY, NEON_BASE_URL, NeonApiRemote


@pytest.mark.asyncio
async def test_neon_provisioner():
    neon_api = NeonApiRemote(url=NEON_BASE_URL, api_key=NEON_API_KEY)

    create_project_rep = None
    try:
        create_project_rep = await neon_api.create_project(
            name="bench-test",
            region=Region.FRANKFURT,
            pg_version=15,
            branch="main",
        )
        assert create_project_rep.project_id
        assert create_project_rep.sql_url
    finally:
        if create_project_rep and create_project_rep.project_id:
            await neon_api.delete_project(project_id=create_project_rep.project_id)
