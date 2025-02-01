from typing import TYPE_CHECKING, Any, Literal, Optional, assert_never

import boto3
import botocore.config

from bench.pb2 import MachineEnvironment
from bench.proto.network import dockerify_url, minikubeify_url
from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from mypy_boto3_s3 import S3Client
else:
    S3Client = Any


_s3_client_by_localized: dict[Any, S3Client] = {}


# get the S3 client if needed to avoid requiring its env vars everywhere
def get_s3_client(*, localize_for: Literal["docker", "minikube"] | None = None) -> S3Client:
    """Gets the S3 client localized for the given environment."""

    if localize_for in _s3_client_by_localized:
        return _s3_client_by_localized[localize_for]

    S3_REGION = get_from_env("S3_REGION", description="S3 region")
    S3_ENDPOINT = get_from_env("S3_ENDPOINT", description="S3 endpoint URL")
    S3_ACCESS_KEY = get_from_env("S3_ACCESS_KEY", description="S3 access key")
    S3_SECRET_KEY = get_from_env("S3_SECRET_KEY", description="S3 secret key")

    if localize_for == "docker":
        S3_ENDPOINT = dockerify_url(S3_ENDPOINT)
    elif localize_for == "minikube":
        S3_ENDPOINT = minikubeify_url(S3_ENDPOINT)
    elif localize_for is not None:
        assert_never(localize_for)

    s3_client = boto3.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        config=botocore.config.Config(signature_version="s3v4", region_name=S3_REGION),
    )
    _s3_client_by_localized[localize_for] = s3_client
    return s3_client


def get_s3_client_for_presigning(zone: Optional[MachineEnvironment]) -> S3Client:
    """Gets the S3 client for presigning URLs."""
    if zone is None or zone == MachineEnvironment.REGULAR:
        return get_s3_client()
    elif zone == MachineEnvironment.DOCKER:
        return get_s3_client(localize_for="docker")
    elif zone == MachineEnvironment.MINIKUBE:
        return get_s3_client(localize_for="minikube")
    else:
        raise ValueError(f"unexpected machine environment: {zone}")
