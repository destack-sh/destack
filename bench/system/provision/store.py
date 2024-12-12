import abc
from typing import TYPE_CHECKING, Any, NamedTuple, override

import httpx
import structlog
from opentelemetry import trace

from bench.language import Bench, Drive, ResourceStatus, Store
from bench.language.bench import Region
from bench.language.const import NodeType
from bench.sql.client import pg_connection
from bench.sql.engine import sqlstr
from bench.sql.migration import sql_migrate
from bench.system.host.core import Host
from bench.system.provision.provisioner import Provisioner
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.func import bittuple
from bench.utils.oracle import REAL_ORACLE
from bench.utils.tenacity import RetryOptions, retry
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass


if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class StoreProvisioner(Provisioner[Store, Store]):
    """Base for provisioning Stores."""

    watch_types = bittuple(NodeType.STORE)
    resource_type = NodeType.STORE

    async def _do_migrate(self, resource: Store):
        """Migrate the store to its indicated 'version'."""
        assert resource.version, f"{resource!r} has no version"
        async with pg_connection(resource) as conn:
            await sql_migrate(
                conn.cursor,
                target=resource.version,
                is_global=False,
                store=resource,
                oracle=self.host.oracle,
            )
            await conn.commit()
        async with self.host.session(commit=True):
            resource.version = resource.target_version

    @override
    async def _do_update(self, resource: Store):
        # auto-migrate if version changed
        if resource.version != resource.target_version:
            await self._do_migrate(resource)


class NeonStoreProvisioner(StoreProvisioner):
    """Provision Stores with the Neon API."""

    def __init__(self, host: "Host", bench: Bench, neon_api: "NeonApi"):
        super().__init__(host, bench)
        self._neon_api = neon_api

    @override
    async def _do_provision(self, resource: Store):
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create postgres database ('project')
        neon_project = await self._neon_api.create_project(
            name=resource.external_name, region=resource.region, pg_version=16
        )
        async with self.host.session(commit=True):
            resource.external_id = neon_project.project_id
            resource.connection_uri = neon_project.connection_uri
            resource.status = ResourceStatus.UP
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        assert resource.external_id, f"{resource!r} has no external ID"
        await self._neon_api.delete_project(project_id=resource.external_id)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class LocalhostStoreProvisioner(StoreProvisioner):
    """Provision Stores as local Postgres databases (in the existing database)."""

    @override
    async def _do_provision(self, resource: Store):
        assert IS_DEV or IS_TEST, f"cannot create localhost store in environment: {ENV!r}"
        # assign a name
        if resource.external_name is None:
            assert resource.bench_id, f"{resource!r} has no bench"
            async with self.host.session(commit=True):
                resource.external_name = f"{ENV}-{resource.bench_id}"
        # create database through existing connection
        # (use same postgres instance as global store)
        async with pg_connection(self.host.global_store, autocommit=True) as conn:
            await conn.execute(sqlstr(f'CREATE DATABASE "{resource.external_name}"'))
        async with self.host.session(commit=True):
            connection_uri = self.host.global_store.connection_uri
            assert connection_uri, f"{self.host.global_store!r} has no connection URI"
            resource.connection_uri = f"{connection_uri.rsplit('/', 1)[0]}/{resource.external_name}"
            resource.status = ResourceStatus.UP
        # migrate it immediately
        await self._do_migrate(resource)

    @override
    async def _do_decommission(self, resource: Store):
        # drop database through existing connection
        async with pg_connection(self.host.global_store, autocommit=True) as conn:
            await conn.execute(sqlstr(f'DROP DATABASE "{resource.external_name}"'))


class S3DriveProvisioner(Provisioner[Drive, Drive]):
    """Provision Drives with an S3-compatible API."""

    watch_types = bittuple(NodeType.DRIVE)
    resource_type = NodeType.DRIVE

    # NOTE: don't actually need to do anything since we share drives between Benches (per region)

    @override
    async def _do_provision(self, resource: Drive):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Drive):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class NeonCreateProjectRep(NamedTuple):
    project_id: str
    connection_uri: str


class NeonCreateBranchRep(NamedTuple):
    branch_id: str
    compute_id: str
    connection_uri: str


# NOTE: we assume throughout our Neon use that there will only be one endpoint per branch for now
#       and that the main branch for a project ('tenant') will always be called 'main'.


class NeonApi(abc.ABC):
    """
    Common Neon API so we can swap remote & local.
    """

    @abc.abstractmethod
    async def create_project(
        self, *, name: str, region: Region, pg_version: int
    ) -> NeonCreateProjectRep: ...

    @abc.abstractmethod
    async def delete_project(self, *, project_id: str) -> None: ...

    @abc.abstractmethod
    async def create_branch_with_rw_compute(
        self,
        *,
        name: str,
        parent_id: str,
        project_id: str,
    ) -> NeonCreateBranchRep: ...


NEON_REGION_BY_REGION: dict[Region, str] = {
    Region.FRANKFURT: "aws-eu-central-1",
    Region.VIRGINIA: "aws-us-east-1",
    Region.OHIO: "aws-us-east-2",
    Region.OREGON: "aws-us-west-2",
    Region.SINGAPORE: "aws-ap-southeast-1",
    Region.SYDNEY: "aws-ap-southeast-2",
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


class NeonRecoverableError(RuntimeError):
    pass


class NeonUnrecoverableError(RuntimeError):
    pass


class NeonApiRemote(NeonApi):
    """Neon API client for the remote API."""

    def __init__(self, *, url: str, api_key: str):
        if url.endswith("/"):
            url = url[:-1]
        self.url = url
        self.api_key = api_key

    @retry(
        RetryOptions(max_attempts=5, retry_interval=2, backoff=1.5, retry_on=NeonRecoverableError),
        oracle=REAL_ORACLE,
    )
    async def _request(
        self,
        method: str,
        path: str,
        params: dict[str, Any] | None = None,
        json: dict[str, Any] | None = None,
    ) -> Any | None:
        async with httpx.AsyncClient() as client:
            headers = {"Authorization": f"Bearer {self.api_key}"}
            logger.trace("neon.request", method=method, path=path, params=params, json=json)
            response = await client.request(
                method, f"{self.url}/{path}", headers=headers, params=params, json=json
            )
            if response.status_code > 400:
                error = {
                    "path": path,
                    "params": params,
                    "json": json,
                    "status": response.status_code,
                    "text": response.text,
                }
                if response.status_code == 404:
                    raise NeonUnrecoverableError(f"not found: {error}")
                elif response.status_code == 409:
                    raise NeonUnrecoverableError(f"conflict: {error}")
                elif response.status_code == 422:
                    raise NeonUnrecoverableError(f"unprocessable: {error}")
                else:
                    raise NeonRecoverableError(f"request failed: {response.status_code} {error}")

            rep = response.json()
            logger.trace("neon.response", status=response.status_code, json=rep)
            return rep

    async def create_project(
        self, *, name: str, region: Region, pg_version: int, branch: str = "main"
    ) -> NeonCreateProjectRep:
        region_id = NEON_REGION_BY_REGION.get(region)
        assert region_id, f"unsupported neon region: {region}"
        project = {
            "name": name,
            "region_id": region_id,
            "pg_version": pg_version,
            "branch": {"name": branch, "role_name": "bench", "database_name": "bench"},
            "provisioner": "k8s-neonvm",
            "default_endpoint_settings": NEON_MAIN_ENDPOINT_SETTINGS,
            "history_retention_seconds": 7 * 24 * 60 * 60,
        }
        rep = await self._request("POST", "projects", json={"project": project})
        assert rep is not None, "no response"
        return NeonCreateProjectRep(
            project_id=rep["project"]["id"],
            connection_uri=rep["connection_uris"][0]["connection_uri"],
        )

    async def delete_project(self, *, project_id: str) -> None:
        await self._request("DELETE", f"projects/{project_id}")

    async def create_branch_with_rw_compute(
        self,
        *,
        name: str,
        parent_id: str,
        project_id: str,
    ) -> NeonCreateBranchRep:
        raise NotImplementedError


NEON_BASE_URL = get_from_env("NEON_BASE_URL", description="Full URL for Neon API")
NEON_API_KEY = get_from_env("NEON_API_KEY", description="Neon API key")
neon_api = NeonApiRemote(url=NEON_BASE_URL, api_key=NEON_API_KEY)
