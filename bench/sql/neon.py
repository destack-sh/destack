import abc
import asyncio
from typing import TYPE_CHECKING, Any, NamedTuple, Never

import aiohttp
import structlog

from bench.language.resource import Region
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Store


logger = structlog.get_logger(__name__)

CreateProjectRep = NamedTuple(
    "CreateProjectResponse", [("project_id", str), ("connection_uri", str)]
)


class NeonApi(abc.ABC):
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

    async def create_branch_with_rw_compute(
        self,
        *,
        name: str,
        parent_id: str,
        project_id: str,
    ) -> Never:
        """
        Creates a new branch with a read-write compute endpoint.
        """
        raise NotImplementedError


neon_client = NeonApi(
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
    log = logger.bind(store=store)
    start = asyncio.get_event_loop().time()
    assert store.external_name, f"{store!r} has no database"

    # create neon project
    logger.info("neon.create_project", store=store)
    neon_project = await neon_client.create_project(
        name=store.external_name, region=store.region, pg_version=16
    )
    store.external_id = neon_project.project_id
    store.connection_uri = neon_project.connection_uri

    log.info("pg.create_db", duration=asyncio.get_event_loop().time() - start, store=store)


async def migrate_pg_store(store: "Store") -> None:
    raise NotImplementedError("nocheckin: migrate_pg_store")
