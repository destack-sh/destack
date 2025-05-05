from typing import TYPE_CHECKING, Literal, cast

import structlog
from kubernetes_asyncio import client as k8
from kubernetes_asyncio.client import ApiClient as KubernetesApiClient
from kubernetes_asyncio.client import CoreV1Api as KubernetesCoreV1Api
from opentelemetry import trace

from bench.utils.utils import get_from_env, get_from_env_maybe

if TYPE_CHECKING:
    pass

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

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
        """Patch a Pod."""
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

    async def watch_pods(self, *, label_selector: str, resource_version: str | None):
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
