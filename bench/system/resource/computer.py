import random
from pathlib import Path
from typing import TYPE_CHECKING, assert_never, cast, override

import docker
import docker.models
import docker.models.containers
import regex
import structlog
from kubernetes_asyncio import client as k8
from opentelemetry import trace

from bench.language import CLOUD, Bench, Computer, NodeType, ResourceStatus
from bench.language.core import bittuple
from bench.proto import dockerify_url, minikubeify_url
from bench.utils.analytics import SENTRY_DSN
from bench.utils.env import ENV, IS_DEV, IS_TEST
from bench.utils.telemetry import OTLP_ENDPOINT
from bench.utils.utils import get_from_env, get_from_env_maybe

from .kubernetes import (
    KUBERNETES_COMPUTER_APP_LABEL,
    KUBERNETES_NAMESPACE,
    KubernetesApi,
)
from .provisioner import Provisioner

if TYPE_CHECKING:
    from bench.system.host import HostService

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


COMPUTER_RUNTIME_IMAGE = get_from_env(
    "COMPUTER_RUNTIME_IMAGE", description="Runtime container image for computer"
)
COMPUTER_OVERCOMMITMENT = get_from_env(
    "COMPUTER_OVERCOMMITMENT",
    default=2.0,
    typ=float,
    description="By how much to over-commit resources",
)
COMPUTER_PORT = get_from_env(
    "COMPUTER_PORT",
    default=60062,
    typ=int,
    description="Port to expose for the computer",
)


def _get_computer_env_vars(
    computer: Computer,
    *,
    is_trusted: bool,
    is_in_docker: bool = False,
    is_in_minikube: bool = False,
) -> dict[str, str]:
    """Gets the environment variables for a Computer."""
    client = computer.client
    assert client, f"{computer!r} has no client"
    supervisor_url = get_from_env("SUPERVISOR_URL", description="Supervisor URL")
    if is_in_docker:
        supervisor_url = dockerify_url(supervisor_url)
    elif is_in_minikube:
        supervisor_url = minikubeify_url(supervisor_url)
    bench = computer.bench
    assert bench is not None, f"missing bench for {computer!r}"
    env_vars: dict[str, str | None] = {
        # hosting
        "SERVICE_NAME": "runtime",
        "ENVIRONMENT": ENV.value,
        "CLOUD": CLOUD.slug,
        "REGION": computer.region.slug,
        "SUPERVISOR_URL": supervisor_url,
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


# TODO :Test!: test computer provisioners


class LocalhostComputerProvisioner(Provisioner[Computer, Computer]):
    """Provision Computers by short-circuiting to localhost."""

    watch_types = bittuple(NodeType.COMPUTER)
    provision_type = NodeType.COMPUTER

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._local_computer_url = get_from_env_maybe(
            "LOCAL_COMPUTER_URL", description="URL for local computer runtime"
        )

    @override
    async def _do_start(self) -> None:
        pass

    @override
    async def _do_provision(self, resource: Computer):
        async with self.host.session(commit=True):
            resource.connection_uri = self._local_computer_url
            resource.status = ResourceStatus.UP

    @override
    async def _do_decommission(self, resource: Computer):
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class DockerComputerProvisioner(Provisioner[Computer, Computer]):
    """Provision Computers as containers in a Docker installation."""

    # NOTE :DevX: use async docker api (instead of blocking sync)?

    watch_types = bittuple(NodeType.COMPUTER)
    provision_type = NodeType.COMPUTER

    def __init__(self, host: "HostService", bench: Bench):
        super().__init__(host, bench)
        self._docker_client = docker.from_env()

    @override
    async def _do_start(self) -> None:
        computers = await Computer.where(Computer.get_property("bench").eq(self.bench)).tolist()
        containers: list[docker.models.containers.Container] = self._docker_client.containers.list(
            all=True
        )
        containers_by_id = {cast(str, c.id): c for c in containers}

        async with self.host.session(commit=True):
            for computer in computers:
                if computer.external_id is None:
                    continue
                container = containers_by_id.get(computer.external_id)
                if container is None:
                    computer.status = ResourceStatus.DECLARED

    @override
    async def _do_provision(self, resource: Computer):
        bench_dir = _get_bench_dir()
        assigned_port = random.randint(60100, 65000)
        env_vars = _get_computer_env_vars(resource, is_trusted=True, is_in_docker=True)
        computer_id_prefix = str(resource.id).split("-")[0]
        external_name = (
            f"bench-{ENV.value}-{CLOUD.slug}-{resource.region.slug}-computer-{computer_id_prefix}"
        )
        container = self._docker_client.containers.run(
            f"{COMPUTER_RUNTIME_IMAGE}:{resource.version}",
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
            resource.status = ResourceStatus.UP
            resource.connection_uri = f"http://localhost:{assigned_port}"

    @override
    async def _do_update(self, resource: Computer):
        pass  # nothing to do?

    @override
    async def _do_decommission(self, resource: Computer):
        # remove container with same external_id if exists
        assert resource.external_id is not None, f"{resource!r} has no external id"
        container = self._docker_client.containers.get(resource.external_id)
        if container is not None:
            container.remove(force=True)
        async with self.host.session(commit=True):
            resource.status = ResourceStatus.DECOMMISSIONED


class KubernetesComputerProvisioner(Provisioner[Computer, Computer]):
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
        external_name = (
            f"bench-{ENV.value}-{CLOUD.slug}-{computer.region.slug}-computer-{computer_id_prefix}"
        )
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

    def _make_pod_from_computer(self, computer: Computer):
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
        # TODO :Security!: kubernetes-deployed computers should not be trusted
        env_vars = _get_computer_env_vars(
            computer, is_trusted=True, is_in_minikube=IS_DEV or IS_TEST
        )

        # pod
        resources = k8.V1ResourceRequirements(
            requests=self._get_pod_resources_requests(computer),
            limits=self._get_pod_resources_limits(computer),
        )
        health_probe = k8.V1Probe(
            grpc=k8.V1GRPCAction(port=COMPUTER_PORT, service="runtime"),
            initial_delay_seconds=10,
            period_seconds=10,
            failure_threshold=3,
        )
        main_container = k8.V1Container(
            name="main",
            image=f"{COMPUTER_RUNTIME_IMAGE}:{computer.version}",
            command=["python", "bench.py", "serve", "runtime", "0.0.0.0", str(COMPUTER_PORT)],
            env=[
                *(k8.V1EnvVar(name=k, value=v) for k, v in env_vars.items()),
                k8.V1EnvVar(
                    name="KUBERNETES_NODE_ID",
                    value_from=k8.V1EnvVarSource(
                        field_ref=k8.V1ObjectFieldSelector(field_path="spec.nodeName")
                    ),
                ),
            ],
            ports=[k8.V1ContainerPort(container_port=COMPUTER_PORT)],
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
        pod_phase = cast(str, pod.status.phase) if pod.status else None  # type: ignore
        status = ResourceStatus.UP if pod_phase == "Running" else ResourceStatus.DOWN
        if computer.status != status:
            computer.status = status

        # connection uri (using pod ip, only works inside cluster for now)
        if pod.status and pod.status.pod_ip:  # type: ignore
            connection_uri = f"http://{pod.status.pod_ip}:{COMPUTER_PORT}"  # type: ignore
            if computer.connection_uri != connection_uri:
                computer.connection_uri = connection_uri

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
                        computer.status = ResourceStatus.DECOMMISSIONED
                else:
                    assert_never(event_type)

    @override
    async def _do_start(self) -> None:
        computers = await Computer.where(Computer.get_property("bench").eq(self.bench)).tolist()
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
                    computer.status = ResourceStatus.DECLARED

        # and keep watching for pod changes
        self.tasks.run(
            self._do_watch_pods(label_selector=pod_selector, resource_version=pod_marker),
            task_id=f"{self.bench.slug}.kubernetes.watch_pods",
        )

    @override
    async def _do_provision(self, resource: Computer):
        pod = self._make_pod_from_computer(resource)
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
            pod = self._make_pod_from_computer(resource)
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
            resource.status = ResourceStatus.DECOMMISSIONED

    @override
    async def wait_closed(self) -> None:
        await super().wait_closed()
        if self._kubernetes_api is not None:
            await self._kubernetes_api.close()
            self._kubernetes_api = None
