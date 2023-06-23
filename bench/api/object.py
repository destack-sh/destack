from typing import Optional

import botocore.exceptions
import structlog
from django.core.exceptions import ValidationError
from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.relay import GlobalID
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_write_project
from bench.api.utils import safe_mutation
from bench.models.object import REMOTE_OBJECT_MAX_SIZE, get_s3_client, is_allowed_content_type
from bench.settings import PROJECT_BUCKET_NAME

logger = structlog.get_logger(__name__)


RemoteObjectStatus = gql.enum(models.RemoteObjectStatus)


@gql.django.type(models.RemoteObject)
class RemoteObject(gql.Node):
    status: RemoteObjectStatus
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]
    presigned_post: Optional[str]
    presigned_get: Optional[str]


@gql.input
class RequestUploadObjectInput:
    project_id: GlobalID
    sha512: str
    content_length: int
    content_type: str
    name: Optional[str]


@gql.input
class NotifyUploadedObjectInput(gql.NodeInput):
    pass


@gql.input
class DeleteObjectInput(gql.NodeInput):
    pass


@gql.type
class ObjectMutation:
    @safe_mutation
    def request_upload_object(
        self, info: Info, input: RequestUploadObjectInput
    ) -> RemoteObject | OperationInfo:
        project = models.Project.objects.get(id=input.project_id.node_id)
        check_can_write_project(info, project)
        if input.content_length >= REMOTE_OBJECT_MAX_SIZE:
            raise ValidationError(
                f"object too large: {input.content_length} >= {REMOTE_OBJECT_MAX_SIZE}"
            )
        if not is_allowed_content_type(input.content_type):
            raise ValidationError(f"invalid content type: {input.content_type}")
        remote_object: RemoteObject = project.remote_objects.filter(sha512=input.sha512).first()
        if remote_object is not None:
            if remote_object.status == models.RemoteObjectStatus.AVAILABLE:
                return remote_object
            else:
                remote_object.status = models.RemoteObjectStatus.UPLOADING
        else:
            remote_object = models.RemoteObject(
                project=project,
                sha512=input.sha512,
                content_length=input.content_length,
                content_type=input.content_type,
                name=input.name,
                status=models.RemoteObjectStatus.UPLOADING,
            )
        remote_object.save()
        remote_object.generate_presigned_post()
        return remote_object

    @safe_mutation
    def notify_uploaded_object(
        self, info: Info, input: NotifyUploadedObjectInput
    ) -> RemoteObject | OperationInfo:
        remote_object = models.RemoteObject.objects.get(id=input.id.node_id)
        check_can_write_project(info, remote_object.project)
        # check that object exists in s3
        s3_client = get_s3_client()
        try:
            metadata = s3_client.head_object(
                Bucket=PROJECT_BUCKET_NAME,
                Key=str(remote_object.id),
            )
            if metadata["ContentLength"] != remote_object.content_length:
                logger.warning(
                    "object_content_length_mismatch", remote_object=remote_object, metadata=metadata
                )
        except botocore.exceptions.ClientError:
            logger.warning("object_not_found", remote_object=remote_object)
            raise ValidationError(f"object not found: {remote_object}")
        remote_object.status = models.RemoteObjectStatus.AVAILABLE
        remote_object.save()
        return remote_object

    @safe_mutation
    def delete_object(self, info: Info, input: DeleteObjectInput) -> RemoteObject | OperationInfo:
        object = models.RemoteObject.objects.get(input.id)
        check_can_write_project(info, object.project)
        object.delete()
        return object
