import abc
import asyncio
import re
from typing import TYPE_CHECKING, Any, NamedTuple

import aiohttp
import structlog

from bench.language.resource import Region
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import has_migration_after, migrate
from bench.sql.schema import VERSION
from bench.utils.env import IS_DEBUG
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Store


logger = structlog.get_logger(__name__)

CreateProjectRep = NamedTuple(
    "CreateProjectResponse", [("project_id", str), ("connection_uri", str)]
)
CreateBranchRep = NamedTuple(
    "CreateBranchResponse", [("branch_id", str), ("compute_id", str), ("connection_uri", str)]
)


class NeonApi(abc.ABC):
    """
    Common Neon API so we can swap remote & local.
    """

    async def create_project(
        self, *, name: str, region: Region, pg_version: int
    ) -> CreateProjectRep:
        raise NotImplementedError

    async def create_branch_with_rw_compute(
        self,
        *,
        name: str,
        parent_id: str,
        project_id: str,
    ) -> CreateBranchRep:
        raise NotImplementedError


class NeonApiLocal(NeonApi):
    """
    Neon API client wrapper for neon_local.
    Assumes that Neon has been set up locally (should run in our dev docker compose).
    """

    async def _execute(self, command: str) -> str:
        # TODO :Test! :Robustness: put neon in docker compose for testing/development
        # run the command with 'cargo neon <command>' in '~/neon'
        process = await asyncio.create_subprocess_shell(
            f"cargo neon {command}",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd="~/neon",
        )
        stdout, stderr = await process.communicate()
        output = stdout.decode()
        if stderr:
            logger.error("neon.error", command=command, stderr=stderr.decode())
        return output

    async def create_project(
        self, *, name: str, region: Region, pg_version: int
    ) -> CreateProjectRep:
        # create tenant, output should look like
        #  > ...
        #  > tenant 9ef87a5bf0d92544f6fafeeb3239695c successfully created on the pageserver
        #  > ...
        output = await self._execute("tenant create")
        tenant_match = re.search(r"tenant ([a-f0-9]+) successfully created", output)
        assert tenant_match, f"tenant create failed: {output}"
        tenant_id = tenant_match.group(1)

        # create endpoint
        output = await self._execute(f"endpoint create main --tenant-id {tenant_id}")
        # start endpoint, output should look like
        # > ...
        # > Starting existing endpoint main...
        # > Starting postgres node at 'postgresql://cloud_admin@127.0.0.1:55436/postgres'
        # > ...
        output = await self._execute(f"endpoint start main --tenant-id {tenant_id}")
        connection_uri_match = re.search(r"Starting postgres node at '([^']+)'", output)
        assert connection_uri_match, f"endpoint start failed: {output}"
        connection_uri = connection_uri_match.group(1)

        return CreateProjectRep(project_id=tenant_id, connection_uri=connection_uri)


class NeonApiRemote(NeonApi):
    """Neon API client for the remote API."""

    def __init__(self, *, url: str, api_key: str):
        if url.endswith("/"):
            url = url[:-1]
        self.url = url
        self.api_key = api_key

    async def _request(
        self,
        method: str,
        path: str,
        params: dict[str, Any] | None = None,
        json: dict[str, Any] | None = None,
    ) -> Any | None:
        """
        Make a request to the Neon API, returns the json (if any).
        """
        async with aiohttp.ClientSession() as session:
            headers = {"Authorization": f"Bearer {self.api_key}"}
            logger.debug("neon.request", method=method, path=path, params=params, json=json)
            async with session.request(
                method, f"{self.url}/{path}", headers=headers, params=params, json=json
            ) as response:
                response.raise_for_status()
                rep = await response.json()
                logger.debug("neon.response", status=response.status, json=rep)
                return rep

    async def create_project(
        self, *, name: str, region: Region, pg_version: int, branch: str = "main"
    ) -> CreateProjectRep:
        """
        Create a new Neon project.
        """
        project = {
            "name": name,
            "region_id": NEON_REGION_BY_REGION[region],
            "pg_version": pg_version,
            "branch": {"name": branch},
        }
        rep = await self._request("POST", "projects", json={"project": project})
        assert rep is not None, "no response"
        return CreateProjectRep(
            project_id=rep["project"]["id"],
            connection_uri=rep["connection_uris"][0]["connection_uri"],
        )


if get_from_env("NEON_LOCAL", default=not IS_DEBUG):
    neon_client = NeonApiLocal()
else:
    neon_client = NeonApiRemote(
        url=get_from_env("NEON_BASE_URL"),
        api_key=get_from_env("NEON_API_KEY"),
    )

NEON_REGION_BY_REGION: dict[Region, str] = {
    Region.EUROPE_CENTRAL: "aws-eu-central-1",
}


async def create_local_pg_store(store: "Store") -> None:
    """
    Creates a new 'local' Neon-based Postgres database and corresponding roles/user for a Bench.
    """
    assert store.external_name, f"{store!r} has no database"

    # create neon project
    start = asyncio.get_event_loop().time()
    neon_project = await neon_client.create_project(
        name=store.external_name, region=store.region, pg_version=16
    )
    store.external_id = neon_project.project_id
    store.connection_uri = neon_project.connection_uri
    duration = asyncio.get_event_loop().time() - start
    logger.info("neon.create_project", store=store, duration=duration)


async def migrate_local_pg_store(store: "Store") -> None:
    """Migrates the store to the latest version of our internal schema."""
    if store.version is not None and not has_migration_after(store.version, is_global=False):
        logger.debug("neon.migrate.skip", store=store)
        return  # nothing to do
    start = asyncio.get_event_loop().time()
    async with pg_cursor_to_store(store) as cur:
        await migrate(cur, target=VERSION, is_global=False, store=store)
        await cur.connection.commit()
    store.version = VERSION
    duration = asyncio.get_event_loop().time() - start
    logger.info("neon.migrate", store=store, duration=duration)
