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
    Bench,
    Client,
    ClientType,
    Computer,
    ComputerType,
    NodeType,
    ResourceStatus,
    bittuple,
)
from bench.proto import dockerify_url, minikubeify_url
from bench.system.core.access import ACCESS_TOKEN_LENGTH
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.func import generate_access_token
from bench.utils.telemetry import OTLP_ENDPOINT
from bench.utils.utils import get_from_env

from .kubernetes import KUBERNETES_COMPUTER_APP_LABEL, KUBERNETES_NAMESPACE, KubernetesApi
from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


COMPUTER_RUNTIME_IMAGE = get_from_env(
    "COMPUTER_RUNTIME_IMAGE", description="Runtime container image for computer"
)
COMPUTER_UBUNTU_DESKTOP_IMAGE = get_from_env(
    "COMPUTER_UBUNTU_DESKTOP_IMAGE", description="Ubuntu desktop container image for computer"
)
COMPUTER_UBUNTU_TERMINAL_IMAGE = get_from_env(
    "COMPUTER_UBUNTU_TERMINAL_IMAGE", description="Ubuntu terminal container image for computer"
)
COMPUTER_OVERCOMMITMENT = get_from_env(
    "COMPUTER_OVERCOMMITMENT",
    default=2.0,
    typ=float,
    description="By how much to over-commit resources",
)
COMPUTER_GRPC_PORT = get_from_env(
    "COMPUTER_GRPC_PORT",
    typ=int,
    description="Port to expose for the computer's GRPC service",
)
COMPUTER_VNC_PORT = get_from_env(
    "COMPUTER_VNC_PORT",
    typ=int,
    description="Port to expose for the computer's VNC service (ws)",
)


def _get_computer_env_vars(
    computer: Computer,
    client: Client,
    *,
    is_trusted: bool,
    is_in_docker: bool = False,
    is_in_minikube: bool = False,
) -> dict[str, str]:
    """Gets the environment variables for a Computer."""
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_in_docker:
        supervisor_url = dockerify_url(supervisor_url)
    elif is_in_minikube:
        supervisor_url = minikubeify_url(supervisor_url)
    bench = computer.bench
    assert bench is not None, f"missing bench for {computer!r}"

    # env vars
    env_vars: dict[str, str | None] = {
        # hosting
        "SERVICE_NAME": "runtime",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": computer.region.slug,
        "SUPERVISOR_URL": supervisor_url,
        "IS_IN_DOCKER": "1" if is_in_docker else None,
        "IS_IN_MINIKUBE": "1" if is_in_minikube else None,
        # bench
        "BENCH_ID": str(bench.id),
        "COMPUTER_ID": str(computer.id),
        "CLIENT_ID": str(client.id),
        "CLIENT_TYPE": str(client.type.value),
        "CLIENT_ACCESS_TOKEN": client.access_token,
        # config
        "OTLP_ENDPOINT": OTLP_ENDPOINT,
        "TRACING": "1",
        "LOG_LEVEL": "DEBUG",
        "LOG_MODE": "JSON",
        "SENTRY_DSN": SENTRY_DSN,
        "DISPLAY_SIZE": f"{computer.width}x{computer.height}x24",
    }

    # add secret api keys directly (for testing/development)
    if is_trusted:
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


def _get_computer_image(computer: Computer) -> str:
    """Gets the Docker image for the given Computer."""
    if computer.type == ComputerType.RUNTIME:
        return f"{COMPUTER_RUNTIME_IMAGE}:{computer.version}"
    elif computer.type == ComputerType.UBUNTU:
        if computer.is_headless:
            return f"{COMPUTER_UBUNTU_TERMINAL_IMAGE}:{computer.version}"
        else:
            return f"{COMPUTER_UBUNTU_DESKTOP_IMAGE}:{computer.version}"
    else:
        raise NotImplementedError(f"cannot provision {computer!r}")


class ComputerProvisioner(Provisioner[Computer, Computer]):
    """Provision Computers."""

    watch_types = bittuple(NodeType.COMPUTER)
    provision_type = NodeType.COMPUTER

    async def _get_or_create_client(self, resource: Computer) -> Client:
        """Gets or creates a Client for the given Computer."""
        if resource.client_ptr is None:
            async with self.host.session(commit=True) as session:
                client = Client(
                    parent=resource.bench,
                    type=ClientType.COMPUTER,
                    name=resource.name,
                    access_token=generate_access_token(ACCESS_TOKEN_LENGTH),
                    computer=resource,
                    seen_at=session._oracle.utc(),
                )
                session._create(client)
                resource.client = client
        else:
            client = await Client.get(resource.client_ptr)
        return client


class DockerComputerProvisioner(ComputerProvisioner):
    """Provision Computers as containers in a Docker installation."""

    watch_types = bittuple(NodeType.COMPUTER)
    provision_type = NodeType.COMPUTER

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._docker_client = aiodocker.Docker()

    @override
    async def _do_start(self) -> None:
        computers = await self._get_resources()
        containers = await self._docker_client.containers.list(all=True)
        containers_by_id = {c.id: c for c in containers}
        async with self.host.session(commit=True):
            for computer in computers:
                if computer.external_id is None:
                    continue
                container = containers_by_id.get(computer.external_id)
                if container is None:
                    computer.update_status(ResourceStatus.PENDING)

    @override
    async def _do_provision(self, resource: Computer):
        client = await self._get_or_create_client(resource)
        grpc_port = random.randint(60100, 65000)
        vnc_port = random.randint(60100, 65000)
        env_vars = _get_computer_env_vars(
            computer=resource,
            client=client,
            is_trusted=resource.type == ComputerType.RUNTIME,
            is_in_docker=True,
        )
        computer_id_prefix = str(resource.id).split("-")[0]
        external_name = f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-{resource.type.name.lower()}-computer-{computer_id_prefix}"
        image = _get_computer_image(resource)
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
            resource.update_status(ResourceStatus.AVAILABLE)

    @override
    async def _do_update(self, resource: Computer):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Computer):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = await self._docker_client.containers.get(resource.external_id)
        if container is not None:
            await container.stop()
            await container.delete()
        async with self.host.session(commit=True):
            resource.update_status(ResourceStatus.OFFLINE)

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._docker_client is not None:
            await self._docker_client.close()


class KubernetesComputerProvisioner(ComputerProvisioner):
    """Provision Computers as Pods on Kubernetes."""

    watch_types = bittuple(NodeType.COMPUTER)
    provision_type = NodeType.COMPUTER

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._kubernetes_api: KubernetesApi | None = None
        self._kubernetes_pods_by_name: dict[str, k8.V1Pod] = {}

    @property
    def kubernetes_api(self) -> KubernetesApi:
        assert self._kubernetes_api is not None, "no kubernetes api"
        return self._kubernetes_api

    def _get_external_name(self, computer: Computer) -> str:
        """Gets the external name of the given Computer."""
        computer_id_prefix = str(computer.id).split("-")[0]
        # NOTE: kubernetes resource names must be valid DNS labels (<= 63 chars)
        external_name = f"bench-{ENV.value}-{CLOUD.slug}-{computer.region.slug}-{computer.type.name.lower()}-computer-{computer_id_prefix}"
        assert len(external_name) <= 63, f"external name too long: {external_name!r}"
        return external_name

    async def _get_computer_by_external_name(self, external_name: str) -> Computer | None:
        """Gets the Computer with the given external name."""
        computer = await Computer.where(
            Computer.get_property("external_name").eq(external_name)
        ).one_or_none()
        return computer

    def _get_pod_resources_requests(self, computer: Computer) -> dict[str, str]:
        """Gets the resource requests for the given Computer."""
        return {
            "cpu": f"{round((computer.cpu / COMPUTER_OVERCOMMITMENT) * 1000)}m",
            "memory": f"{round((computer.ram / COMPUTER_OVERCOMMITMENT) * 1000)}Mi",
        }

    def _get_pod_resources_limits(self, computer: Computer) -> dict[str, str]:
        """Gets the resource limits for the given Computer."""
        return {
            "cpu": f"{round(computer.cpu * 1000)}m",
            "memory": f"{round(computer.ram * 1000)}Mi",
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

    def _make_pod_from_computer(self, computer: Computer, client: Client):
        """Creates a Kubernetes Pod for the Computer."""

        # context
        labels = {
            "app": KUBERNETES_COMPUTER_APP_LABEL,
            "bench_id": str(self.bench.id),
            "computer_id": str(computer.id),
            "version": computer.version,
            "environment": ENV.value,
            "cloud": CLOUD.slug,
            "region": computer.region.slug,
        }
        # TODO :Security!: kubernetes-deployed runtime computers should not be trusted
        env_vars = _get_computer_env_vars(
            computer=computer,
            client=client,
            is_trusted=computer.type == ComputerType.RUNTIME,
            is_in_minikube=IS_DEV or IS_TEST,
        )

        # pod
        resources = k8.V1ResourceRequirements(
            requests=self._get_pod_resources_requests(computer),
            limits=self._get_pod_resources_limits(computer),
        )
        health_probe = k8.V1Probe(
            grpc=k8.V1GRPCAction(port=5432, service="runtime"),
            initial_delay_seconds=10,
            period_seconds=10,
            failure_threshold=3,
        )
        image = _get_computer_image(computer)
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
        if computer.type == ComputerType.RUNTIME:
            ports = [k8.V1ContainerPort(container_port=5432, host_port=COMPUTER_GRPC_PORT)]
        elif computer.type == ComputerType.UBUNTU:
            ports = [
                k8.V1ContainerPort(container_port=5432, host_port=COMPUTER_GRPC_PORT),
                k8.V1ContainerPort(container_port=6080, host_port=COMPUTER_VNC_PORT),
            ]
        else:
            raise NotImplementedError(f"cannot provision {computer!r}")
        main_container = k8.V1Container(
            name="main",
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
            metadata=k8.V1ObjectMeta(name=self._get_external_name(computer), labels=labels),
            spec=k8.V1PodSpec(
                containers=[main_container],
                image_pull_secrets=[k8.V1LocalObjectReference(name=image_pull_secret)],
                termination_grace_period_seconds=30,
            ),
        )
        return pod

    def _update_computer_from_pod(self, computer: Computer, pod: k8.V1Pod):
        """Updates the current state of the Computer from its respective Pod."""
        # resources
        resources_limits = cast(dict, pod.spec.containers[0].resources.limits)  # type: ignore
        cpu = self._parse_pod_resource_scalar(resources_limits["cpu"])
        if cpu is not None and computer.cpu != cpu:
            computer.cpu = cpu
        ram = self._parse_pod_resource_scalar(resources_limits["memory"])
        if ram is not None and computer.ram != ram:
            computer.ram = ram

        # version
        version = cast(str, pod.metadata.labels.get("version"))  # type: ignore
        if version is not None and computer.version != version:
            computer.version = version

        # status
        pod_phase = pod.status.phase if pod.status else None
        status = ResourceStatus.AVAILABLE if pod_phase == "Running" else ResourceStatus.UNAVAILABLE
        computer.update_status(status)

        # connection urls (using pod ip, only works inside cluster for now)
        if pod.status and pod.status.pod_ip:
            grpc_url = f"http://{pod.status.pod_ip}:{COMPUTER_GRPC_PORT}"
            if computer.grpc_url != grpc_url:
                computer.grpc_url = grpc_url
            if not computer.is_headless:
                vnc_url = f"ws://{pod.status.pod_ip}:{COMPUTER_VNC_PORT}"
                if computer.vnc_url != vnc_url:
                    computer.vnc_url = vnc_url

    async def _do_watch_pods(self, *, label_selector: str, resource_version: str) -> None:
        """Watches for changes to these Pods, update corresponding Computers."""
        async for event_type, pod in self.kubernetes_api.watch_pods(
            label_selector=label_selector, resource_version=resource_version
        ):
            async with self._lock:
                assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
                computer = await self._get_computer_by_external_name(pod.metadata.name)
                if computer is None:
                    continue  # ignore
                if event_type == "ADDED" or event_type == "MODIFIED":
                    self._kubernetes_pods_by_name[pod.metadata.name] = pod
                    async with self.host.session(commit=True):
                        self._update_computer_from_pod(computer, pod)
                elif event_type == "DELETED":
                    self._kubernetes_pods_by_name.pop(pod.metadata.name, None)
                    async with self.host.session(commit=True):
                        computer.update_status(ResourceStatus.OFFLINE)
                else:
                    assert_never(event_type)

    @override
    async def _do_start(self) -> None:
        computers = await self._get_resources()
        computers_by_external_name: dict[str, Computer] = {
            m.external_name or self._get_external_name(m): m for m in computers
        }

        self._kubernetes_api = KubernetesApi(namespace=KUBERNETES_NAMESPACE)
        await self._kubernetes_api.start()

        # sync computers with current pods
        pod_selector = f"app={KUBERNETES_COMPUTER_APP_LABEL},bench_id={self.bench.id}"
        pods, pod_marker = await self._kubernetes_api.get_pods(label_selector=pod_selector)
        for pod in pods:
            assert pod.metadata is not None, f"missing metadata for pod {pod!r}"
            self._kubernetes_pods_by_name[pod.metadata.name] = pod
            computer = computers_by_external_name.get(pod.metadata.name)
            if computer is None:
                # delete pod for removed computer
                await self._kubernetes_api.delete_pod(pod.metadata.name)
            else:
                async with self.host.session(commit=True):
                    self._update_computer_from_pod(computer, pod)
        async with self.host.session(commit=True):
            for computer in computers:
                if (
                    computer.external_name
                    and computer.external_name not in self._kubernetes_pods_by_name
                ):
                    computer.update_status(ResourceStatus.PENDING)

        # and keep watching for pod changes
        self.tasks.run(
            self._do_watch_pods(label_selector=pod_selector, resource_version=pod_marker),
            task_id=f"{self.bench.slug}.kubernetes.watch_pods",
        )

    @override
    async def _do_provision(self, resource: Computer):
        client = await self._get_or_create_client(resource)
        pod = self._make_pod_from_computer(resource, client)
        await self.kubernetes_api.create_pod(pod)
        async with self.host.session(commit=True):
            resource.external_name = self._get_external_name(resource)
            self._update_computer_from_pod(resource, pod)

    @override
    async def _do_update(self, resource: Computer):
        target_diff = resource._get_target_diff("cpu", "ram", "version")
        if (
            resource.status.is_extant
            and resource.reset_at is not None
            and resource.activated_at is not None
            and resource.reset_at > resource.activated_at
        ):
            # 'restart' by deleting it (to be recreated)
            assert resource.external_name is not None, f"{resource!r} has no external name"
            await self.kubernetes_api.delete_pod(resource.external_name)
        elif target_diff:
            assert resource.external_name is not None, f"{resource!r} has no external name"
            client = await self._get_or_create_client(resource)
            pod = self._make_pod_from_computer(resource, client)
            if "version" in target_diff:
                # replace full pod (to be recreated)
                await self.kubernetes_api.delete_pod(resource.external_name)
            else:
                # patch pod in place
                pod_patch = {
                    "spec": {
                        "containers": [
                            {
                                "name": "main",
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
                    self._update_computer_from_pod(resource, pod)

    @override
    async def _do_decommission(self, resource: Computer):
        if resource.external_name:
            try:
                await self.kubernetes_api.delete_pod(resource.external_name)
            except k8.ApiException as e:
                # probably already gone
                logger.warn("computer.decommission.error", resource=resource, exc_info=e)
        async with self.host.session(commit=True):
            resource.update_status(ResourceStatus.OFFLINE)

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
