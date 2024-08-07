import random
from pathlib import Path
from typing import TYPE_CHECKING, cast, override

import docker
import docker.models
import docker.models.containers
import structlog
from kubernetes_asyncio import client as k8
from kubernetes_asyncio.client import ApiClient as KubernetesApiClient
from kubernetes_asyncio.client import CoreV1Api as KubernetesCoreV1Api
from opentelemetry import trace

from bench.language import Bench, Client, Machine, ResourceStatus, Server
from bench.language.const import CLOUD, ClientType, NodeType, dockerify_domain
from bench.system.access import ACCESS_TOKEN_LENGTH
from bench.system.core import Commit, HostApi
from bench.system.provisioner import Provisioner
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV
from bench.utils.func import bittuple, generate_access_token
from bench.utils.utils import get_from_env, get_from_env_maybe

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


class ElasticServerProvisioner(Provisioner[Server, Server | Machine]):
    """Provision Servers by creating/deleting/scaling Machines (and their Clients) on-demand."""

    watch_types = bittuple(NodeType.SERVER, NodeType.MACHINE)
    provision_types = bittuple(NodeType.SERVER)

    async def _reconcile(self, server: Server):
        machines = server.machines.tolist()

        # "rescale server" (just ensure a single machine exists for now)
        # TODO :Incomplete!: scale ElasticServerProvisioner properly (up/down/sleep/...)
        if not machines:
            async with self.host.session(commit=True) as session:
                client = Client(
                    parent=server,
                    type=ClientType.BENCH_MACHINE,
                    name="Machine1",
                    access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                )
                session._create(client)
                await session.flush()
                machine = Machine(name="Machine1", cpu=0.25, ram=0.5, client=client)
                server.machines.append(machine)
                server.status = ResourceStatus.PROVISIONING
                client.machine = machine

        # update server status to reflect machines (if needed)
        if machines and all(m.status == ResourceStatus.HEALTHY for m in machines):
            actual_status = ResourceStatus.HEALTHY
        elif machines and any(m.status == ResourceStatus.UNHEALTHY for m in machines):
            actual_status = ResourceStatus.UNHEALTHY
        else:
            actual_status = ResourceStatus.HEALTHY  # not sure?
        if server.status != actual_status:
            async with self.host.session(commit=True):
                server.status = actual_status

    @override
    async def _do_on_commit_deferred(self, commit: Commit[Server | Machine]) -> None:
        # find any changed servers (directly or indirectly via machines)
        servers: set[Server] = set()
        for node in commit.edited:
            if isinstance(node, Server):
                servers.add(node)
            elif isinstance(node, Machine):
                servers.add(cast(Server, node.parent))
            else:
                raise TypeError(f"unexpected node type {type(node)}")

        # and check/update them
        for server in servers:
            if server.status != ResourceStatus.DECOMMISSIONED:
                await self._reconcile(server)

    @override
    async def _do_provision(self, resource: Server):
        await self._reconcile(resource)

    @override
    async def _do_update(self, resource: Server):
        await self._reconcile(resource)

    @override
    async def _do_decommission(self, resource: Server):
        # nothing special, child machines are automatically removed too
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


MACHINE_RUNTIME_IMAGE = get_from_env(
    "MACHINE_RUNTIME_IMAGE", description="Runtime container image for machine"
)


def _get_machine_env_vars(machine: Machine, *, is_trusted: bool, is_docker: bool) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    client = machine.client
    assert client, f"{machine!r} has no client"
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_docker:
        supervisor_url = dockerify_domain(supervisor_url)
    env_vars: dict[str, str | None] = {
        # hosting
        "SERVICE_NAME": "runtime",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": machine.region.slug,
        "SUPERVISOR_URL": supervisor_url,
        # bench
        "BENCH_ID": str(machine.bench.id),
        "SERVER_ID": str(machine.parent.id) if isinstance(machine.parent, Server) else None,
        "MACHINE_ID": str(machine.id),
        "CLIENT_ID": str(client.id),
        "CLIENT_TYPE": str(client.type.value),
        "CLIENT_ACCESS_TOKEN": client.access_token,
        # config
        "TRACING": "0",
        "LOG_LEVEL": "DEBUG",
        "LOG_MODE": "JSON",
    }
    if is_docker:
        env_vars["IS_IN_DOCKER"] = "1"
    if SENTRY_DSN:
        env_vars["SENTRY_DSN"] = SENTRY_DSN
    if is_trusted:
        # add secret api keys directly (for testing/development)
        env_vars["OPENAI_API_KEY"] = get_from_env("OPENAI_API_KEY", description="OpenAI API key")
        env_vars["ANTHROPIC_API_KEY"] = get_from_env(
            "ANTHROPIC_API_KEY", description="Anthropic API key"
        )
    return {k: v for k, v in env_vars.items() if v}


class LocalhostMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines by short-circuiting to localhost."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench):
        super().__init__(host, bench)
        self._local_machine_url = get_from_env_maybe(
            "LOCAL_MACHINE_URL", description="URL for local machine runtime"
        )

    @override
    async def _do_provision(self, resource: Machine):
        async with self.host.session(commit=True):
            resource.connection_uri = self._local_machine_url
            resource.status = ResourceStatus.HEALTHY

    @override
    async def _do_decommission(self, resource: Machine):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class DockerMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as containers in a Docker installation."""

    # NOTE :DevX: use async docker api (instead of blocking sync)
    # TODO :Test: test docker machine provisioner

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench):
        super().__init__(host, bench)
        self._docker_client = docker.from_env()

    @override
    async def _do_start(self) -> None:
        servers = self.bench.servers.tolist()
        machines = [m for s in servers for m in s.machines]
        containers: list[docker.models.containers.Container] = self._docker_client.containers.list(
            all=True
        )
        containers_by_id = {cast(str, c.id): c for c in containers}

        async with self.host.session(commit=True):
            for machine in machines:
                if machine.external_id is None:
                    continue
                container = containers_by_id.get(machine.external_id)
                if container is None:
                    machine.status = ResourceStatus.DECLARED

    @override
    async def _do_provision(self, resource: Machine):
        project_dir = Path(__file__).parent.parent.parent
        assert project_dir.exists() and project_dir.name == "bench", f"{project_dir!r}"
        bench_dir = (project_dir / "bench").absolute().as_posix()
        assigned_port = random.randint(60100, 65000)
        env_vars = _get_machine_env_vars(resource, is_trusted=True, is_docker=True)
        machine_id_prefix = str(resource.id).split("-")[0]
        external_name = (
            f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-machine-{machine_id_prefix}"
        )
        container = self._docker_client.containers.run(
            f"{MACHINE_RUNTIME_IMAGE}:{resource.version}",
            environment=env_vars,
            detach=True,
            name=external_name,
            command=["python", "bench.py", "serve", "runtime", "0.0.0.0", str(assigned_port)],
            volumes=[f"{bench_dir}:/bench:ro"],  # mount our local code directly
            ports={f"{assigned_port}/tcp": ("0.0.0.0", assigned_port)},
        )
        async with self.host.session(commit=True):
            resource.external_name = external_name
            resource.external_id = container.id
            resource.status = ResourceStatus.HEALTHY
            resource.connection_uri = f"http://localhost:{assigned_port}"

    @override
    async def _do_update(self, resource: Machine):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Machine):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = self._docker_client.containers.get(resource.external_id)
        if container is not None:
            container.remove(force=True)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


KUBERNETES_KUBECONFIG_PATH = get_from_env_maybe(
    "KUBERNETES_KUBECONFIG_PATH", description="Path to kubeconfig file"
)
KUBERNETES_NAMESPACE = get_from_env(
    "KUBERNETES_NAMESPACE",
    default="default",
    description="Namespace to use for Kubernetes resources",
)

KUBERNETES_MACHINE_APP_LABEL = get_from_env_maybe(
    "KUBERNETES_MACHINE_APP_LABEL",
    default="bench-machine",
    description="Label to use for Kubernetes resources",
)
KUBERNETES_OVER_ALLOCATION = get_from_env(
    "KUBERNETES_OVER_ALLOCATION",
    default=4.0,
    typ=float,
    description="Whether to over-allocate resources",
)


async def get_kubernetes_client() -> KubernetesApiClient:
    """Gets the Kubernetes client."""
    from kubernetes_asyncio import config

    if KUBERNETES_KUBECONFIG_PATH is not None:
        await config.load_kube_config(KUBERNETES_KUBECONFIG_PATH)
    else:
        config.load_incluster_config()
    kubernetes_api = KubernetesApiClient()
    return kubernetes_api


class KubernetesApi:
    """Kubernetes API wrapper (because the generated kubernetes client is pretty bad)."""

    def __init__(self, namespace: str):
        self._namespace = namespace
        self._kubernetes_api: KubernetesApiClient | None = None
        self._kubernetes_core_api: KubernetesCoreV1Api | None = None

    @property
    def api(self) -> KubernetesApiClient:
        assert self._kubernetes_api is not None, "kubernetes api not ready"
        return self._kubernetes_api

    @property
    def core_api(self) -> KubernetesCoreV1Api:
        assert self._kubernetes_core_api is not None, "kubernetes core api not ready"
        return self._kubernetes_core_api

    async def start(self):
        self._kubernetes_api = await get_kubernetes_client()
        self._kubernetes_core_api = KubernetesCoreV1Api(self._kubernetes_api)

    async def close(self):
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
        self._kubernetes_core_api = None

    async def create_pod(self, pod: k8.V1Pod) -> None:
        """Create a Pod."""
        await self.core_api.create_namespaced_pod(namespace=self._namespace, body=pod)  # type: ignore

    async def patch_pod(self, pod: k8.V1Pod) -> None:
        """Patch a Pod. We only patch specific fields"""
        assert pod.spec and pod.spec.containers, f"missing containers for {pod!r}"
        pod_patch = {
            "spec": {
                "containers": [
                    {
                        "name": pod.spec.containers[0].name,
                        "image": pod.spec.containers[0].resources,
                    }
                ]
            }
        }
        await self.core_api.patch_namespaced_pod(
            namespace=self._namespace,
            name=pod.metadata.name,  # type: ignore
            body=pod_patch,
        )

    async def delete_pod(self, name: str) -> None:
        """Delete a Pod."""
        await self.core_api.delete_namespaced_pod(namespace=self._namespace, name=name)  # type: ignore

    async def get_pods(self, label_selector: str) -> tuple[list[k8.V1Pod], str]:
        """Get all Pods with the given label selector."""
        pods = await self.core_api.list_namespaced_pod(
            namespace=self._namespace, label_selector=label_selector
        )
        return pods.items, pods.metadata.resource_version


class KubernetesMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as Pods on Kubernetes."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_types = bittuple(NodeType.MACHINE)

    def __init__(self, host: HostApi, bench: Bench):
        super().__init__(host, bench)
        self._kubernetes_api: KubernetesApi | None = None
        self._kubernetes_pods_by_name: dict[str, k8.V1Pod] = {}

    @property
    def kubernetes_api(self) -> KubernetesApi:
        assert self._kubernetes_api is not None, "no kubernetes api"
        return self._kubernetes_api

    def _get_machine_external_name(self, machine: Machine) -> str:
        """Gets the external name of the given Machine."""
        machine_id_prefix = str(machine.id).split("-")[0]
        external_name = (
            f"bench-{ENV.value}-{CLOUD.slug}-{machine.region.slug}-machine-{machine_id_prefix}"
        )
        return external_name

    def _make_pod_from_machine(self, machine: Machine):
        """Creates a Kubernetes Pod for the Machine."""
        # context
        # NOTE: kubernetes resource names must be valid DNS labels (<= 63 chars)

        labels = {
            "app": KUBERNETES_MACHINE_APP_LABEL,
            "bench_id": str(machine.bench.id),
            "machine_id": str(machine.id),
            "environment": ENV.value,
            "cloud": CLOUD.slug,
            "region": machine.region.slug,
        }
        if isinstance(machine.parent, Server):
            labels["server_id"] = str(machine.parent.id)
        # nocheckin :Security: kubernetes-deployed machines should not trusted
        env_vars = _get_machine_env_vars(machine, is_trusted=True, is_docker=False)

        # pod
        resources = k8.V1ResourceRequirements(
            requests={
                "cpu": f"{round((machine.cpu / KUBERNETES_OVER_ALLOCATION) * 1000)}m",
                "memory": f"{round((machine.ram / KUBERNETES_OVER_ALLOCATION) * 1000)}Mi",
            },
            limits={
                "cpu": f"{round(machine.cpu * 1000)}m",
                "memory": f"{round(machine.ram * 1000)}Mi",
            },
        )
        health_probe = k8.V1Probe(
            grpc=k8.V1GRPCAction(port=60062, service="runtime"),
            initial_delay_seconds=5,
            period_seconds=10,
            failure_threshold=3,
        )
        main_container = k8.V1Container(
            name="main",
            image=f"{MACHINE_RUNTIME_IMAGE}:{machine.version}",
            command=["python", "bench.py", "serve", "runtime", "0.0.0.0", "60062"],
            env=[
                *(k8.V1EnvVar(name=k, value=v) for k, v in env_vars.items()),
                k8.V1EnvVar(
                    name="KUBERNETES_NODE_ID",
                    value_from=k8.V1EnvVarSource(
                        field_ref=k8.V1ObjectFieldSelector(field_path="spec.nodeName")
                    ),
                ),
            ],
            ports=[k8.V1ContainerPort(container_port=60062)],
            resources=resources,
            readiness_probe=health_probe,
            liveness_probe=health_probe,
        )
        image_pull_secret = get_from_env(
            "KUBERNETES_IMAGE_PULL_SECRET", description="Name of the image pull secret"
        )
        pod = k8.V1Pod(
            metadata=k8.V1ObjectMeta(name=self._get_machine_external_name(machine), labels=labels),
            spec=k8.V1PodSpec(
                containers=[main_container],
                image_pull_secrets=[k8.V1LocalObjectReference(name=image_pull_secret)],
                termination_grace_period_seconds=20,
            ),
        )
        return pod

    def _update_machine_from_pod(self, machine: Machine, pod: k8.V1Pod):
        """Updates the current config of the Machine from the given Pod."""
        machine.current_cpu = machine.cpu
        machine.current_ram = machine.ram
        machine.current_version = machine.version
        machine.status = machine.current_status = ResourceStatus.HEALTHY  # nocheckin

    @override
    async def _do_start(self) -> None:
        servers = self.bench.servers.tolist()
        machines = [m for s in servers for m in s.machines]
        machines_by_external_name: dict[str, Machine] = {
            m.external_name or self._get_machine_external_name(m): m for m in machines
        }

        self._kubernetes_api = KubernetesApi(namespace=KUBERNETES_NAMESPACE)
        await self._kubernetes_api.start()

        # sync machines with current pods
        pod_selector = f"app={KUBERNETES_MACHINE_APP_LABEL},bench_id={self.bench.id}"
        pods, pod_marker = await self._kubernetes_api.get_pods(label_selector=pod_selector)
        for pod in pods:
            assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
            self._kubernetes_pods_by_name[pod.metadata.name] = pod
            machine = machines_by_external_name.get(pod.metadata.name)
            if machine is None:
                # delete old pod
                await self._kubernetes_api.delete_pod(pod.metadata.name)
            else:
                async with self.host.session(commit=True):
                    self._update_machine_from_pod(machine, pod)
        async with self.host.session(commit=True):
            for machine in machines:
                if (
                    machine.external_name
                    and machine.external_name not in self._kubernetes_pods_by_name
                ):
                    machine.status = ResourceStatus.DECLARED

        # and keep watching for pod changes
        # nocheckin

    @override
    async def _do_provision(self, resource: Machine):
        pod = self._make_pod_from_machine(resource)
        await self.kubernetes_api.create_pod(pod)
        async with self.host.session(commit=True):
            resource.external_name = self._get_machine_external_name(resource)
            self._update_machine_from_pod(resource, pod)

    @override
    async def _do_update(self, resource: Machine):
        target_diff = resource._get_target_diff("cpu", "ram", "version")
        if target_diff:
            # only update if we need to
            assert resource.external_name is not None, f"{resource!r} has no external name"
            pod = self._make_pod_from_machine(resource)
            await self.kubernetes_api.patch_pod(pod)
            async with self.host.session(commit=True):
                self._update_machine_from_pod(resource, pod)

    @override
    async def _do_decommission(self, resource: Machine):
        if resource.external_name:
            await self.kubernetes_api.delete_pod(resource.external_name)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
