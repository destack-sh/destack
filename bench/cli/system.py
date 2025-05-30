import base64
import json
from typing import TYPE_CHECKING, Annotated

import structlog
import typer

from bench.language import REGION, Region, Session
from bench.utils.env import ENV

from .utils import async_to_sync, parse_region

if TYPE_CHECKING:
    pass

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="create builtin stuff (User, Benches, etc.)")
@async_to_sync
async def bootstrap(
    region: Annotated[Region, typer.Option(parser=parse_region)], upsert: bool = False
):
    from bench.language import NodeArea
    from bench.system import (
        PostgresStore,
        create_system_benches,
        get_global_database_from_env,
        get_main_database_from_env,
    )

    global_database = get_global_database_from_env()
    main_database = get_main_database_from_env(region=region)
    store = PostgresStore(
        {NodeArea.GLOBAL_POSTGRES: global_database, NodeArea.MAIN_POSTGRES: main_database}
    )
    async with Session(store=store) as session:
        await create_system_benches(region=region, session=session, upsert=upsert)
        await session.commit()


@app.command(
    name="make-local-machine-runtime",
    help="gets or creates a local runtime Machine (and Client) for a Bench",
)
@async_to_sync
async def make_local_machine_runtime(
    bench_slug: str,
    title: str = "Local Runtime Machine",
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    local_machine_url: str = "http://localhost:60062",
):
    raise NotImplementedError


@app.command(name="create-image-pull-secret", help="create image pull secret in local cluster")
@async_to_sync
async def create_image_pull_secret(*, ghcr_username: str, ghcr_token: str):
    from kubernetes_asyncio import client as k8
    from kubernetes_asyncio.client import CoreV1Api as KubernetesCoreV1Api

    from bench.language import CLOUD, REGION
    from bench.system.plugin.kubernetes import KUBERNETES_NAMESPACE, get_kubernetes_client

    kubernetes_api = await get_kubernetes_client()
    kubernetes_core_api = KubernetesCoreV1Api(kubernetes_api)
    namespace = KUBERNETES_NAMESPACE

    docker_config = {
        "auths": {
            "ghcr.io": {"auth": base64.b64encode(f"{ghcr_username}:{ghcr_token}".encode()).decode()}
        }
    }

    secret_name = f"bench-{ENV.slug}-{CLOUD.slug}-{REGION.slug}-image-pull-secret"
    secret = k8.V1Secret(
        api_version="v1",
        kind="Secret",
        metadata=k8.V1ObjectMeta(name=secret_name, namespace=namespace),
        type="kubernetes.io/dockerconfigjson",
        data={".dockerconfigjson": base64.b64encode(json.dumps(docker_config).encode()).decode()},
    )

    try:
        await kubernetes_core_api.create_namespaced_secret(namespace=namespace, body=secret)  # type: ignore
        logger.info("secret.create", secret_name=secret_name, namespace=namespace)
    except k8.ApiException as e:
        if e.status == 409:
            logger.info("secret.update.attempt", secret_name=secret_name, namespace=namespace)
            await kubernetes_core_api.patch_namespaced_secret(
                name=secret_name, namespace=namespace, body=secret
            )
            logger.info("secret.update.success", secret_name=secret_name, namespace=namespace)
        else:
            logger.error(
                "secret.create.error", secret_name=secret_name, namespace=namespace, error=str(e)
            )
            raise
