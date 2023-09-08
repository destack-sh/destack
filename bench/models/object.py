import os
import urllib
import urllib.parse
from functools import cache
from typing import Optional

import botocore
import structlog
from django.core.exceptions import ValidationError
from django.db import models

from bench.language.remote import REMOTE_OBJECT_HASH_LENGTH
from bench.models.utils import UUIDModel
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME

REMOTE_OBJECT_PRESIGNED_POST_EXPIRY = 60 * 60  # 1 hour
REMOTE_OBJECT_PRESIGNED_GET_EXPIRY = 60 * 60 * 24  # 1 day


def is_allowed_content_type(content_type: str) -> bool:
    # should we check anything here?
    return True


logger = structlog.get_logger(__name__)


# :RemoteObjectType
class RemoteObjectStatus(models.TextChoices):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


class RemoteObject(UUIDModel):
    """
    A pointer to a remotely stored object.
    """

    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    prepared_at = models.DateTimeField(null=True, blank=True)
    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="remote_objects")
    sha512 = models.CharField(max_length=REMOTE_OBJECT_HASH_LENGTH)
    content_length = models.IntegerField()
    content_type = models.CharField(max_length=255)
    name = models.CharField(max_length=255, null=True, blank=True)
    status = models.CharField(
        max_length=32,
        choices=RemoteObjectStatus.choices,
        default=RemoteObjectStatus.PREPARED,
    )

    _presigned_post: Optional[str] = None  # set manually
    _presigned_get: Optional[str] = None  # set manually

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<RemoteObject {self}>"

    @property
    def presigned_post(self) -> Optional[str]:
        return self._presigned_post  # must be set manually

    @property
    def presigned_get(self) -> Optional[str]:
        if self._presigned_get is not None:
            return self._presigned_get
        elif self.status == RemoteObjectStatus.AVAILABLE:
            self._presigned_get = self.generate_presigned_get()
            return self._presigned_get
        else:
            return None

    def mark_available_if_exists_in_s3(self):
        s3_client = get_s3_client()
        try:
            metadata = s3_client.head_object(
                Bucket=GLOBAL_PROJECT_BUCKET_NAME,
                Key=str(self.id),
            )
            if metadata["ContentLength"] != self.content_length:
                logger.warning(
                    "object_content_length_mismatch", remote_object=self, metadata=metadata
                )
            self.status = RemoteObjectStatus.AVAILABLE
        except botocore.exceptions.ClientError:
            logger.warning("object_not_found", remote_object=self)
            raise ValidationError(f"{self} not found in s3")

    def generate_presigned_post(self) -> str:
        if self.presigned_post is not None:
            return self.presigned_post
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_post(
            Bucket=GLOBAL_PROJECT_BUCKET_NAME,
            Key=str(self.id),
            ExpiresIn=REMOTE_OBJECT_PRESIGNED_POST_EXPIRY,
            Fields={},
        )
        if "url" not in response:
            raise RuntimeError(f"failed to generate presigned post for {self}: {response}")
        # encode the url as a string (with parameters)
        encoded_params = urllib.parse.urlencode(response["fields"])
        encoded_url = f"{response['url']}?{encoded_params}"
        self._presigned_post = encoded_url
        return self.presigned_post

    def generate_presigned_get(self) -> str:
        """Generate a presigned get url for this object."""
        if self.status != RemoteObjectStatus.AVAILABLE:
            raise ValueError(f"cannot generate presigned get for {self} with status {self.status}")
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_url(
            ClientMethod="get_object",
            Params={
                "Bucket": GLOBAL_PROJECT_BUCKET_NAME,
                "Key": str(self.id),
            },
            ExpiresIn=REMOTE_OBJECT_PRESIGNED_GET_EXPIRY,
        )
        return response

    class Meta:
        constraints = [
            # deduplicate objects per project/sha512
            models.UniqueConstraint(
                fields=["project", "sha512"], name="bench_remoteobject_sha512_ak"
            )
        ]


@cache
def get_s3_client():
    import boto3
    from botocore.client import Config

    endpoint_url = os.environ.get("AWS_ENDPOINT_URL")
    return boto3.client(
        "s3",
        endpoint_url=endpoint_url,
        config=Config(s3={"addressing_style": "path"}, region_name=os.environ.get("AWS_REGION")),
    )
