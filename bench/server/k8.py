import asyncio
import base64
import enum
import time
from dataclasses import dataclass
from typing import AsyncIterator, Optional
from uuid import UUID

import structlog
from kubernetes_asyncio import client, config, watch

from bench import models, settings
from bench.language.session import WorkerProfile, WorkerRegion, WorkerSetStatus
from bench.settings.k8 import (
    KUBERNETES_WORKER_ENV_VARS_STR,
    KUBERNETES_WORKER_IMAGE,
    KUBERNETES_WORKER_NAMESPACE,
)
from bench.utils.utils import LOCAL

logger = structlog.get_logger(__name__)

k8_init = asyncio.Event()

K8_AVAILABLE = False


async def init():
    global K8_AVAILABLE
    if settings.KUBERNETES_KUBECONFIG_PATH is not None:
        await config.load_kube_config(settings.KUBERNETES_KUBECONFIG_PATH)
        K8_AVAILABLE = True
    elif not LOCAL:
        await config.load_incluster_config()
        K8_AVAILABLE = True

    k8_init.set()


WORKER_RESOURCES_BY_PROFILE = {
    WorkerProfile.TINY: client.V1ResourceRequirements(
        requests={"cpu": "500m", "memory": "500Mi"},
        limits={"cpu": "500m", "memory": "500Mi"},
    ),
    WorkerProfile.SMALL: client.V1ResourceRequirements(
        requests={"cpu": "1", "memory": "1Gi"},
        limits={"cpu": "1", "memory": "1Gi"},
    ),
    WorkerProfile.MEDIUM: client.V1ResourceRequirements(
        requests={"cpu": "2", "memory": "4Gi"},
        limits={"cpu": "2", "memory": "4Gi"},
    ),
    WorkerProfile.LARGE: client.V1ResourceRequirements(
        requests={"cpu": "4", "memory": "8Gi"},
        limits={"cpu": "4", "memory": "8Gi"},
    ),
    WorkerProfile.XLARGE_CPU: client.V1ResourceRequirements(
        requests={"cpu": "8", "memory": "16Gi"},
        limits={"cpu": "8", "memory": "16Gi"},
    ),
    WorkerProfile.XLARGE_MEM: client.V1ResourceRequirements(
        requests={"cpu": "4", "memory": "64Gi"},
        limits={"cpu": "4", "memory": "64Gi"},
    ),
}

BASE_WORKER_ENV_VARS: list[client.V1EnvVar] = []
try:
    for v in base64.decodestring(KUBERNETES_WORKER_ENV_VARS_STR).decode().split(";"):
        k, v = v.split("=")
        BASE_WORKER_ENV_VARS.append(client.V1EnvVar(name=k, value=v))
except Exception:
    logger.exception(f"failed to parse worker env vars: {KUBERNETES_WORKER_ENV_VARS_STR}")
    raise


def _get_deployment_status(deployment: client.V1Deployment) -> WorkerSetStatus:
    if not deployment.status.conditions:
        return WorkerSetStatus.UNKNOWN

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


DEPLOYMENT_DYNAMIC_FIELDS = ["available_replicas", "ready_replicas", "sleeping", "status"]


@dataclass
class Deployment:
    project_id: UUID
    worker_set_id: UUID
    region: WorkerRegion
    profile: WorkerProfile
    desired_replicas: int
    target_replicas: int
    # read from k8
    available_replicas: Optional[int] = None
    ready_replicas: Optional[int] = None
    status: Optional[WorkerSetStatus] = None

    @property
    def name(self):
        return f"bench-worker-{self.project_id}-{self.worker_set_id.hex[:6]}"

    def to_k8(self: "Deployment") -> client.V1Deployment:
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
            command=["python", "manageworker.py"],
        )
        labels = {
            "app": "bench-worker",
            "project_id": str(self.project_id),
            "worker_set_id": str(self.worker_set_id),
            "region": self.region.lower(),
            "profile": self.profile.lower(),
        }
        template = client.V1PodTemplateSpec(
            metadata=client.V1ObjectMeta(labels=labels),
            spec=client.V1PodSpec(
                containers=[container],
                image_pull_secrets=[
                    client.V1LocalObjectReference(name=settings.KUBERNETES_IMAGE_PULL_SECRET_NAME)
                ],
            ),
        )
        deployment = client.V1Deployment(
            metadata=client.V1ObjectMeta(name=self.name, labels=labels),
            spec=client.V1DeploymentSpec(
                replicas=self.target_replicas,
                selector=client.V1LabelSelector(match_labels=labels),
                template=template,
            ),
        )
        return deployment

    @classmethod
    def from_k8(cls, deployment: client.V1Deployment) -> "Deployment":
        """Gets partial deployment info from k8."""
        # check conditions (and reasons) for status (updating, healthy, unhealthy)
        labels = deployment.metadata.labels
        return cls(
            project_id=UUID(labels["project_id"]),
            worker_set_id=UUID(labels["worker_set_id"]),
            region=WorkerRegion(labels["region"]),
            profile=WorkerProfile(labels["profile"]),
            status=_get_deployment_status(deployment),
            target_replicas=deployment.spec.replicas,
            available_replicas=deployment.status.available_replicas,
            ready_replicas=deployment.status.ready_replicas,
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
            available_replicas=self.available_replicas,
            ready_replicas=self.ready_replicas,
        )


def _check_k8_available():
    if not K8_AVAILABLE:
        raise RuntimeError("Kubernetes API is not available")


async def update_deployments(deployments: list[Deployment]) -> None:
    _check_k8_available()
    api = client.AppsV1Api()
    for deployment in deployments:
        k8_deployment = deployment.to_k8()
        await api.replace_namespaced_deployment(
            k8_deployment.metadata.name, k8_deployment.metadata.namespace, k8_deployment
        )


async def restart_deployment(deployment: Deployment) -> None:
    _check_k8_available()
    api = client.AppsV1Api()
    await api.patch_namespaced_deployment(
        deployment.name,
        KUBERNETES_WORKER_NAMESPACE,
        {
            "spec": {
                "template": {
                    "metadata": {
                        "annotations": {"kubectl.kubernetes.io/restartedAt": str(int(time.time()))}
                    }
                }
            }
        },
    )


async def get_all_deployments() -> list[Deployment]:
    """Gets all bench worker set deployments."""
    _check_k8_available()
    api = client.AppsV1Api()
    deployments = await api.list_deployment_for_all_namespaces(
        label_selector="app=bench-worker",
    )
    return [Deployment.from_k8(d) for d in deployments.items]


class EventType(enum.StrEnum):
    ADDED = "ADDED"
    MODIFIED = "MODIFIED"
    DELETED = "DELETED"


async def watch_our_deployments() -> AsyncIterator[tuple[EventType, Deployment]]:
    """Watches all bench worker set deployments and their nodes."""
    _check_k8_available()
    v1 = client.CoreV1Api()
    async with watch.Watch().stream(
        v1.list_event_for_all_namespaces,
        label_selector="app=bench-worker",
    ) as stream:
        async for event in stream:
            # map kubernetes event to EventType
            event_type = EventType(event["type"])
            # map kubernetes event to deployment or pod
            if event["object"].get("involvedObject", {}).get("kind") == "Deployment":
                deployment = client.V1Deployment(**event["object"])
                yield event_type, Deployment.from_k8(deployment)
            else:
                continue  # ignore other events?
