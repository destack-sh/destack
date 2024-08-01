from typing import TYPE_CHECKING, Any, Optional

import boto3
import botocore.config

from bench.utils.utils import get_from_env

if TYPE_CHECKING:
    from mypy_boto3_s3 import S3Client
else:
    S3Client = Any

S3_PRESIGNED_URL_EXPIRY = get_from_env(
    "S3_PRESIGNED_URL_EXPIRY",
    typ=int,
    default=3600,
    description="S3 presigned URL expiry (in seconds)",
)

_s3_client: Optional[S3Client] = None


# get the S3 client if needed to avoid requiring its env vars everywhere
def get_s3_client() -> S3Client:
    global _s3_client
    if _s3_client is not None:
        return _s3_client

    S3_REGION = get_from_env("S3_REGION", description="S3 region")
    S3_ENDPOINT = get_from_env("S3_ENDPOINT", description="S3 endpoint URL")
    S3_ACCESS_KEY = get_from_env("S3_ACCESS_KEY", description="S3 access key")
    S3_SECRET_KEY = get_from_env("S3_SECRET_KEY", description="S3 secret key")

    _s3_client = boto3.client(
        "s3",
        endpoint_url=S3_ENDPOINT,
        aws_access_key_id=S3_ACCESS_KEY,
        aws_secret_access_key=S3_SECRET_KEY,
        config=botocore.config.Config(signature_version="s3v4", region_name=S3_REGION),
    )
    return _s3_client
