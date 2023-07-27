import asyncio
import base64
import enum
import subprocess
import time
from collections import defaultdict
from dataclasses import dataclass
from typing import AsyncIterator, Optional
from uuid import UUID

import structlog
from kubernetes import config as sync_config
from kubernetes_asyncio import client, config, watch

from bench import models, settings
from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.settings.k8 import (
    KUBERNETES_WORKER_ENV_VARS_STR,
    KUBERNETES_WORKER_IMAGE,
    KUBERNETES_WORKER_NAMESPACE,
)
from bench.utils.utils import DEBUG, LOCAL

logger = structlog.get_logger(__name__)

k8_init = asyncio.Event()

BENCH_WORKER_APP = "bench-worker"
K8_AVAILABLE = False

if KUBERNETES_WORKER_IMAGE is not None and DEBUG:
    image_name, image_version = KUBERNETES_WORKER_IMAGE.split(":")
    if image_version == "latest":
        # use current git commit hash as image version
        image_version = (
            subprocess.check_output(["git", "rev-parse", "--short", "HEAD"]).decode("utf-8").strip()
        )
    KUBERNETES_WORKER_IMAGE = f"{image_name}:{image_version}"


async def init():
    # TODO @Cleanup: somehow kubernetes asyncio sometimes stalls while authenticating
    # so we use the sync kubernetes client, then patch the config into the async client
    global K8_AVAILABLE
    if settings.KUBERNETES_KUBECONFIG_PATH is not None:
        sync_config.load_kube_config(
            settings.KUBERNETES_KUBECONFIG_PATH, settings.KUBERNETES_KUBECONFIG_CTX
        )
        K8_AVAILABLE = True
    elif not LOCAL:
        sync_config.load_incluster_config()
        K8_AVAILABLE = True

    if K8_AVAILABLE:
        config.kube_config.Configuration.set_default(
            sync_config.kube_config.Configuration.get_default_copy()
        )

    logger.info("k8_init", K8_AVAILABLE=K8_AVAILABLE)
    k8_init.set()


# :WorkerProfiles
WORKER_RESOURCES_BY_PROFILE = {
    WorkerProfile.TINY: client.V1ResourceRequirements(
        requests={"cpu": "250m", "memory": "250Mi"},
        limits={"cpu": "500m", "memory": "500Mi"},
    ),
    WorkerProfile.SMALL: client.V1ResourceRequirements(
        requests={"cpu": "0.5", "memory": "1Gi"},
        limits={"cpu": "1", "memory": "2Gi"},
    ),
    WorkerProfile.MEDIUM: client.V1ResourceRequirements(
        requests={"cpu": "1", "memory": "2Gi"},
        limits={"cpu": "2", "memory": "4Gi"},
    ),
    WorkerProfile.LARGE: client.V1ResourceRequirements(
        requests={"cpu": "2", "memory": "4Gi"},
        limits={"cpu": "4", "memory": "8Gi"},
    ),
    WorkerProfile.XLARGE_CPU: client.V1ResourceRequirements(
        requests={"cpu": "4", "memory": "8Gi"},
        limits={"cpu": "8", "memory": "16Gi"},
    ),
    WorkerProfile.XLARGE_MEM: client.V1ResourceRequirements(
        requests={"cpu": "2", "memory": "32Gi"},
        limits={"cpu": "4", "memory": "64Gi"},
    ),
}

BASE_WORKER_ENV_VARS: list[client.V1EnvVar] = []
try:
    decoded_vars = base64.b64decode(KUBERNETES_WORKER_ENV_VARS_STR).decode().split(";")
    for part in decoded_vars:
        k, v = part.split("=")
        BASE_WORKER_ENV_VARS.append(client.V1EnvVar(name=k, value=v))
except Exception as e:
    logger.exception(
        f"failed to parse worker env vars: {KUBERNETES_WORKER_ENV_VARS_STR}", exc_info=e
    )
    if not DEBUG:
        raise


def _get_deployment_status(deployment: client.V1Deployment) -> WorkerSetStatus:
    """Maps K8 deployment status to WorkerSetStatus."""
    if not deployment.status.conditions:
        return WorkerSetStatus.UNKNOWN
    # if target and actual replicas is 0 then the deployment is SLEEPING
    if deployment.status.replicas == 0 and deployment.status.available_replicas == 0:
        return WorkerSetStatus.SLEEPING
    for condition in deployment.status.conditions:
        if condition.type == "Progressing":
            if condition.status == "True":
                if condition.reason in [
                    "NewReplicaSetCreated",
                    "FoundNewReplicaSet",
                    "ReplicaSetUpdated",
                ]:
                    return WorkerSetStatus.UPDATING
                elif condition.reason == "NewReplicaSetAvailable":
                    return WorkerSetStatus.HEALTHY
            elif condition.status == "False":
                if condition.reason == "ProgressDeadlineExceeded":
                    return WorkerSetStatus.UNHEALTHY
        elif condition.type == "Available":
            if condition.status == "False":
                return WorkerSetStatus.UNHEALTHY
        elif condition.type == "ReplicaFailure":
            if condition.status == "True":
                return WorkerSetStatus.UNHEALTHY
    return WorkerSetStatus.UNKNOWN


@dataclass
class Pod:
    name: str
    deployment_name: str
    project_id: UUID
    worker_set_id: UUID
    region: WorkerRegion
    profile: WorkerProfile

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
            project_id=UUID(labels["project_id"]),
            worker_set_id=UUID(labels["worker_set_id"]),
            region=WorkerRegion(labels["region"].upper()),
            profile=WorkerProfile(labels["profile"].upper()),
        )


@dataclass
class Deployment:
    project_id: UUID
    worker_set_id: UUID
    region: WorkerRegion
    profile: WorkerProfile
    target_replicas: int
    # read from k8
    active_replicas_ids: list[str] = None
    available_replicas: Optional[int] = None
    ready_replicas: Optional[int] = None
    status: Optional[WorkerSetStatus] = None

    def __str__(self):
        return f"{self.name} ({self.ready_replicas}/{self.target_replicas}, {self.region}, {self.profile})"

    def __repr__(self):
        return f"<Deployment {self}>"

    @property
    def name(self):
        return f"{BENCH_WORKER_APP}-{self.project_id}-{self.worker_set_id.hex[:6]}"

    def to_k8(self: "Deployment") -> client.V1Deployment:
        namespace = settings.KUBERNETES_WORKER_NAMESPACE
        extended_env_vars = [
            client.V1EnvVar(name="WORKER_PROJECT_ID", value=str(self.project_id)),
            client.V1EnvVar(name="WORKER_SET_ID", value=str(self.worker_set_id)),
            # worker node id from k8
            client.V1EnvVar(
                name="WORKER_NODE_ID",
                value_from=client.V1EnvVarSource(
                    field_ref=client.V1ObjectFieldSelector(field_path="spec.nodeName")
                ),
            ),
            *BASE_WORKER_ENV_VARS,
        ]
        container = client.V1Container(
            name=self.name,
            image=KUBERNETES_WORKER_IMAGE,
            image_pull_policy="Always",
            ports=[client.V1ContainerPort(container_port=80, name="http")],
            resources=WORKER_RESOURCES_BY_PROFILE[self.profile],
            env=extended_env_vars,
            command=["python", "manageworker.py", "host"],
            liveness_probe=client.V1Probe(
                # /healthz on port 80, see :WorkerHealthProbe
                http_get=client.V1HTTPGetAction(path="/healthz", port=80),
                initial_delay_seconds=5,
                period_seconds=10,
                failure_threshold=3,
            ),
        )
        labels = {
            "app": BENCH_WORKER_APP,
            "deployment": self.name,
            "project_id": str(self.project_id),
            "worker_set_id": str(self.worker_set_id),
            "region": self.region.lower(),
            "profile": self.profile.lower(),
        }
        template = client.V1PodTemplateSpec(
            metadata=client.V1ObjectMeta(namespace=namespace, labels=labels),
            spec=client.V1PodSpec(
                containers=[container],
                image_pull_secrets=[
                    client.V1LocalObjectReference(
                        name=settings.KUBERNETES_WORKER_IMAGE_PULL_SECRET_NAME
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
    def from_k8(cls, deployment: client.V1Deployment, pods: list[Pod]) -> "Deployment":
        """Gets partial deployment info from k8."""
        # check conditions (and reasons) for status (updating, healthy, unhealthy)
        labels = deployment.metadata.labels
        return cls(
            project_id=UUID(labels["project_id"]),
            worker_set_id=UUID(labels["worker_set_id"]),
            region=WorkerRegion(labels["region"].upper()),
            profile=WorkerProfile(labels["profile"].upper()),
            status=_get_deployment_status(deployment),
            target_replicas=deployment.spec.replicas,
            active_replicas_ids=[pod.name for pod in pods],
            available_replicas=deployment.status.available_replicas or 0,
            ready_replicas=deployment.status.ready_replicas or 0,
        )

    @classmethod
    def from_model(cls, model: models.WorkerSet) -> "Deployment":
        return cls(
            project_id=model.project_id,
            worker_set_id=model.id,
            region=model.region,
            profile=model.profile,
            target_replicas=model.target_replicas,
        )

    def to_model(self) -> models.WorkerSet:
        return models.WorkerSet(
            id=self.worker_set_id,
            project_id=self.project_id,
            region=self.region,
            profile=self.profile,
            target_replicas=self.target_replicas,
            active_replicas_ids=self.active_replicas_ids,
            available_replicas=self.available_replicas,
            ready_replicas=self.ready_replicas,
            status=self.status,
        )


def _check_k8_available():
    if not K8_AVAILABLE:
        raise RuntimeError("Kubernetes API is not available")


async def update_deployments(deployments: list[Deployment]) -> None:
    """Upserts deployments in k8."""
    _check_k8_available()
    async with client.ApiClient() as api:
        for deployment in deployments:
            k8_deployment = deployment.to_k8()
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


async def restart_deployment(deployment: Deployment) -> None:
    _check_k8_available()
    logger.info("k8.deployment.restart", deployment=deployment)
    async with client.ApiClient() as api:
        annotations_patch = {"kubectl.kubernetes.io/restartedAt": str(int(time.time()))}
        await client.AppsV1Api(api).patch_namespaced_deployment(
            name=deployment.name,
            namespace=KUBERNETES_WORKER_NAMESPACE,
            body={"spec": {"template": {"metadata": {"annotations": annotations_patch}}}},
        )


async def get_all_deployments() -> list[Deployment]:
    """Gets all bench worker set deployments."""
    _check_k8_available()
    selector = f"app={BENCH_WORKER_APP}"
    async with client.ApiClient() as api:
        apps = client.AppsV1Api(api)
        core = client.CoreV1Api(api)
        deployments = await apps.list_deployment_for_all_namespaces(label_selector=selector)
        pods = await core.list_pod_for_all_namespaces(label_selector=selector)
    pods = [Pod.from_k8(p) for p in pods.items]
    pods_by_deployment: dict[str, list[Pod]] = defaultdict(list)
    for pod in pods:
        pods_by_deployment[pod.deployment_name].append(pod)
    deployments = [
        Deployment.from_k8(d, pods_by_deployment[d.metadata.name]) for d in deployments.items
    ]
    return deployments


class EventType(enum.StrEnum):
    ADDED = "ADDED"
    MODIFIED = "MODIFIED"
    DELETED = "DELETED"


async def watch_our_deployments() -> AsyncIterator[tuple[EventType, Deployment | Pod]]:
    """Watches all bench worker set deployments and their nodes."""
    _check_k8_available()
    selector = f"app={BENCH_WORKER_APP}"
    logger.info("k8.deployment.watch", selector=selector)
    async with client.ApiClient() as api:
        async with watch.Watch().stream(
            client.CoreV1Api(api).list_event_for_all_namespaces, label_selector=selector
        ) as stream:
            async for event in stream:
                # map kubernetes event to EventType
                event_type = EventType(event["type"])
                # map kubernetes event to deployment or pod
                if event["object"].get("involvedObject", {}).get("kind") == "Deployment":
                    deployment = client.V1Deployment(**event["object"])
                    yield event_type, Deployment.from_k8(deployment)
                elif event["object"].get("involvedObject", {}).get("kind") == "Pod":
                    pod = client.V1Pod(**event["object"])
                    yield event_type, Pod.from_k8(pod)
                else:
                    continue  # ignore other events?
