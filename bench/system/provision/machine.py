import random
import re
from pathlib import Path
from typing import TYPE_CHECKING, assert_never, cast, override

import docker
import docker.models
import docker.models.containers
import structlog
from kubernetes_asyncio import client as k8
from opentelemetry import trace

from bench.language import Bench, Machine, ResourceStatus, Server
from bench.language.const import CLOUD, NodeType
from bench.proto.networking import dockerify_url, minikubeify_url
from bench.system.host.core import HostApi
from bench.system.provision.kubernetes import (
    KUBERNETES_MACHINE_APP_LABEL,
    KUBERNETES_NAMESPACE,
    KubernetesApi,
)
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
MACHINE_OVERCOMMITMENT = get_from_env(
    "MACHINE_OVERCOMMITMENT",
    default=4.0,
    typ=float,
    description="By how much to over-commit resources",
)
MACHINE_PORT = get_from_env(
    "MACHINE_PORT",
    default=60062,
    typ=int,
    description="Port to expose for the machine",
)


def _get_machine_env_vars(
    machine: Machine, *, is_trusted: bool, is_in_docker: bool = False, is_in_minikube: bool = False
) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    client = machine.client
    assert client, f"{machine!r} has no client"
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_in_docker:
        supervisor_url = dockerify_url(supervisor_url)
    elif is_in_minikube:
        supervisor_url = minikubeify_url(supervisor_url)
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
    project_dir = Path(__file__).parent.parent.parent.parent
    assert project_dir.exists() and project_dir.name == "bench", f"bad project dir: {project_dir!r}"
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
            resource.current_status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Machine):
        async with self.host.session(commit=True):
            resource.current_status = ResourceStatus.GONE


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
                    machine.current_status = ResourceStatus.DECLARED

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
            resource.current_status = ResourceStatus.UP
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
            resource.current_status = ResourceStatus.GONE


class KubernetesMachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines as Pods on Kubernetes."""

    # TODO :Test!: test kubernetes machine provisioner

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
            "cpu": f"{round((machine.cpu / MACHINE_OVERCOMMITMENT) * 1000)}m",
            "memory": f"{round((machine.ram / MACHINE_OVERCOMMITMENT) * 1000)}Mi",
        }

    def _get_pod_resources_limits(self, machine: Machine) -> dict[str, str]:
        """Gets the resource limits for the given Machine."""
        return {
            "cpu": f"{round(machine.cpu * 1000)}m",
            "memory": f"{round(machine.ram * 1000)}Mi",
        }

    def _parse_pod_resource_scalar(self, scalar: str) -> float | None:
        """Parses a pod scalar into a float in G units."""
        try:
            # use regex (we assume the format is as above)
            match = re.match(r"^([0-9.]+)(m|Mi)$", scalar)
            if match:
                return float(match.group(1)) / 1000
            else:
                return None
        except Exception:
            return None

    def _make_pod_from_machine(self, machine: Machine):
        """Creates a Kubernetes Pod for the Machine."""
        # context
        # NOTE: kubernetes resource names must be valid DNS labels (<= 63 chars)

        labels = {
            "app": KUBERNETES_MACHINE_APP_LABEL,
            "bench_id": str(machine.bench.id),
            "machine_id": str(machine.id),
            "version": machine.version,
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
            grpc=k8.V1GRPCAction(port=MACHINE_PORT, service="runtime"),
            initial_delay_seconds=10,
            period_seconds=10,
            failure_threshold=3,
        )
        main_container = k8.V1Container(
            name="main",
            image=f"{MACHINE_RUNTIME_IMAGE}:{machine.version}",
            command=["python", "bench.py", "serve", "runtime", "0.0.0.0", str(MACHINE_PORT)],
            env=[
                *(k8.V1EnvVar(name=k, value=v) for k, v in env_vars.items()),
                k8.V1EnvVar(
                    name="KUBERNETES_NODE_ID",
                    value_from=k8.V1EnvVarSource(
                        field_ref=k8.V1ObjectFieldSelector(field_path="spec.nodeName")
                    ),
                ),
            ],
            ports=[k8.V1ContainerPort(container_port=MACHINE_PORT)],
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
                termination_grace_period_seconds=30,
            ),
        )
        return pod

    def _update_machine_from_pod(self, machine: Machine, pod: k8.V1Pod):
        """Updates the current state of the Machine from its respective Pod."""
        # resources
        resources_limits = cast(dict, pod.spec.containers[0].resources.limits)  # type: ignore
        current_cpu = self._parse_pod_resource_scalar(resources_limits["cpu"])
        if current_cpu is not None and machine.current_cpu != current_cpu:
            machine.current_cpu = current_cpu
        current_ram = self._parse_pod_resource_scalar(resources_limits["memory"])
        if current_ram is not None and machine.current_ram != current_ram:
            machine.current_ram = current_ram

        # version
        current_version = cast(str, pod.metadata.labels.get("version"))  # type: ignore
        if current_version is not None and machine.current_version != current_version:
            machine.current_version = current_version

        # status
        pod_phase = cast(str, pod.status.phase) if pod.status else None  # type: ignore
        current_status = ResourceStatus.UP if pod_phase == "Running" else ResourceStatus.DOWN
        if machine.current_status != current_status:
            machine.current_status = current_status

        # connection uri (using pod ip, only works inside cluster for now)
        if pod.status and pod.status.pod_ip:  # type: ignore
            connection_uri = f"http://{pod.status.pod_ip}:{MACHINE_PORT}"  # type: ignore
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
                self._kubernetes_pods_by_name[pod.metadata.name] = pod
                async with self.host.session(commit=True):
                    self._update_machine_from_pod(machine, pod)
            elif event_type == "DELETED":
                self._kubernetes_pods_by_name.pop(pod.metadata.name, None)
                async with self.host.session(commit=True):
                    machine.current_status = ResourceStatus.GONE
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
                    machine.current_status = ResourceStatus.DECLARED

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
                            "name": "main",
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
            try:
                await self.kubernetes_api.delete_pod(resource.external_name)
            except k8.ApiException as e:
                # probably already gone
                logger.warn("machine.decommission.error", resource=resource, exc_info=e)
        async with self.host.session(commit=True):
            resource.current_status = ResourceStatus.GONE

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
