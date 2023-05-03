import os
from functools import cache
from typing import Optional

import structlog
from django.db import models
from django_choices_field import TextChoicesField

from bench.models.project import get_project_bucket_name
from bench.models.utils import UUIDModel

REMOTE_OBJECT_HASH_LENGTH = 32

logger = structlog.get_logger(__name__)


class RemoteObjectStatus(models.TextChoices):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


REMOTE_OBJECT_PRESIGNED_POST_EXPIRY = 60 * 60  # 1 hour
REMOTE_OBJECT_PRESIGNED_GET_EXPIRY = 60 * 60 * 24  # 1 day


class RemoteObject(UUIDModel):
    """A pointer to a remotely stored object."""

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    prepared_at = models.DateTimeField(null=True, blank=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="remote_objects")
    md5 = models.CharField(max_length=REMOTE_OBJECT_HASH_LENGTH)
    content_length = models.IntegerField()
    content_type = models.CharField(max_length=255)
    name = models.CharField(max_length=255, null=True, blank=True)
    status = TextChoicesField(RemoteObjectStatus, default=RemoteObjectStatus.PREPARED)

    def delete(self, *args, **kwargs):
        if self.status == RemoteObjectStatus.AVAILABLE:
            try:
                s3_client = get_s3_client()
                s3_client.delete_object(
                    Bucket=get_project_bucket_name(self.project_id),
                    Key=str(self.id),
                )
            except Exception as e:
                logger.exception(f"failed to delete object {self}: {e}")
        super().delete(*args, **kwargs)

    @property
    def presigned_post(self) -> Optional[str]:
        return None  # must be set manually

    @property
    def presigned_get(self) -> Optional[str]:
        if self.status == RemoteObjectStatus.AVAILABLE:
            return self.generate_presigned_get()
        else:
            return None

    def generate_presigned_post(self) -> str:
        if self.status != RemoteObjectStatus.PREPARED:
            raise ValueError(f"cannot generate presigned post for {self} with status {self.status}")
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_post(
            Bucket=get_project_bucket_name(self.project_id),
            Key=str(self.id),
            ExpiresIn=REMOTE_OBJECT_PRESIGNED_POST_EXPIRY,
            Fields={
                "Content-Type": self.content_type,
                "Content-Length": str(self.content_length),
                "Name": self.name,
            },
        )
        if "url" not in response:
            raise RuntimeError(f"failed to generate presigned post for {self}: {response}")
        return response["url"]

    def generate_presigned_get(self) -> str:
        """Generate a presigned get url for this object."""
        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"cannot generate presigned get for {self} with status {self.status}")
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_url(
            ClientMethod="get_object",
            Params={
                "Bucket": get_project_bucket_name(self.project_id),
                "Key": str(self.id),
            },
            ExpiresIn=REMOTE_OBJECT_PRESIGNED_GET_EXPIRY,
        )
        if "url" not in response:
            raise RuntimeError(f"failed to generate presigned get for {self}: {response}")
        return response["url"]

    class Meta:
        constraints = [
            # deduplicate objects per project/md5
            models.UniqueConstraint(fields=["project", "md5"], name="bench_remoteobject_md5_ak")
        ]


@cache
def get_s3_client():
    import boto3
    from botocore.client import Config

    endpoint_url = os.environ.get("AWS_ENDPOINT_URL")
    return boto3.client(
        "s3", endpoint_url=endpoint_url, config=Config(s3={"addressing_style": "path"})
    )
