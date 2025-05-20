import base64
import json
from typing import TYPE_CHECKING, Annotated

import structlog
import typer
from more_itertools import first
from rich import print

from bench.language.core.const import REGION, Region
from bench.utils.env import ENV
from bench.utils.func import generate_access_token
from bench.utils.oracle import REAL_ORACLE

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
        create_system_benches,
        global_database_from_env,
        pg_engine_from_database,
        regional_database_from_env,
    )

    global_database = global_database_from_env()
    global_pg_engine = pg_engine_from_database("pg-global", global_database, NodeArea.GLOBAL_DB)
    regional_database = regional_database_from_env(region=region)
    regional_pg_engine = pg_engine_from_database(
        f"pg-regional-{regional_database.region.name.lower()}",
        regional_database,
        NodeArea.REGIONAL_DB,
    )

    await create_system_benches(
        region=region,
        global_database=global_database,
        global_pg_engine=global_pg_engine,
        regional_database=regional_database,
        regional_pg_engine=regional_pg_engine,
        upsert=upsert,
    )


@app.command(
    name="make-local-computer-runtime",
    help="gets or creates a local runtime Computer (and Client) for a Bench",
)
@async_to_sync
async def make_local_computer_runtime(
    bench_slug: str,
    title: str = "Local Runtime Computer",
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
    local_computer_url: str = "http://localhost:60062",
):
    from bench.language import (
        Bench,
        Client,
        ClientType,
        Computer,
        ComputerType,
        NodeArea,
        NodeType,
        ResourceStatus,
    )
    from bench.system import (
        ACCESS_TOKEN_LENGTH,
        global_database_from_env,
        global_session,
        pg_engine_from_database,
        regional_database_from_env,
    )

    global_database = global_database_from_env()
    global_pg_engine = pg_engine_from_database("pg-global", global_database, NodeArea.GLOBAL_DB)
    regional_database = regional_database_from_env(region=region)
    regional_pg_engine = pg_engine_from_database(
        f"pg-regional-{regional_database.region.name.lower()}",
        regional_database,
        NodeArea.REGIONAL_DB,
    )
    async with global_session(
        global_database, (global_pg_engine, regional_pg_engine), REAL_ORACLE, epoch=0
    ) as session:
        bench = await Bench.include_descendants(NodeType.CLIENT).select_all().get(slug=bench_slug)
        computers = await Computer.where(
            Computer.property("bench").eq(bench)
            & Computer.property("type").eq(ComputerType.RUNTIME)
            & Computer.property("status").neq(ResourceStatus.OFFLINE)
        ).to_list()
        computer = first(computers, None)
        if computer is None:
            raise ValueError(f"{bench!r} has no runtime computers")
        clients = (
            await Client.where(
                Client.property("parent").eq(bench)
                & Client.property("computer").eq(computer)
                & Client.property("type").eq(ClientType.COMPUTER)
            )
            .select_all()
            .to_list()
        )
        client = first(clients, None)
        if client is None:
            client = Client(
                parent=bench,
                type=ClientType.COMPUTER,
                name=title,
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                computer=computer,
                seen_at=REAL_ORACLE.utc(),
            )
            session._create(client)
        computer.client = client
        computer.update_status(ResourceStatus.AVAILABLE)
        computer.grpc_url = local_computer_url

        client_env = {
            "BENCH_ID": str(bench.id),
            "COMPUTER_ID": str(computer.id),
            "CLIENT_TYPE": str(int(client.type)),
            "CLIENT_ID": str(client.id),
            "CLIENT_ACCESS_TOKEN": client.access_token,
        }
        print("----------------------")
        print(f"Computer: {computer!r}")
        print(f"Client: {client!r}")
        print("--- .env.dev.local ---")
        for k, v in client_env.items():
            print(f"{k}={v}")

        await session.commit()


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
