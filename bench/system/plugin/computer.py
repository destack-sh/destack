import random
from pathlib import Path
from typing import TYPE_CHECKING, assert_never, cast, override

import aiodocker
import regex
import structlog
from kubernetes_asyncio import client as k8
from opentelemetry import trace

from bench.language import (
    CLOUD,
    VERSION,
    Bench,
    Client,
    ClientType,
    Machine,
    MachineType,
    NodeType,
    ResourceStatus,
    bittuple,
)
from bench.proto import dockerify_url, minikubeify_url
from bench.system.core.access import ACCESS_TOKEN_LENGTH
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.func import generate_access_token
from bench.utils.telemetry import OTLP_ENDPOINT
from bench.utils.utils import get_from_env, get_from_env_maybe

from .kubernetes import KUBERNETES_NAMESPACE, KubernetesApi
from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


MACHINE_RUNTIME_IMAGE = get_from_env(
    "MACHINE_RUNTIME_IMAGE", description="Runtime container image for machine"
)
MACHINE_UBUNTU_DESKTOP_IMAGE = get_from_env(
    "MACHINE_UBUNTU_DESKTOP_IMAGE", description="Ubuntu desktop container image for machine"
)
MACHINE_UBUNTU_TERMINAL_IMAGE = get_from_env(
    "MACHINE_UBUNTU_TERMINAL_IMAGE", description="Ubuntu terminal container image for machine"
)
MACHINE_OVERCOMMITMENT = get_from_env(
    "MACHINE_OVERCOMMITMENT",
    default=2.0,
    typ=float,
    description="By how much to over-commit resources",
)
MACHINE_GRPC_PORT = get_from_env(
    "MACHINE_GRPC_PORT",
    typ=int,
    description="Port to expose for the machine's GRPC service",
)
MACHINE_VNC_PORT = get_from_env(
    "MACHINE_VNC_PORT",
    typ=int,
    description="Port to expose for the machine's VNC service (ws)",
)

# TODO :Security!: review keys to pass to semi-trusted machines (proxy/sidecar?)
#  (also should if anything pass them as k8 secret key refs?)
MACHINE_SECRET_ENV_KEYS = (
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GEMINI_API_KEY",
    "XAI_API_KEY",
    "OPENROUTER_API_KEY",
    "EXA_API_KEY",
    "UNSPLASH_ACCESS_KEY",
    "POSTHOG_HOST",
    "POSTHOG_TOKEN",
)


def _get_machine_env_vars(
    machine: Machine,
    client: Client,
    *,
    is_in_docker: bool = False,
    is_in_minikube: bool = False,
) -> dict[str, str]:
    """Gets the environment variables for a Machine."""
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_in_docker:
        supervisor_url = dockerify_url(supervisor_url)
    elif is_in_minikube:
        supervisor_url = minikubeify_url(supervisor_url)
    bench = machine.bench
    assert bench is not None, f"missing bench for {machine!r}"

    # env vars
    env_vars: dict[str, str | None] = {
        # hosting
        "SERVICE_NAME": "bench-machine",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": machine.region.slug,
        "VERSION": machine.version,
        "SUPERVISOR_URL": supervisor_url,
        "IS_IN_DOCKER": "1" if is_in_docker else None,
        "IS_IN_MINIKUBE": "1" if is_in_minikube else None,
        # bench
        "BENCH_ID": str(bench.id),
        "MACHINE_ID": str(machine.id),
        "CLIENT_ID": str(client.id),
        "CLIENT_TYPE": str(client.type.value),
        "CLIENT_ACCESS_TOKEN": client.access_token,
        # config
        "OTLP_ENDPOINT": OTLP_ENDPOINT,
        "TRACING": "1",
        "LOG_LEVEL": "DEBUG",
        "LOG_MODE": "JSON",
        "DISPLAY_SIZE": f"{machine.width}x{machine.height}x24",
    }

    # add secret keys
    for key in MACHINE_SECRET_ENV_KEYS:
        env_vars[key] = get_from_env_maybe(key)

    return {k: v for k, v in env_vars.items() if v}


def _get_bench_dir() -> str:
    project_dir = Path(__file__).parent.parent.parent.parent
    assert project_dir.exists() and project_dir.name == "bench", f"bad project dir: {project_dir!r}"
    bench_dir = (project_dir / "bench").absolute().as_posix()
    return bench_dir


def _get_machine_image(machine: Machine) -> str:
    """Gets the Docker image for the given Machine."""
    if machine.type == MachineType.RUNTIME:
        return f"{MACHINE_RUNTIME_IMAGE}:{machine.version}"
    elif machine.type == MachineType.UBUNTU:
        if machine.is_headless:
            return f"{MACHINE_UBUNTU_TERMINAL_IMAGE}:{machine.version}"
        else:
            return f"{MACHINE_UBUNTU_DESKTOP_IMAGE}:{machine.version}"
    else:
        raise NotImplementedError(f"cannot provision {machine!r}")


class MachineProvisioner(Provisioner[Machine, Machine]):
    """Provision Machines."""

    watch_types = bittuple(NodeType.MACHINE)
    provision_type = NodeType.MACHINE

    async def _get_or_create_client(self, resource: Machine) -> Client:
        """Gets or creates a Client for the given Machine."""
        if resource.client_ptr is None:
            async with self.host.session(commit=True) as session:
                client = Client(
                    parent=resource.bench,
                    type=ClientType.MACHINE,
                    name=resource.name,
                    access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                    machine=resource,
                    seen_at=session.oracle.utc(),
                )
                session._create(client)
                resource.client = client
        else:
            client = await Client.get(resource.client_ptr)
        return client


class DockerMachineProvisioner(MachineProvisioner):
    """
    Provision Machines as containers in a Docker installation.
    TODO :Dev: use Kubernetes locally too (drop Docker compose & Docker provisioning)
    """

    watch_types = bittuple(NodeType.MACHINE)
    provision_type = NodeType.MACHINE

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._docker_client = aiodocker.Docker()

    @override
    async def _do_start(self) -> None:
        machines = await self._get_resources()
        containers = await self._docker_client.containers.list(all=True)
        containers_by_id = {c.id: c for c in containers}
        async with self.host.session(commit=True):
            for machine in machines:
                if machine.external_id is None:
                    continue
                container = containers_by_id.get(machine.external_id)
                if container is None:
                    machine.update_status(ResourceStatus.PENDING)

    @override
    async def _do_provision(self, resource: Machine):
        client = await self._get_or_create_client(resource)
        grpc_port = random.randint(60100, 65000)
        vnc_port = random.randint(60100, 65000)
        env_vars = _get_machine_env_vars(machine=resource, client=client, is_in_docker=True)
        machine_id_prefix = str(resource.id).split("-")[0]
        external_name = f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-{resource.type.name.lower()}-machine-{machine_id_prefix}"
        image = _get_machine_image(resource)
        config = {
            "Image": image,
            "Env": [f"{key}={value}" for key, value in env_vars.items()],
            "HostConfig": {
                "Binds": [f"{_get_bench_dir()}:/bench:ro"],
                "PortBindings": {
                    "5432/tcp": [{"HostIp": "0.0.0.0", "HostPort": str(grpc_port)}],
                    "6080/tcp": [{"HostIp": "0.0.0.0", "HostPort": str(vnc_port)}],
                },
            },
        }
        container = await self._docker_client.containers.run(config, name=external_name)
        async with self.host.session(commit=True):
            resource.external_name = external_name
            resource.external_id = container.id
            resource.grpc_url = f"http://localhost:{grpc_port}"
            if not resource.is_headless:
                resource.vnc_url = f"ws://localhost:{vnc_port}"
            else:
                resource.vnc_url = None
            self._set_resource_status(resource, ResourceStatus.AVAILABLE)

    @override
    async def _do_update(self, resource: Machine):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Machine):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = await self._docker_client.containers.get(resource.external_id)
        if container is not None:
            await container.stop()
            await container.delete()
        async with self.host.session(commit=True):
            self._set_resource_status(resource, ResourceStatus.OFFLINE)

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._docker_client is not None:
            await self._docker_client.close()


class KubernetesMachineProvisioner(MachineProvisioner):
    """
    Provision Machines as Pods on Kubernetes.
    NOTE :Incomplete :RichComputing: Machines should really be in k8 Deployments/Services?
    """

    watch_types = bittuple(NodeType.MACHINE)
    provision_type = NodeType.MACHINE

    def __init__(self, host: "HostService", bench: Bench):
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
        # NOTE: kubernetes resource names must be valid DNS labels (<= 63 chars)
        external_name = f"bench-{ENV.value}-{CLOUD.slug}-{machine.region.slug}-machine-{machine.type.name.lower()}-{machine_id_prefix}"
        assert len(external_name) <= 63, f"external name too long: {external_name!r}"
        return external_name

    async def _get_machine_by_external_name(self, external_name: str) -> Machine | None:
        """Gets the Machine with the given external name."""
        machine_query = Machine.where(Machine.property("external_name").eq(external_name))
        machine_query._include_memory = False
        machine = await machine_query.one_or_none()
        return machine

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
            match = regex.match(r"^([0-9.]+)(m|Mi)$", scalar)
            if match:
                return float(match.group(1)) / 1000
            else:
                return None
        except Exception:
            return None

    def _make_pod_from_machine(self, machine: Machine, client: Client):
        """Creates a Kubernetes Pod for the Machine."""

        # context
        labels = {
            "app": f"bench-machine-{machine.type.name.lower()}",
            "bench_id": str(self.bench.id),
            "machine_id": str(machine.id),
            "version": machine.version,
            "env": ENV.value,
            "cloud": CLOUD.slug,
            "region": machine.region.slug,
        }
        env_vars = _get_machine_env_vars(
            machine=machine, client=client, is_in_minikube=IS_DEV or IS_TEST
        )

        # pod
        resources = k8.V1ResourceRequirements(
            requests=self._get_pod_resources_requests(machine),
            limits=self._get_pod_resources_limits(machine),
        )
        health_probe = k8.V1Probe(
            grpc=k8.V1GRPCAction(port=5432, service="runtime"),
            initial_delay_seconds=10,
            period_seconds=10,
            failure_threshold=3,
        )
        image = _get_machine_image(machine)
        env = [
            *(k8.V1EnvVar(name=k, value=v) for k, v in env_vars.items()),
            k8.V1EnvVar(
                name="KUBERNETES_NODE_ID",
                value_from=k8.V1EnvVarSource(
                    field_ref=k8.V1ObjectFieldSelector(field_path="spec.nodeName")
                ),
            ),
        ]

        # container
        if machine.type == MachineType.RUNTIME:
            ports = [k8.V1ContainerPort(container_port=5432, host_port=MACHINE_GRPC_PORT)]
        elif machine.type == MachineType.UBUNTU:
            ports = [
                k8.V1ContainerPort(container_port=5432, host_port=MACHINE_GRPC_PORT),
                k8.V1ContainerPort(container_port=6080, host_port=MACHINE_VNC_PORT),
            ]
        else:
            raise NotImplementedError(f"cannot provision {machine!r}")
        main_container = k8.V1Container(
            name=f"machine-{machine.type.name.lower()}",
            image=image,
            env=env,
            ports=ports,
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
        cpu = self._parse_pod_resource_scalar(resources_limits["cpu"])
        if cpu is not None and machine.cpu != cpu:
            machine.cpu = cpu
        ram = self._parse_pod_resource_scalar(resources_limits["memory"])
        if ram is not None and machine.ram != ram:
            machine.ram = ram

        # version
        version = cast(str, pod.metadata.labels.get("version"))  # type: ignore
        if version is not None and machine.version != version:
            machine.version = version

        # status
        pod_phase = pod.status.phase if pod.status else None
        status = ResourceStatus.AVAILABLE if pod_phase == "Running" else ResourceStatus.UNAVAILABLE
        machine.update_status(status)

        # connection urls (using pod ip, only works inside cluster for now)
        if pod.status and pod.status.pod_ip:
            grpc_url = f"http://{pod.status.pod_ip}:{MACHINE_GRPC_PORT}"
            if machine.grpc_url != grpc_url:
                machine.grpc_url = grpc_url
            if not machine.is_headless:
                vnc_url = f"ws://{pod.status.pod_ip}:{MACHINE_VNC_PORT}"
            else:
                vnc_url = None
            if machine.vnc_url != vnc_url:
                machine.vnc_url = vnc_url

    async def _do_watch_pods(self, *, label_selector: str, resource_version: str | None) -> None:
        """Watches for changes to these Pods, update corresponding Machines."""
        async for event_type, pod in self.kubernetes_api.watch_pods(
            label_selector=label_selector, resource_version=resource_version
        ):
            async with self._lock:
                assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
                machine = await self._get_machine_by_external_name(pod.metadata.name)
                if machine is None:
                    continue  # ignore
                if event_type == "ADDED" or event_type == "MODIFIED":
                    self._kubernetes_pods_by_name[pod.metadata.name] = pod
                    async with self.host.session(commit=True):
                        self._update_machine_from_pod(machine, pod)
                elif event_type == "DELETED":
                    self._kubernetes_pods_by_name.pop(pod.metadata.name, None)
                    async with self.host.session(commit=True):
                        machine.update_status(ResourceStatus.OFFLINE)
                else:
                    assert_never(event_type)

    @override
    async def _do_start(self) -> None:
        machines = await self._get_resources()
        machines_by_external_name: dict[str, Machine] = {
            m.external_name or self._get_external_name(m): m for m in machines
        }

        self._kubernetes_api = KubernetesApi(namespace=KUBERNETES_NAMESPACE)
        await self._kubernetes_api.start()

        # sync machines with current pods
        machine_labels = [f"bench-machine-{machine.type.name.lower()}" for machine in machines]
        pod_selector = f"app in ({','.join(machine_labels)}),bench_id={self.bench.id}"
        pods, _ = await self._kubernetes_api.get_pods(label_selector=pod_selector)
        for pod in pods:
            assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
            self._kubernetes_pods_by_name[pod.metadata.name] = pod
            machine = machines_by_external_name.get(pod.metadata.name)
            if machine is None:
                # delete pod for removed machine
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
                    machine.update_status(ResourceStatus.PENDING)

        # and keep watching for pod changes (we don't pass resource_version as it may be too old)
        self.tasks.run_forever(
            lambda: self._do_watch_pods(label_selector=pod_selector, resource_version=None),
            task_id=f"{self.bench.slug}.kubernetes.watch_pods",
            skip_errors=True,
        )

    @override
    async def _do_provision(self, resource: Machine):
        client = await self._get_or_create_client(resource)
        pod = self._make_pod_from_machine(resource, client)
        await self.kubernetes_api.create_pod(pod)
        async with self.host.session(commit=True):
            resource.external_name = self._get_external_name(resource)
            self._update_machine_from_pod(resource, pod)

    @override
    async def _do_update(self, resource: Machine):
        if resource.should_reset or resource.version != VERSION:
            # 'restart' by deleting it (to be recreated)
            assert resource.external_name is not None, f"{resource!r} has no external name"
            try:
                await self.kubernetes_api.delete_pod(resource.external_name)
            except k8.ApiException as e:
                # probably already gone
                logger.warn("machine.delete.error", resource=resource, exc_info=e)

    @override
    async def _do_decommission(self, resource: Machine):
        if resource.external_name:
            try:
                await self.kubernetes_api.delete_pod(resource.external_name)
            except k8.ApiException as e:
                # probably already gone
                logger.warn("machine.decommission.error", resource=resource, exc_info=e)
        async with self.host.session(commit=True):
            self._set_resource_status(resource, ResourceStatus.OFFLINE)

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
