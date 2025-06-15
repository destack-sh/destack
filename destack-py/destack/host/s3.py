from typing import TYPE_CHECKING, Any, Literal

import boto3
import botocore.config

from destack.utils.env import get_from_env

if TYPE_CHECKING:
    from mypy_boto3_s3 import S3Client  # type: ignore
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

    s3_client = boto3.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        config=botocore.config.Config(signature_version="s3v4", region_name=S3_REGION),
    )
    _s3_client_by_localized[localize_for] = s3_client
    return s3_client
