import os
import urllib
import urllib.parse
from functools import cache
from typing import Optional

import botocore
import structlog
from django.core.exceptions import ValidationError
from django.db import models

from bench.language.blob import BLOB_HASH_LENGTH
from bench.models.utils import CrudNode
from bench.settings import GLOBAL_PROJECT_BUCKET_NAME

BLOB_PRESIGNED_POST_EXPIRY = 60 * 60  # 1 hour
BLOB_PRESIGNED_GET_EXPIRY = 60 * 60 * 24  # 1 day


def is_allowed_content_type(content_type: str) -> bool:
    # should we check anything here?
    return True


logger = structlog.get_logger(__name__)


# :BlobType
class BlobStatus(models.TextChoices):
    PREPARED = "prepared"
    UPLOADING = "uploading"
    AVAILABLE = "available"


class Blob(CrudNode):
    """
    A pointer to a remotely stored object.
    """

    project = models.ForeignKey("Project", on_delete=models.CASCADE, related_name="blobs")
    prepared_at = models.DateTimeField(null=True, blank=True)
    sha512 = models.CharField(max_length=BLOB_HASH_LENGTH)
    content_length = models.IntegerField()
    content_type = models.CharField(max_length=255)
    name = models.CharField(max_length=255, null=True, blank=True)
    status = models.CharField(
        max_length=32,
        choices=BlobStatus.choices,
        default=BlobStatus.PREPARED,
    )

    _presigned_post: Optional[str] = None  # set manually
    _presigned_get: Optional[str] = None  # set manually

    def __str__(self):
        return f"{self.id} {self.name} ({self.status}, {self.content_type}, {self.content_length} bytes)"

    def __repr__(self):
        return f"<Blob {self}>"

    @property
    def key(self) -> str:
        return f"{self.id}/{self.name}"

    @property
    def presigned_post(self) -> Optional[str]:
        return self._presigned_post  # must be set manually

    @property
    def presigned_get(self) -> Optional[str]:
        if self._presigned_get is not None:
            return self._presigned_get
        elif self.status == BlobStatus.AVAILABLE:
            self._presigned_get = self.generate_presigned_get()
            return self._presigned_get
        else:
            return None

    def mark_available_if_exists_in_s3(self):
        s3_client = get_s3_client()
        try:
            metadata = s3_client.head_object(
                Bucket=GLOBAL_PROJECT_BUCKET_NAME,
                Key=self.key,
            )
            if metadata["ContentLength"] != self.content_length:
                logger.warning("object_content_length_mismatch", blob=self, metadata=metadata)
            self.status = BlobStatus.AVAILABLE
        except botocore.exceptions.ClientError:
            logger.warning("object_not_found", blob=self)
            raise ValidationError(f"{self} not found in s3")

    def generate_presigned_post(self) -> str:
        if self.presigned_post is not None:
            return self.presigned_post
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_post(
            Bucket=GLOBAL_PROJECT_BUCKET_NAME,
            Key=self.key,
            ExpiresIn=BLOB_PRESIGNED_POST_EXPIRY,
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
        if self.status != BlobStatus.AVAILABLE:
            raise ValueError(f"cannot generate presigned get for {self} with status {self.status}")
        s3_client = get_s3_client()
        response = s3_client.generate_presigned_url(
            ClientMethod="get_object",
            Params={"Bucket": GLOBAL_PROJECT_BUCKET_NAME, "Key": self.key},
            ExpiresIn=BLOB_PRESIGNED_GET_EXPIRY,
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
