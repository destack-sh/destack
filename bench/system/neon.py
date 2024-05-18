import abc
import asyncio
import re
from pathlib import Path
from typing import TYPE_CHECKING, Any, NamedTuple

import httpx
import structlog

from bench.language.bench import Region
from bench.sql.client import pg_cursor_to_store
from bench.sql.migration import has_migration_after, migrate
from bench.sql.schema import VERSION
from bench.utils.env import IS_DEBUG
from bench.utils.tenacity import RetryOptions, retry
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from bench.language import Bench, Store


logger = structlog.get_logger(__name__)

CreateProjectRep = NamedTuple(
    "CreateProjectResponse", [("project_id", str), ("connection_uri", str)]
)
CreateBranchRep = NamedTuple(
    "CreateBranchResponse", [("branch_id", str), ("compute_id", str), ("connection_uri", str)]
)

# NOTE: we assume throughout our Neon use that there will only be one endpoint per branch for now
#       and that the main branch for a project ('tenant') will always be called 'main'.


class NeonApi(abc.ABC):
    """
    Common Neon API so we can swap remote & local.
    """

    async def create_project(
        self, *, name: str, region: Region, pg_version: int
    ) -> CreateProjectRep:
        raise NotImplementedError

    async def delete_project(self, *, project_id: str) -> None:
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
    Neon API client wrapper for neon_local. Assumes that Neon has been set up locally at NEON_PATH.
    See https://github.com/neondatabase/neon#running-local-installation
    """

    def __init__(self, neon_path: str):
        self.neon_path: str = Path(neon_path).resolve().absolute().as_posix()

    async def _execute(self, command: str) -> str:
        process = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            cwd=self.neon_path,
        )
        stdout, stderr = await process.communicate()
        output = stdout.decode()
        if stderr:
            logger.error("neon.error", command=command, stderr=stderr.decode())
        else:
            logger.trace("neon.local", command=command, output=output)
        return output

    async def create_project(
        self, *, name: str, region: Region, pg_version: int
    ) -> CreateProjectRep:
        # create tenant, output should look like
        #  > ...
        #  > tenant 9ef87a5bf0d92544f6fafeeb3239695c successfully created on the pageserver
        #  > ...
        output = await self._execute("cargo neon tenant create")
        tenant_match = re.search(r"tenant ([a-f0-9]+) successfully created", output)
        assert tenant_match, f"tenant create failed: {output}"
        tenant_id = tenant_match.group(1)

        # create endpoint
        endpoint_name = f"{tenant_id}-main"
        output = await self._execute(
            f"cargo neon endpoint create {endpoint_name} --tenant-id {tenant_id}"
        )

        # start endpoint, output should look like
        # > ...
        # > Starting existing endpoint main...
        # > Starting postgres node at 'postgresql://cloud_admin@127.0.0.1:55436/postgres'
        # > ...
        output = await self._execute(f"cargo neon endpoint start {endpoint_name}")
        connection_uri_match = re.search(r"Starting postgres node at '([^']+)'", output)
        assert connection_uri_match, f"endpoint start failed: {output}"
        connection_uri = connection_uri_match.group(1)

        return CreateProjectRep(project_id=tenant_id, connection_uri=connection_uri)

    async def _ensure_branch_endpoint(self, *, project_id: str, branch_name="main") -> str:
        # check if endpoint is up, output looks like
        # > ...
        # >  ENDPOINT   ADDRESS          TIMELINE                          BRANCH NAME  LSN        STATUS
        # >  ep-main    127.0.0.1:55436  4c32574ffb5439346b403c717ad6f74b  main         0/14B8340  stopped
        # >  test-main  127.0.0.1:55440  4c32574ffb5439346b403c717ad6f74b  main         0/14B8340  running
        # > ...
        output = await self._execute(f"cargo neon endpoint list --tenant-id {project_id}")
        endpoint_name = f"{project_id}-{branch_name}"
        # attempt to parse out address for endpoint (just the address part)
        endpoint_match = re.search(rf"\s*{endpoint_name}\s+([\d\.:]+)\s+", output, re.MULTILINE)
        if endpoint_match is not None:  # found it!
            address = endpoint_match.group(1)
            # yeah the could_admin/postgres stuff seems hardcoded in neon_local somewhere
            return f"postgresql://cloud_admin@{address}/postgres"

        # nope, start endpoint
        output = await self._execute(f"cargo neon endpoint start {project_id}-{branch_name}")
        connection_uri_match = re.search(r"Starting postgres node at '([^']+)'", output)
        assert connection_uri_match, f"endpoint start failed: {output}"
        connection_uri = connection_uri_match.group(1)
        return connection_uri

    async def delete_project(self, *, project_id: str) -> None:
        # api is ignored but must be passed
        _ = await self._execute(
            f"cargo run --bin=storcon_cli -- --api=http://localhost:1234 tenant-delete --tenant-id={project_id}"
        )


NEON_REGION_BY_REGION: dict[Region, str] = {
    Region.EUROPE_CENTRAL: "aws-eu-central-1",
}
NEON_MAIN_ENDPOINT_SETTINGS = {
    "autoscaling_limit_min_cu": 0.25,
    "autoscaling_limit_max_cu": 4,
    "suspend_timeout_seconds": 600,
}
NEON_BRANCH_ENDPOINT_SETTINGS = {
    "autoscaling_limit_min_cu": 0.25,
    "autoscaling_limit_max_cu": 4,
    "suspend_timeout_seconds": 600,
}


class RecoverableError(RuntimeError):
    pass


class UnrecoverableError(RuntimeError):
    pass


class NeonApiRemote(NeonApi):
    """Neon API client for the remote API."""

    def __init__(self, *, url: str, api_key: str):
        if url.endswith("/"):
            url = url[:-1]
        self.url = url
        self.api_key = api_key

    @retry(RetryOptions(max_attempts=5, retry_interval=2, backoff=1.5, retry_on=RecoverableError))
    async def _request(
        self,
        method: str,
        path: str,
        params: dict[str, Any] | None = None,
        json: dict[str, Any] | None = None,
    ) -> Any | None:
        async with httpx.AsyncClient() as client:
            headers = {"Authorization": f"Bearer {self.api_key}"}
            logger.debug("neon.request", method=method, path=path, params=params, json=json)
            response = await client.request(
                method, f"{self.url}/{path}", headers=headers, params=params, json=json
            )
            if response.status_code > 400:
                error = dict(
                    path=path,
                    params=params,
                    json=json,
                    status=response.status_code,
                    text=response.text,
                )
                if response.status_code == 404:
                    raise UnrecoverableError(f"not found: {error}")
                elif response.status_code == 409:
                    raise UnrecoverableError(f"conflict: {error}")
                elif response.status_code == 422:
                    raise UnrecoverableError(f"unprocessable: {error}")
                else:
                    raise RecoverableError(f"request failed: {response.status_code} {error}")

            rep = response.json()
            logger.debug("neon.response", status=response.status_code, json=rep)
            return rep

    async def create_project(
        self, *, name: str, region: Region, pg_version: int, branch: str = "main"
    ) -> CreateProjectRep:
        project = {
            "name": name,
            "region_id": NEON_REGION_BY_REGION[region],
            "pg_version": pg_version,
            "branch": {"name": branch, "role_name": "bench", "database_name": "bench"},
            "provisioner": "k8s-neonvm",
            "default_endpoint_settings": NEON_MAIN_ENDPOINT_SETTINGS,
            "history_retention_seconds": 7 * 24 * 60 * 60,
        }
        rep = await self._request("POST", "projects", json={"project": project})
        assert rep is not None, "no response"
        return CreateProjectRep(
            project_id=rep["project"]["id"],
            connection_uri=rep["connection_uris"][0]["connection_uri"],
        )

    async def delete_project(self, *, project_id: str) -> None:
        await self._request("DELETE", f"projects/{project_id}")


NEON_LOCAL = get_from_env("NEON_LOCAL", default=IS_DEBUG, typ=bool)
if NEON_LOCAL:
    neon_client = NeonApiLocal(neon_path=get_from_env("NEON_PATH"))
else:
    neon_client = NeonApiRemote(
        url=get_from_env("NEON_BASE_URL"),
        api_key=get_from_env("NEON_API_KEY"),
    )


async def prepare_local_stores(bench: "Bench") -> None:
    assert isinstance(neon_client, NeonApiLocal), f"not in local mode (client={neon_client!r})"
    for store in bench.stores:
        if store.external_id is not None:
            connection_uri = await neon_client._ensure_branch_endpoint(project_id=store.external_id)
            if store.connection_uri != connection_uri:
                store.connection_uri = connection_uri


async def create_local_store(store: "Store") -> None:
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


async def migrate_local_store(store: "Store") -> None:
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


async def delete_local_store(store: "Store") -> None:
    assert store.external_id, f"{store!r} has no external_id"

    # delete neon project
    start = asyncio.get_event_loop().time()
    await neon_client.delete_project(project_id=store.external_id)
    duration = asyncio.get_event_loop().time() - start
    logger.info("neon.delete_project", store=store, duration=duration)
