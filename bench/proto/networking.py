from bench.pb2 import MachineEnvironment
from bench.utils.utils import get_from_env

IS_IN_DOCKER = get_from_env(
    "IS_IN_DOCKER", typ=bool, default=False, description="Whether we're running in Docker"
)
IS_IN_MINIKUBE = get_from_env(
    "IS_IN_MINIKUBE", typ=bool, default=False, description="Whether we're running in Minikube"
)
if IS_IN_DOCKER:
    MACHINE_ENVIRONMENT = MachineEnvironment.DOCKER
elif IS_IN_MINIKUBE:
    MACHINE_ENVIRONMENT = MachineEnvironment.MINIKUBE
else:
    MACHINE_ENVIRONMENT = MachineEnvironment.REGULAR


def localize_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside the current environment."""
    if IS_IN_DOCKER:
        return dockerify_url(domain)
    elif IS_IN_MINIKUBE:
        return minikubeify_url(domain)
    else:
        return domain


def dockerify_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside Docker."""
    domain = domain.replace("localhost", "host.docker.internal")
    domain = domain.replace("127.0.0.1", "host.docker.internal")
    return domain


def minikubeify_url(domain: str) -> str:
    """Converts a domain name to something we can reach inside Minikube."""
    domain = domain.replace("localhost", "host.minikube.internal")
    domain = domain.replace("127.0.0.1", "host.minikube.internal")
    return domain
