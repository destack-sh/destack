import random
from pathlib import Path
from typing import TYPE_CHECKING, Literal, assert_never, cast, override

import docker
import docker.models
import docker.models.containers
import structlog
from kubernetes_asyncio import client as k8
from kubernetes_asyncio.client import ApiClient as KubernetesApiClient
from kubernetes_asyncio.client import CoreV1Api as KubernetesCoreV1Api
from opentelemetry import trace

from bench.language import Bench, Machine, ResourceStatus, Server
from bench.language.const import (
    CLOUD,
    NodeType,
    dockerify_domain,
    minikubeify_domain,
)
from bench.system.host.core import HostApi
from bench.system.provision.provisioner import Provisioner
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.func import bittuple
from bench.utils.utils import get_from_env, get_from_env_maybe

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


MACHINE_RUNTIME_IMAGE = get_from_env(
    "MACHINE_RUNTIME_IMAGE", description="Runtime container image for machine"
)


def _get_machine_env_vars(
    machine: Machine, *, is_trusted: bool, is_in_docker: bool = False, is_in_minikube: bool = False
) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    client = machine.client
    assert client, f"{machine!r} has no client"
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_in_docker:
        supervisor_url = dockerify_domain(supervisor_url)
    elif is_in_minikube:
        supervisor_url = minikubeify_domain(supervisor_url)
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
    if is_in_docker:
        env_vars["IS_IN_DOCKER"] = "1"
    if is_in_minikube:
        env_vars["IS_IN_MINIKUBE"] = "1"
    if SENTRY_DSN:
        env_vars["SENTRY_DSN"] = SENTRY_DSN
    if is_trusted:
        # add secret api keys directly (for testing/development)
        env_vars["OPENAI_API_KEY"] = get_from_env("OPENAI_API_KEY", description="OpenAI API key")
        env_vars["ANTHROPIC_API_KEY"] = get_from_env(
            "ANTHROPIC_API_KEY", description="Anthropic API key"
        )
    return {k: v for k, v in env_vars.items() if v}


def _get_bench_dir() -> str:
    project_dir = Path(__file__).parent.parent.parent
    assert project_dir.exists() and project_dir.name == "bench", f"{project_dir!r}"
    bench_dir = (project_dir / "bench").absolute().as_posix()
    return bench_dir


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
            resource.status = ResourceStatus.READY

    @override
    async def _do_decommission(self, resource: Machine):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.GONE


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
        bench_dir = _get_bench_dir()
        assigned_port = random.randint(60100, 65000)
        env_vars = _get_machine_env_vars(resource, is_trusted=True, is_in_docker=True)
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
            resource.status = ResourceStatus.READY
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
            resource.status = ResourceStatus.GONE


KUBERNETES_KUBECONFIG_PATH = get_from_env_maybe(
    "KUBERNETES_KUBECONFIG_PATH", description="Path to kubeconfig file"
)
KUBERNETES_KUBECONFIG_CONTEXT = get_from_env_maybe(
    "KUBERNETES_KUBECONFIG_CONTEXT",
    description="Context to use in kubeconfig file",
    default="minikube",
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
    description="By how much to over-commit resources",
)


async def get_kubernetes_client() -> KubernetesApiClient:
    """Gets the Kubernetes client."""
    from kubernetes_asyncio import config

    if KUBERNETES_KUBECONFIG_PATH is not None:
        await config.load_kube_config(
            KUBERNETES_KUBECONFIG_PATH, context=KUBERNETES_KUBECONFIG_CONTEXT
        )
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

    async def patch_pod(self, name: str, patch: dict) -> None:
        """Patch a Pod. We only patch specific fields"""
        await self.core_api.patch_namespaced_pod(namespace=self._namespace, name=name, body=patch)

    async def delete_pod(self, name: str) -> None:
        """Delete a Pod."""
        await self.core_api.delete_namespaced_pod(namespace=self._namespace, name=name)  # type: ignore

    async def get_pods(self, label_selector: str) -> tuple[list[k8.V1Pod], str]:
        """Get all Pods with the given label selector."""
        pods = await self.core_api.list_namespaced_pod(
            namespace=self._namespace, label_selector=label_selector
        )
        return pods.items, pods.metadata.resource_version

    async def watch_pods(self, *, label_selector: str, resource_version: str):
        """Watch for Pod changes with the given label selector."""
        from kubernetes_asyncio.watch import Watch as KubernetesWatch

        async with KubernetesWatch().stream(
            self.core_api.list_namespaced_pod,
            namespace=self._namespace,
            label_selector=label_selector,
            resource_version=resource_version,
        ) as stream:
            async for event in stream:
                event_type = cast(Literal["ADDED", "MODIFIED", "DELETED"], event["type"])  # type: ignore
                assert event_type in (
                    "ADDED",
                    "MODIFIED",
                    "DELETED",
                ), f"unexpected event type: {event_type}"
                event_object = cast(k8.V1Pod, event["object"])  # type: ignore
                yield event_type, event_object


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

    def _get_external_name(self, machine: Machine) -> str:
        """Gets the external name of the given Machine."""
        machine_id_prefix = str(machine.id).split("-")[0]
        external_name = (
            f"bench-{ENV.value}-{CLOUD.slug}-{machine.region.slug}-machine-{machine_id_prefix}"
        )
        return external_name

    def _get_machine_by_external_name(self, external_name: str) -> Machine | None:
        """Gets the Machine with the given external name."""
        for server in self.bench.servers:
            for machine in server.machines:
                if machine.external_name == external_name:
                    return machine
        return None

    def _get_pod_resources_requests(self, machine: Machine) -> dict[str, str]:
        """Gets the resource requests for the given Machine."""
        return {
            "cpu": f"{round((machine.cpu / KUBERNETES_OVER_ALLOCATION) * 1000)}m",
            "memory": f"{round((machine.ram / KUBERNETES_OVER_ALLOCATION) * 1000)}Mi",
        }

    def _get_pod_resources_limits(self, machine: Machine) -> dict[str, str]:
        """Gets the resource limits for the given Machine."""
        return {
            "cpu": f"{round(machine.cpu * 1000)}m",
            "memory": f"{round(machine.ram * 1000)}Mi",
        }

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
        env_vars = _get_machine_env_vars(machine, is_trusted=True, is_in_minikube=IS_DEV or IS_TEST)

        # pod
        resources = k8.V1ResourceRequirements(
            requests=self._get_pod_resources_requests(machine),
            limits=self._get_pod_resources_limits(machine),
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
            metadata=k8.V1ObjectMeta(name=self._get_external_name(machine), labels=labels),
            spec=k8.V1PodSpec(
                containers=[main_container],
                image_pull_secrets=[k8.V1LocalObjectReference(name=image_pull_secret)],
                termination_grace_period_seconds=20,
            ),
        )
        return pod

    def _update_machine_from_pod(self, machine: Machine, pod: k8.V1Pod):
        """Updates the current config of the Machine from the given Pod."""
        if machine.current_cpu != machine.cpu:
            machine.current_cpu = machine.cpu
        if machine.current_ram != machine.ram:
            machine.current_ram = machine.ram
        if machine.current_version != machine.version:
            machine.current_version = machine.version
        # NOTE :Robustness: reflect actual pod status in Machine status
        machine.status = machine.current_status = ResourceStatus.READY
        if pod.status and pod.status.pod_ip:  # type: ignore
            connection_uri = (
                f"http://{pod.status.pod_ip}:{pod.spec.containers[0].ports[0].container_port}"  # type: ignore
            )
        else:
            connection_uri = None
        if machine.connection_uri != connection_uri:
            machine.connection_uri = connection_uri

    async def _do_watch_pods(self, *, label_selector: str, resource_version: str) -> None:
        async for event_type, pod in self.kubernetes_api.watch_pods(
            label_selector=label_selector, resource_version=resource_version
        ):
            assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
            machine = self._get_machine_by_external_name(pod.metadata.name)
            if machine is None:
                continue  # ignore
            if event_type == "ADDED" or event_type == "MODIFIED":
                async with self.host.session(commit=True):
                    self._update_machine_from_pod(machine, pod)
            elif event_type == "DELETED":
                async with self.host.session(commit=True):
                    machine.current_status = machine.status = ResourceStatus.DECLARED
            else:
                assert_never(event_type)

    @override
    async def _do_start(self) -> None:
        servers = self.bench.servers.tolist()
        machines = [m for s in servers for m in s.machines]
        machines_by_external_name: dict[str, Machine] = {
            m.external_name or self._get_external_name(m): m for m in machines
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
        self.tasks.run(
            self._do_watch_pods(label_selector=pod_selector, resource_version=pod_marker),
            task_id=f"{self.bench.slug}.kubernetes.watch_pods",
        )

    @override
    async def _do_provision(self, resource: Machine):
        pod = self._make_pod_from_machine(resource)
        await self.kubernetes_api.create_pod(pod)
        async with self.host.session(commit=True):
            resource.external_name = self._get_external_name(resource)
            self._update_machine_from_pod(resource, pod)

    @override
    async def _do_update(self, resource: Machine):
        # only update if we need to
        target_diff = resource._get_target_diff("cpu", "ram", "version")
        if target_diff:
            assert resource.external_name is not None, f"{resource!r} has no external name"
            pod = self._make_pod_from_machine(resource)
            pod_patch = {
                "spec": {
                    "containers": [
                        {
                            "name": resource.external_name,
                            "image": f"{MACHINE_RUNTIME_IMAGE}:{resource.version}",
                            "resources": {
                                "requests": self._get_pod_resources_requests(resource),
                                "limits": self._get_pod_resources_limits(resource),
                            },
                        }
                    ]
                }
            }
            await self.kubernetes_api.patch_pod(resource.external_name, pod_patch)
            async with self.host.session(commit=True):
                self._update_machine_from_pod(resource, pod)

    @override
    async def _do_decommission(self, resource: Machine):
        if resource.external_name:
            await self.kubernetes_api.delete_pod(resource.external_name)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.GONE

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
