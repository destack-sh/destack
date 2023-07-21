from kubernetes import config

from bench import settings
from bench.utils.utils import LOCAL

if settings.KUBERNETES_KUBECONFIG_PATH is not None:
    config.load_kube_config(settings.KUBERNETES_KUBECONFIG_PATH)
elif not LOCAL:
    config.load_incluster_config()
