import abc
from typing import TYPE_CHECKING, Any, NamedTuple

import httpx
import structlog
from opentelemetry import trace

from bench.language import Region
from bench.utils.oracle import REAL_ORACLE
from bench.utils.tenacity import RetryOptions, retry
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    pass


logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class NeonCreateProjectRep(NamedTuple):
    project_id: str
    sql_url: str


class NeonCreateBranchRep(NamedTuple):
    branch_id: str
    compute_id: str
    sql_url: str


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
            sql_url=rep["sql_urls"][0]["sql_url"],
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
