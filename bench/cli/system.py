import base64
import json
from typing import Annotated

import structlog
import typer
from more_itertools import first
from rich import print

from bench.cli.utils import async_to_sync, parse_region
from bench.language import (
    BENCH_SLUG,
    CLOUD,
    REGION,
    SYSTEM_SLUG,
    Bench,
    Client,
    ClientType,
    Machine,
    NodeArea,
    NodeType,
    Region,
    ResourceStatus,
    User,
    UserStatus,
)
from bench.utils.env import ENV
from bench.utils.func import generate_access_token
from bench.utils.oracle import REAL_ORACLE

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


@app.command(help="create 'bench' and 'system' Benches (owned by 'system' User)")
@async_to_sync
async def bootstrap(region: Annotated[Region, typer.Option(parser=parse_region)] = REGION):
    from bench.system import (
        create_default_bench,
        global_session,
        global_store_from_env,
        pg_engine_from_store,
        regional_store_from_env,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    regional_store = regional_store_from_env(region=region)
    regional_pg_engine = pg_engine_from_store(
        f"pg-regional-{regional_store.region.name.lower()}", regional_store, NodeArea.REGIONAL
    )
    async with global_session(
        global_store, (global_pg_engine, regional_pg_engine), REAL_ORACLE, epoch=0
    ) as session:
        system_user = User(
            name="System",
            slug="system",
            email="system@bench.com",
            region=Region.ZURICH,
            status=UserStatus.REGISTERED,
        )
        session._create(system_user)
        await session.flush(optimistic=True)
        system_user.main_handle = system_user.handles.create(slug=SYSTEM_SLUG)
        system_bench = await create_default_bench(
            main_handle=system_user.main_handle,
            owner=system_user,
            region=region,
            global_store=global_store,
            session=session,
        )
        bench_bench_handle = system_user.handles.create(slug=BENCH_SLUG)
        bench_bench = await create_default_bench(
            main_handle=bench_bench_handle,
            owner=system_user,
            region=region,
            global_store=global_store,
            session=session,
        )
        logger.info(
            "system.bootstrap",
            system_user=system_user,
            system_bench=system_bench,
            bench_bench=bench_bench,
        )
        await session.commit()


@app.command(
    name="make-local-machine", help="gets or creates a local Machine (and Client) for a Bench"
)
@async_to_sync
async def make_local_machine(
    bench_slug: str,
    title: str = "Localhost",
    region: Annotated[Region, typer.Option(parser=parse_region)] = REGION,
):
    from bench.system import (
        ACCESS_TOKEN_LENGTH,
        global_session,
        global_store_from_env,
        pg_engine_from_store,
        regional_store_from_env,
    )

    global_store = global_store_from_env()
    global_pg_engine = pg_engine_from_store("pg-global", global_store, NodeArea.GLOBAL)
    regional_store = regional_store_from_env(region=region)
    regional_pg_engine = pg_engine_from_store(
        f"pg-regional-{regional_store.region.name.lower()}", regional_store, NodeArea.REGIONAL
    )
    async with global_session(
        global_store, (global_pg_engine, regional_pg_engine), REAL_ORACLE, epoch=0
    ) as session:
        bench = (
            await Bench.include_descendants(NodeType.MACHINE, NodeType.CLIENT)
            .select_all()
            .get(slug=bench_slug)
        )
        machines = await Machine.where(
            Machine.get_property("bench").eq(bench)
            & Machine.get_property("status").neq(ResourceStatus.DECOMMISSIONED)
        ).tolist()
        machine = first(machines, None)
        if machine is None:
            raise ValueError(f"{bench!r} has no machines")
        clients = (
            await Client.where(
                Client.get_property("parent").eq(machine)
                & Client.get_property("type").eq(ClientType.BENCH_MACHINE)
            )
            .select_all()
            .tolist()
        )
        client = first(clients, None)
        if client is None:
            client = Client(
                parent=bench,
                type=ClientType.BENCH_MACHINE,
                name=title,
                access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                machine=machine,
                seen_at=REAL_ORACLE.utc(),
            )
            session._create(client)

        client_env = {
            "BENCH_ID": str(bench.id),
            "MACHINE_ID": str(machine.id),
            "CLIENT_TYPE": str(int(client.type)),
            "CLIENT_ID": str(client.id),
            "CLIENT_ACCESS_TOKEN": client.access_token,
        }
        print("--- .env.dev.local ---")
        for k, v in client_env.items():
            print(f"{k}={v}")

        await session.commit()


@app.command(name="create-image-pull-secret", help="create image pull secret in local cluster")
@async_to_sync
async def create_image_pull_secret(*, ghcr_username: str, ghcr_token: str):
    from kubernetes_asyncio import client as k8
    from kubernetes_asyncio.client import CoreV1Api as KubernetesCoreV1Api

    from bench.system.provision.kubernetes import KUBERNETES_NAMESPACE, get_kubernetes_client

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
