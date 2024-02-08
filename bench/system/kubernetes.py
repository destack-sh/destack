import asyncio
import base64
import enum
import subprocess
import time
from collections import defaultdict
from dataclasses import dataclass
from typing import AsyncIterator, Optional
from uuid import UUID

import aiostream
import structlog
from kubernetes import config as sync_config
from kubernetes_asyncio import client, config, watch

from bench import settings
from bench.language.const import BenchRegion, ServerProfile, ServerStatus
from bench.utils.utils import get_from_env
from bench.utils.env import IS_DEBUG

logger = structlog.get_logger(__name__)

k8_init = asyncio.Event()

SERVER_APP_LABEL = "user-server"
KUBERNETES_AVAILABLE = False
KUBERNETES_KUBECONFIG_PATH = get_from_env("KUBERNETES_KUBECONFIG_PATH", optional=True)
KUBERNETES_KUBECONFIG_CTX = get_from_env("KUBERNETES_KUBECONFIG_CTX", optional=True)
KUBERNETES_SERVER_IMAGE = get_from_env("KUBERNETES_SERVER_IMAGE", optional=True)
KUBERNETES_SERVER_NAMESPACE = get_from_env("KUBERNETES_SERVER_NAMESPACE", default="default")
KUBERNETES_SERVER_ENV_VARS_STR = get_from_env("KUBERNETES_SERVER_ENV_VARS", optional=True)
KUBERNETES_SERVER_IMAGE_PULL_SECRET_NAME = get_from_env(
    "KUBERNETES_SERVER_IMAGE_PULL_SECRET_NAME", optional=True
)
KUBERNETES_ENABLED = get_from_env("KUBERNETES_ENABLED", default=not IS_DEBUG, type_cast=bool)


if KUBERNETES_SERVER_IMAGE is not None and IS_DEBUG:
    image_name, image_version = KUBERNETES_SERVER_IMAGE.split(":")
    if image_version == "latest":
        # use current git commit hash as image version
        image_version = (
            subprocess.check_output(["git", "rev-parse", "--short", "HEAD"]).decode("utf-8").strip()
        )
    KUBERNETES_SERVER_IMAGE = f"{image_name}:{image_version}"


async def init():
    # TODO @Cleanup: somehow kubernetes asyncio sometimes stalls while authenticating
    # so we use the sync kubernetes client, then patch the config into the async client
    global KUBERNETES_AVAILABLE
    if KUBERNETES_KUBECONFIG_PATH is not None:
        sync_config.load_kube_config(KUBERNETES_KUBECONFIG_PATH, KUBERNETES_KUBECONFIG_CTX)
        KUBERNETES_AVAILABLE = True
    elif not IS_DEBUG:
        sync_config.load_incluster_config()
        KUBERNETES_AVAILABLE = True

    if KUBERNETES_AVAILABLE:
        config.kube_config.Configuration.set_default(
            sync_config.kube_config.Configuration.get_default_copy()
        )

    logger.info("k8_init", K8_AVAILABLE=KUBERNETES_AVAILABLE)
    k8_init.set()


# :ServerProfiles
SERVER_RESOURCES_BY_PROFILE = {
    ServerProfile.TINY: client.V1ResourceRequirements(
        requests={"cpu": "250m", "memory": "250Mi"},
        limits={"cpu": "500m", "memory": "500Mi"},
    ),
    ServerProfile.SMALL: client.V1ResourceRequirements(
        requests={"cpu": "0.5", "memory": "1Gi"},
        limits={"cpu": "1", "memory": "2Gi"},
    ),
    ServerProfile.MEDIUM: client.V1ResourceRequirements(
        requests={"cpu": "1", "memory": "2Gi"},
        limits={"cpu": "2", "memory": "4Gi"},
    ),
}

BASE_SERVER_ENV_VARS: list[client.V1EnvVar] = []
try:
    if KUBERNETES_SERVER_ENV_VARS_STR is not None:
        decoded_vars = base64.b64decode(KUBERNETES_SERVER_ENV_VARS_STR).decode().split(";")
        for part in decoded_vars:
            k, v = part.split("=")
            BASE_SERVER_ENV_VARS.append(client.V1EnvVar(name=k, value=v))
except Exception as e:
    logger.exception(
        f"failed to parse server env vars: {KUBERNETES_SERVER_ENV_VARS_STR}", exc_info=e
    )
    if not IS_DEBUG:
        raise


def _get_deployment_status(deployment: client.V1Deployment) -> ServerStatus:
    """Maps K8 deployment status to ServerSetStatus."""
    if not deployment.status.conditions:
        return ServerStatus.UNKNOWN
    if (deployment.status.replicas or 0) == 0:
        return ServerStatus.SLEEPING
    if deployment.status.available_replicas == deployment.status.replicas:
        return ServerStatus.HEALTHY
    for condition in deployment.status.conditions:
        if condition.type == "Progressing":
            if condition.status == "True":
                if condition.reason in [
                    "NewReplicaSetCreated",
                    "NewReplicaSetAvailable",
                    "FoundNewReplicaSet",
                    "ReplicaSetUpdated",
                ]:
                    return ServerStatus.UPDATING
            elif condition.status == "False" and condition.reason == "ProgressDeadlineExceeded":
                return ServerStatus.UNHEALTHY
        elif condition.type == "Available":
            if condition.status == "False":
                return ServerStatus.PENDING
        elif condition.type == "ReplicaFailure":
            if condition.status == "True":
                return ServerStatus.UNHEALTHY
    if (deployment.status.unavailable_replicas or 0) == deployment.status.replicas:
        return ServerStatus.UNAVAILABLE
    if (deployment.status.unavailable_replicas or 0) > 0:
        return ServerStatus.UNHEALTHY
    return ServerStatus.UNKNOWN


@dataclass
class Pod:
    name: str
    deployment_name: str
    bench_id: UUID
    server_set_id: UUID
    region: BenchRegion
    profile: ServerProfile

    def __str__(self):
        return f"{self.name} ({self.deployment_name}, {self.region}, {self.profile})"

    def __repr__(self):
        return f"<Pod {self}>"

    @classmethod
    def from_k8(cls, pod: client.V1Pod) -> "Pod":
        labels = pod.metadata.labels
        return cls(
            name=pod.metadata.name,
            deployment_name=labels["deployment"],
            bench_id=UUID(labels["bench_id"]),
            server_set_id=UUID(labels["server_set_id"]),
            region=BenchRegion(labels["region"].upper()),
            profile=ServerProfile(labels["profile"].upper()),
        )


@dataclass
class Deployment:
    bench_id: UUID
    server_set_id: UUID
    region: BenchRegion
    profile: ServerProfile
    target_replicas: int
    # read from k8
    active_replicas_ids: list[str] = None
    available_replicas: Optional[int] = None
    ready_replicas: Optional[int] = None
    status: Optional[ServerStatus] = None

    def __str__(self):
        return f"{self.name} ({self.status}, {self.ready_replicas}/{self.target_replicas}, {self.region}, {self.profile})"

    def __repr__(self):
        return f"<Deployment {self}>"

    @property
    def name(self):
        return f"{SERVER_APP_LABEL}-{self.bench_id}-{self.server_set_id.hex[:6]}"

    def to_k8(self: "Deployment", bench: "models.Bench") -> client.V1Deployment:
        namespace = settings.KUBERNETES_SERVER_NAMESPACE
        env_vars: dict[str, str] = {
            "SERVER_BENCH_ID": str(self.bench_id),
            "SERVER_SET_ID": str(self.server_set_id),
            # local opensearch auth
            "LOCAL_OS_HOST": get_from_env("GLOBAL_OS_HOST"),
            "LOCAL_OS_NAME": bench.os_name,
            "LOCAL_OS_PORT": get_from_env("GLOBAL_OS_PORT"),
            "LOCAL_OS_USERNAME": bench.os_username,
            "LOCAL_OS_PASSWORD": bench.os_password,
            # global postgres auth
            "LOCAL_PG_HOST": get_from_env("USER_PG_HOST"),
            "LOCAL_PG_NAME": bench.pg_name,
            "LOCAL_PG_PORT": get_from_env("USER_PG_PORT"),
            "LOCAL_PG_USERNAME": bench.pg_username,
            "LOCAL_PG_PASSWORD": bench.pg_password,
        }
        extended_env_vars = [
            *(client.V1EnvVar(name=k, value=v) for k, v in env_vars.items()),
            # server node id from k8
            client.V1EnvVar(
                name="SERVER_ID",
                value_from=client.V1EnvVarSource(
                    field_ref=client.V1ObjectFieldSelector(field_path="spec.nodeName")
                ),
            ),
            *BASE_SERVER_ENV_VARS,
        ]
        container = client.V1Container(
            name=self.name,
            image=KUBERNETES_SERVER_IMAGE,
            image_pull_policy="Always",
            ports=[client.V1ContainerPort(container_port=80, name="http")],
            resources=SERVER_RESOURCES_BY_PROFILE[self.profile],
            env=extended_env_vars,
            command=["python", "manageserver.py", "host"],
            readiness_probe=client.V1Probe(
                # /healthz on port 80, see :ServerHealthProbe
                http_get=client.V1HTTPGetAction(path="/ready", port=80),
                initial_delay_seconds=5,
                period_seconds=10,
                failure_threshold=3,
            ),
            liveness_probe=client.V1Probe(
                # /healthz on port 80, see :ServerHealthProbe
                http_get=client.V1HTTPGetAction(path="/healthz", port=80),
                initial_delay_seconds=5,
                period_seconds=10,
                failure_threshold=3,
            ),
        )
        labels = {
            "app": SERVER_APP_LABEL,
            "deployment": self.name,
            "bench_id": str(self.bench_id),
            "server_set_id": str(self.server_set_id),
            "region": self.region.lower(),
            "profile": self.profile.lower(),
        }
        template = client.V1PodTemplateSpec(
            metadata=client.V1ObjectMeta(namespace=namespace, labels=labels),
            spec=client.V1PodSpec(
                containers=[container],
                image_pull_secrets=[
                    client.V1LocalObjectReference(
                        name=settings.KUBERNETES_SERVER_IMAGE_PULL_SECRET_NAME
                    )
                ],
                termination_grace_period_seconds=20,
            ),
        )
        deployment = client.V1Deployment(
            metadata=client.V1ObjectMeta(namespace=namespace, name=self.name, labels=labels),
            spec=client.V1DeploymentSpec(
                replicas=self.target_replicas,
                selector=client.V1LabelSelector(match_labels=labels),
                template=template,
            ),
        )
        return deployment

    @classmethod
    def from_k8(cls, deployment: client.V1Deployment, pods: list[Pod] | None) -> "Deployment":
        """Gets partial deployment info from k8."""
        # check conditions (and reasons) for status (updating, healthy, unhealthy)
        labels = deployment.metadata.labels
        return cls(
            bench_id=UUID(labels["bench_id"]),
            server_set_id=UUID(labels["server_set_id"]),
            region=BenchRegion(labels["region"].upper()),
            profile=ServerProfile(labels["profile"].upper()),
            status=_get_deployment_status(deployment),
            target_replicas=deployment.spec.replicas,
            active_replicas_ids=[pod.name for pod in pods] if pods is not None else None,
            available_replicas=deployment.status.available_replicas or 0,
            ready_replicas=deployment.status.ready_replicas or 0,
        )

    @classmethod
    def from_model(cls, model: models.ServerSet) -> "Deployment":
        return cls(
            bench_id=model.bench_id,
            server_set_id=model.id,
            region=model.region,
            profile=model.profile,
            target_replicas=model.target_replicas,
        )

    def to_model(self) -> models.ServerSet:
        return models.ServerSet(
            id=self.server_set_id,
            bench_id=self.bench_id,
            region=self.region,
            profile=self.profile,
            target_replicas=self.target_replicas,
            active_replicas_ids=self.active_replicas_ids,
            available_replicas=self.available_replicas,
            ready_replicas=self.ready_replicas,
            status=self.status,
        )


def _check_k8_available():
    if not KUBERNETES_AVAILABLE:
        raise RuntimeError("Kubernetes API is not available")


async def update_deployments(benches: list[models.Bench], deployments: list[Deployment]) -> None:
    """Upserts deployments in k8."""
    _check_k8_available()
    async with client.ApiClient() as api:
        for bench, deployment in zip(benches, deployments):
            k8_deployment = deployment.to_k8(bench)
            try:
                logger.info("k8.deployment.update", deployment=deployment)
                await client.AppsV1Api(api).replace_namespaced_deployment(
                    name=k8_deployment.metadata.name,
                    namespace=k8_deployment.metadata.namespace,
                    body=k8_deployment,
                )
            except client.ApiException as e:
                if e.status == 404:
                    logger.info("k8.deployment.create", deployment=deployment)
                    await client.AppsV1Api(api).create_namespaced_deployment(
                        namespace=k8_deployment.metadata.namespace, body=k8_deployment
                    )
                else:
                    raise


async def delete_deployments(deployments: list[Deployment]) -> None:
    """Deletes deployments in k8."""
    _check_k8_available()
    async with client.ApiClient() as api:
        for deployment in deployments:
            logger.info("k8.deployment.delete", deployment=deployment)
            try:
                await client.AppsV1Api(api).delete_namespaced_deployment(
                    name=deployment.name, namespace=KUBERNETES_SERVER_NAMESPACE
                )
            except client.ApiException as e:
                logger.warning("k8.deployment.delete.failed", deployment=deployment, error=e)
                if e.status == 404:
                    pass
                else:
                    raise


async def restart_deployment(deployment: Deployment) -> None:
    _check_k8_available()
    logger.info("k8.deployment.restart", deployment=deployment)
    async with client.ApiClient() as api:
        annotations_patch = {"kubectl.kubernetes.io/restartedAt": str(int(time.time()))}
        await client.AppsV1Api(api).patch_namespaced_deployment(
            name=deployment.name,
            namespace=KUBERNETES_SERVER_NAMESPACE,
            body={"spec": {"template": {"metadata": {"annotations": annotations_patch}}}},
        )


@dataclass
class VersionMarker:
    deployment_resource_version: str
    pod_resource_version: str


async def get_all_deployments() -> tuple[list[Deployment], VersionMarker]:
    """Gets all bench server set deployments at the latest version."""
    _check_k8_available()
    logger.info("k8.deployment.get_all")
    selector = f"app={SERVER_APP_LABEL}"
    async with client.ApiClient() as api:
        apps = client.AppsV1Api(api)
        core = client.CoreV1Api(api)
        deployments = await apps.list_deployment_for_all_namespaces(label_selector=selector)
        pods = await core.list_pod_for_all_namespaces(label_selector=selector)
    mark = VersionMarker(
        deployment_resource_version=deployments.metadata.resource_version,
        pod_resource_version=pods.metadata.resource_version,
    )
    pods = [Pod.from_k8(p) for p in pods.items]
    pods_by_deployment: dict[str, list[Pod]] = defaultdict(list)
    for pod in pods:
        pods_by_deployment[pod.deployment_name].append(pod)
    deployments = [
        Deployment.from_k8(d, pods_by_deployment[d.metadata.name]) for d in deployments.items
    ]
    return deployments, mark


class EventType(enum.StrEnum):
    ADDED = "ADDED"
    MODIFIED = "MODIFIED"
    DELETED = "DELETED"


async def watch_our_deployments(
    version_info: VersionMarker,
) -> AsyncIterator[tuple[EventType, Deployment | Pod]]:
    """Watches all bench server set deployments and their nodes."""
    _check_k8_available()
    selector = f"app={SERVER_APP_LABEL}"
    logger.info("k8.deployment.watch", selector=selector)
    async with client.ApiClient() as api, watch.Watch().stream(
        client.CoreV1Api(api).list_pod_for_all_namespaces,
        label_selector=selector,
        resource_version=version_info.pod_resource_version,
    ) as pod_stream, watch.Watch().stream(
        client.AppsV1Api(api).list_deployment_for_all_namespaces,
        label_selector=selector,
        resource_version=version_info.deployment_resource_version,
    ) as deployment_stream, aiostream.stream.merge(
        pod_stream, deployment_stream
    ).stream() as combined_stream:
        # combine streams
        async for event in combined_stream:
            # map kubernetes event to EventType
            event_type = EventType(event["type"])
            # map kubernetes event to deployment or pod
            if event["object"].type == "Deployment":
                yield event_type, Deployment.from_k8(event["object"], None)
            elif event["object"].type == "Pod":
                yield event_type, Pod.from_k8(event["object"])
            else:
                continue  # ignore other events?
